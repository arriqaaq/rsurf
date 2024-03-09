use std::collections::VecDeque;

use crate::bitmap::Bitmap;
use crate::dense::Builder;
use crate::iterator::{Error, Iterator};
use crate::key::{truncate, Key};
use crate::options::Options;

pub struct Surf {
    dense_labels: Bitmap,
    dense_has_child: Bitmap,
    dense_is_prefix_key: Bitmap,
}

impl Surf {
    pub fn new(raw_keys: Vec<Vec<u8>>, options: Options) -> Result<Surf, Error> {
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

    pub fn get(&mut self, key: Vec<u8>) -> Result<(bool, Vec<u8>, Iterator), Error> {
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
                    if e == Error::NoSuchEdge {
                        // No edge with this value, so the key doesn't exist.
                        return Ok((false, vec![], it));
                    } else if e == Error::IsLeaf {
                        // We attempted to enter a leaf node, so the key exists
                        return Ok((true, key[..=i].to_vec(), it));
                    } else {
                        // Non-specific error, e.g. issue with bitmap access
                        return Err(e);
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
                    Ok((true, key, it))
                } else {
                    Ok((false, vec![], it))
                }
            }
            Err(e) => Err(Error::CustomError(format!(
                "Error accessing bit in D-IsPrefixKey: {}",
                e
            ))),
        }
    }

    pub fn get_or_next(&mut self, key: Vec<u8>) -> Result<(Vec<u8>, Iterator), Error> {
        let (exists, matched_key, mut it) = self.get(key)?;

        if exists {
            it.next_edge += 1;
            Ok((matched_key, it))
        } else {
            match it.next_key() {
                Ok(larger_key) => Ok((larger_key, it)),
                Err(e) => Err(e),
            }
        }
    }

    pub fn range(&mut self, low: Vec<u8>, high: Vec<u8>) -> Result<bool, Error> {
        let (matched_key, _) = self.get_or_next(low)?;

        if matched_key <= high {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn count(&mut self, low: Vec<u8>, high: Vec<u8>) -> Result<usize, Error> {
        let (matched_key, mut it) = self.get_or_next(low)?;

        let mut count = 0;
        let high_key = high;
        let mut cur_key = matched_key;
        while cur_key <= high_key {
            count += 1;

            match it.next_key() {
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
    fn test_get() {
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
            match surf.get(k.clone()) {
                Ok(exists) => {
                    assert_eq!(exists.0, true);
                }
                Err(e) => panic!("Error looking up key: {:?}", e),
            }
        }

        let non_existent_keys: Vec<Vec<u8>> = vec![vec![0x00, 0x02], vec![0x43]];

        for k in non_existent_keys {
            match surf.get(k) {
                Ok(exists) => {
                    assert_eq!(exists.0, false);
                }
                Err(e) => panic!("Error looking up key: {:?}", e),
            }
        }
    }
}
