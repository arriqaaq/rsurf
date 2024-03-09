use std::collections::VecDeque;

use crate::bitmap::Bitmap;
use crate::dense::Builder;
use crate::iterator::{Iterator, SurfError};
use crate::key::{truncate, Key};
use crate::options::Options;

pub struct Surf {
    dense_labels: Bitmap,
    dense_has_child: Bitmap,
    dense_is_prefix_key: Bitmap,
}

impl Surf {
    pub fn new(raw_keys: Vec<Vec<u8>>, options: Options) -> Result<Surf, SurfError> {
        // Convert raw_keys to keys
        let mut keys: Vec<Key> = raw_keys;
        keys.sort();

        // Truncate keys
        keys = truncate(&keys);

        let mut dense_builder = Builder::new(options.memory_limit);
        dense_builder.build(&keys)?;

        let dense_labels = dense_builder.labels;
        let dense_has_child = dense_builder.has_child;
        let dense_is_prefix_key = dense_builder.is_prefix_key;

        Ok(Surf {
            dense_labels,
            dense_has_child,
            dense_is_prefix_key,
        })
    }

    pub fn lookup(
        &mut self,
        key: Vec<u8>,
    ) -> Result<(bool, Vec<u8>, Iterator, Option<SurfError>), SurfError> {
        let mut it = Iterator {
            labels: self.dense_labels.clone(),
            has_child: self.dense_has_child.clone(),
            is_prefix_key: self.dense_is_prefix_key.clone(),
            node_index: 0,
            next_edge: 0,
            edges: VecDeque::new(),
            nodes: VecDeque::new(),
            key_prefix: VecDeque::new(),
        };

        for i in 0..key.len() {
            let key_byte = key[i];

            match it.go_to_child(key_byte) {
                Ok(_) => {}
                Err(e) => {
                    if e == SurfError::NoSuchEdge {
                        // No edge with this value, so the key doesn't exist.
                        return Ok((false, vec![], it, None));
                    } else if e == SurfError::IsLeaf {
                        // We attempted to enter a leaf node, so the key exists
                        return Ok((true, key[..=i].to_vec(), it, None));
                    } else {
                        // Non-specific error, e.g. issue with bitmap access
                        return Ok((false, vec![], it, Some(e)));
                    }
                }
            }
        }

        // If we get until here, then we traversed the whole key. To determine
        // whether the key exists, we now must check if our current node has
        // is_prefix_key set to true.
        match self.dense_is_prefix_key.get(it.node_index) {
            Ok(is_prefix_key) => {
                if is_prefix_key == 1 {
                    Ok((true, key, it, None))
                } else {
                    Ok((false, vec![], it, None))
                }
            }
            Err(e) => Ok((
                false,
                vec![],
                it,
                Some(SurfError::CustomError(format!(
                    "Error accessing bit in D-IsPrefixKey: {}",
                    e
                ))),
            )),
        }
    }

    pub fn lookup_or_greater(
        &mut self,
        key: Vec<u8>,
    ) -> Result<(Vec<u8>, Iterator, Option<SurfError>), SurfError> {
        let (exists, matched_key, mut it, _err) = self.lookup(key)?;

        if exists {
            it.next_edge += 1;
            Ok((matched_key, it, None))
        } else {
            match it.next() {
                Ok(larger_key) => Ok((larger_key, it, None)),
                Err(e) => Ok((vec![], it, Some(e))),
            }
        }
    }

    pub fn range_lookup(&mut self, low: Vec<u8>, high: Vec<u8>) -> Result<bool, SurfError> {
        let (matched_key, _, _err) = self.lookup_or_greater(low)?;

        if matched_key <= high {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn count(&mut self, low: Vec<u8>, high: Vec<u8>) -> Result<usize, SurfError> {
        let (matched_key, mut it, _err) = self.lookup_or_greater(low)?;

        let mut count = 0;
        let high_key = high;
        let mut cur_key = matched_key;
        while cur_key <= high_key {
            count += 1;

            match it.next() {
                Ok(next_key) => cur_key = next_key,
                Err(_) => break,
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup() {
        let keys: Vec<Vec<u8>> = vec![
            vec![0x00, 0x01],       // Key in intermediary node
            vec![0x00, 0x01, 0x02], // Key in leaf node
            vec![0x42],
            vec![0xFF, 0x42, 0x70, 0x71],
        ];

        let mut surf = match Surf::new(keys.clone(), Options::new()) {
            Ok(surf) => surf,
            Err(e) => panic!("Error creating SuRF store: {:?}", e),
        };

        for k in &keys {
            match surf.lookup(k.clone()) {
                Ok(exists) => {
                    assert_eq!(exists.0, true);
                }
                Err(e) => panic!("Error looking up key: {:?}", e),
            }
        }

        let non_existent_keys: Vec<Vec<u8>> = vec![vec![0x00, 0x02], vec![0x43]];

        for k in non_existent_keys {
            match surf.lookup(k) {
                Ok(exists) => {
                    assert_eq!(exists.0, false);
                }
                Err(e) => panic!("Error looking up key: {:?}", e),
            }
        }
    }
}
