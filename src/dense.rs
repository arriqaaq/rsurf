use crate::bitmap::Bitmap;
use crate::key::Key;

// NodeTask contains things which need to be considered for building up a future node.
//
// This includes keys whose path contains that node, but also additional
// information such as whether the node might be a prefix key.
#[derive(Clone)]
struct NodeTask {
    // keys is the slice of keys whose path will pass through the given node.
    keys: Vec<Key>,
    // is_prefix_key defines whether this node's is_prefix_key flag will have
    // to be set to true - if the node will exist at all.
    is_prefix_key: bool,
}

// Builder provides methods to build up a LOUDS-DENSE encoded FST tree from a
// set of keys.
pub(crate) struct Builder {
    // Labels is the D-Labels bitmap of the DENSE-encoded FST.
    //
    // Nodes are encoded as blocks of 256 bits in level-order. If the node
    // has an outbound edge with value `b`, then the b-th bit of the node's
    // 256 bit block is set.
    pub(crate) labels: Bitmap,

    // HasChild is the D-HasChild bitmap of the DENSE-encoded FST.
    //
    // If a node has an outbound edge with value `b` leading to a subtree
    // of the tree (rather than a terminal value), then the b-th bit of the
    // node's block is set.
    pub(crate) has_child: Bitmap,

    // IsPrefixKey is the D-IsPrefixKey bitmap of the DENSE-encoded FST.
    //
    // If the n-th node is also the terminal node of a stored key, the n-th
    // bit of this bitmap will be set.
    pub(crate) is_prefix_key: Bitmap,

    // tasks is a slice of tasks to be taken care of to define nodes
    // further down the tree.
    // There is a 1:1 correspondence between tasks and (potential) future
    // nodes.
    tasks: Vec<NodeTask>,
    // current_task is a pointer to the most recent element of the tasks
    // slice.
    // As such it is not the task currently being worked on, but the task
    // currently being defined. A bit of a misnomer, I admit.
    current_task_id: usize,

    // current_node_id is the 0-indexed level-order ID of the node we are
    // currently building up.
    current_node_id: usize,
}

impl Builder {
    // NewBuilder instantiates a new LOUDS-DENSE builder.
    //
    // memory_limit specifies the memory limits in bits.
    pub(crate) fn new(memory_limit: usize) -> Self {
        // Labels and HasChild are 256 bits per node, IsPrefixKey is 1 bit per node.
        let memory_unit = memory_limit / (256 + 256 + 1);

        Builder {
            labels: Bitmap::new(256, 256 * memory_unit),
            has_child: Bitmap::new(256, 256 * memory_unit),
            is_prefix_key: Bitmap::new(1, memory_unit),
            tasks: Vec::new(),
            current_task_id: 0,
            current_node_id: 0,
        }
    }

    // Build instantiates a LOUDS-DENSE encoded tree using the given keys.
    //
    // Build may only be called on a freshly created instance. Calling Build on a
    // builder more than once is not guaranteed to produce a consistent tree.
    pub(crate) fn build(&mut self, keys: &[Key]) -> Result<(), &'static str> {
        // For depth = 0 we'll consider all keys
        self.append_node_task();
        {
            let tasks = &mut self.tasks;
            let current_task = tasks.get_mut(self.current_task_id).unwrap();
            current_task.keys = keys.to_vec();
        }

        for depth in 0..max_key_length(keys) {
            // During iteration we'll be adding tasks of the next tree
            // level. But we only want to consider tasks of the current
            // level.
            let n = self.tasks.len();
            for i in 0..n {
                let task = {
                    let tasks = &mut self.tasks;
                    tasks[i].clone()
                };

                if task.keys.is_empty() {
                    // Empty tasks are the result of there only being a
                    // single key pointing to this node, which has reached
                    // the end.
                    continue;
                }

                // Each task corresponds to one node, so us starting
                // with a new task means we're populating a new node
                let mut node_has_edges = false;
                let mut most_recent_edge: u8 = 0x00;

                // We'll make sure the current node's extents in the various bitmaps are
                // allocated.
                // This is not strictly needed, but makes for cleaner / easier to test
                // results.
                self.labels.get(self.label_offset() + 255)?;
                self.has_child.get(self.has_child_offset() + 255)?;
                let bit = self.is_prefix_key_offset();
                self.is_prefix_key.get(bit)?;

                // If the node is non-empty (which is the case if we are here), and the task has
                // its is_prefix_key flag set, then that means that one key ended on this node.
                if task.is_prefix_key {
                    self.is_prefix_key.set(bit)?;
                }

                for key in &task.keys {
                    let edge = key[depth];

                    if !node_has_edges || most_recent_edge != edge {
                        let bit = self.label_offset() + usize::from(edge);
                        self.labels.set(bit)?;

                        let task = NodeTask {
                            keys: Vec::new(),
                            is_prefix_key: false,
                        };

                        self.tasks.push(task);
                        self.current_task_id = self.tasks.len() - 1;

                        most_recent_edge = edge;
                        node_has_edges = true;
                    }

                    if depth == key.len() - 1 {
                        let tasks = &mut self.tasks;
                        let current_task = tasks.get_mut(self.current_task_id).unwrap();
                        current_task.is_prefix_key = true;
                    } else {
                        let bit = self.has_child_offset() + usize::from(edge);
                        self.has_child.set(bit)?;

                        let tasks = &mut self.tasks;
                        let current_task = tasks.get_mut(self.current_task_id).unwrap();
                        current_task.keys.push(key.to_vec());
                    }
                }

                // Reached end of the current node.
                self.current_node_id += 1;
            }

            // We processed all tasks of the current level, so we'll
            // discard them.
            self.tasks.drain(..n);
        }
        Ok(())
    }

    // label_offset returns the offset in the D-Labels bitmap of the currently processed node.
    fn label_offset(&self) -> usize {
        self.current_node_id * 256
    }

    // has_child_offset returns the offset in the D-HasChild bitmap of the currently processed node.
    fn has_child_offset(&self) -> usize {
        self.current_node_id * 256
    }

    // is_prefix_key_offset returns the offset in the D-IsPrefixKey bitmap of the currently processed node.
    fn is_prefix_key_offset(&self) -> usize {
        self.current_node_id
    }

    // append_node_task adds a new empty NodeTask to the list of future tasks to
    // perform, and updates the pointer to the most recently added NodeTask.
    fn append_node_task(&mut self) {
        let task = NodeTask {
            keys: Vec::new(),
            is_prefix_key: false,
        };

        self.tasks.push(task);
        self.current_task_id = self.tasks.len() - 1;
    }
}

// max_key_length returns the maximum length in bytes of the given LOUDS keys.
fn max_key_length(keys: &[Key]) -> usize {
    keys.iter().map(|k| k.len()).max().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_keys() -> Vec<Key> {
        vec![
            b"f".to_vec(),    // 0
            b"far".to_vec(),  // 1
            b"fas".to_vec(),  // 2
            b"fast".to_vec(), // 3
            b"fat".to_vec(),  // 4
            b"s".to_vec(),    // 5
            b"top".to_vec(),  // 6
            b"toy".to_vec(),  // 7
            b"trie".to_vec(), // 8
            b"trip".to_vec(), // 9
            b"try".to_vec(),  // 10
        ]
    }

    const MEM_LIMIt: usize = 80_000_000;

    #[test]
    fn test_build_one_level() {
        let mut b = Builder::new(MEM_LIMIt);
        let keys: Vec<Key> = vec![
            vec![0x00], // 0
            vec![0x17], // 1
            vec![0x42], // 2
            vec![0x60], // 3
            vec![0xF9], // 4
        ];
        let mut e_labels = Bitmap::new(256, 256);
        let e_has_child = Bitmap::new(256, 256);
        let e_is_prefix_key = Bitmap::new(1, 256);

        for k in &keys {
            e_labels.set(k[0] as usize).unwrap();
        }

        b.build(&keys).unwrap();

        assert_eq!(e_labels.data, b.labels.data);
        assert_eq!(e_has_child.data, b.has_child.data);
        assert_eq!(e_is_prefix_key.data, b.is_prefix_key.data);
    }

    #[test]
    fn test_build_two_levels() {
        let mut builder = Builder::new(MEM_LIMIt);
        let keys: Vec<Key> = vec![
            b"ai".to_vec(),
            b"ao".to_vec(),
            b"f".to_vec(),
            b"fa".to_vec(),
            b"fe".to_vec(),
        ];

        let mut expected_labels = Bitmap::new(768, 768);
        let mut expected_has_child = Bitmap::new(768, 768);
        let mut expected_is_prefix_key = Bitmap::new(3, 256);

        let labels: Vec<usize> = vec![
            // First node: Edges a, f
            0 * 256 + (b'a' as usize),
            0 * 256 + (b'f' as usize),
            // Second node: Edges i, o
            1 * 256 + (b'i' as usize),
            1 * 256 + (b'o' as usize),
            // Third node: Edges a, e
            2 * 256 + (b'a' as usize),
            2 * 256 + (b'e' as usize),
        ];
        for bit in labels {
            expected_labels.set(bit).unwrap();
        }

        let children: Vec<usize> = vec![
            // First node: a, f have sub-tree
            0 * 256 + (b'a' as usize),
            0 * 256 + (b'f' as usize),
        ];

        for bit in children {
            expected_has_child.set(bit as usize).unwrap();
        }

        let prefix_keys = vec![
            // Third node (-> 'f'): Is prefix key
            2,
        ];
        for bit in prefix_keys {
            expected_is_prefix_key.set(bit).unwrap();
        }

        // Let's test it :)
        builder.build(&keys).unwrap();

        assert_eq!(
            expected_labels.data, builder.labels.data,
            "Expected Labels:\n{:?}\nGot:\n{:?}",
            expected_labels, builder.labels
        );
        assert_eq!(
            expected_has_child.data, builder.has_child.data,
            "Expected HasChild:\n{:?}\nGot:\n{:?}",
            expected_has_child, builder.has_child
        );
        assert_eq!(
            expected_is_prefix_key.data, builder.is_prefix_key.data,
            "Expected IsPrefixKey:\n{:?}\nGot:\n{:?}",
            expected_is_prefix_key, builder.is_prefix_key
        );
    }
}
