use crate::bitmap::Bitmap;


use std::collections::VecDeque;
use std::error::Error;


#[derive(Debug, PartialEq)]
pub enum SurfError {
    NoSuchEdge,
    IsLeaf,
    EndOfTrie,
    CustomError(String),
}

impl std::fmt::Display for SurfError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SurfError::NoSuchEdge => write!(f, "No such edge"),
            SurfError::IsLeaf => write!(f, "Is leaf"),
            SurfError::EndOfTrie => write!(f, "Reached end of trie"),
            SurfError::CustomError(s) => write!(f, "{}", s),
        }
    }
}

impl Error for SurfError {}

impl From<&'static str> for SurfError {
    fn from(err: &'static str) -> SurfError {
        SurfError::CustomError(err.to_string())
    }
}

// #[derive(Debug, Clone)]
// pub struct ErrNoSuchEdge;

// impl fmt::Display for ErrNoSuchEdge {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "Cannot move to non-existent edge")
//     }
// }

// impl Error for ErrNoSuchEdge {}

// #[derive(Debug, Clone)]
// pub struct ErrIsLeaf;

// impl fmt::Display for ErrIsLeaf {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "Cannot move to leaf node")
//     }
// }

// impl Error for ErrIsLeaf {}

// #[derive(Debug, Clone)]
// pub struct ErrEndOfTrie;

// impl fmt::Display for ErrEndOfTrie {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "Reached end of trie")
//     }
// }

// impl Error for ErrEndOfTrie {}

pub struct Iterator {
    pub labels: Bitmap,
    pub has_child: Bitmap,
    pub is_prefix_key: Bitmap,
    pub node_index: usize,
    pub nodes: VecDeque<usize>,
    pub next_edge: usize,
    pub edges: VecDeque<usize>,
    pub key_prefix: VecDeque<u8>,
}

impl Iterator {
    pub fn go_to_child(&mut self, edge: u8) -> Result<(), SurfError> {
        self.next_edge = edge as usize;

        let offset = 256 * self.node_index + edge as usize;

        let has_label = self.labels.get(offset)?;
        if has_label != 1 {
            return Err(SurfError::NoSuchEdge);
        }

        let has_child = self.has_child.get(offset)?;
        if has_child != 1 {
            return Err(SurfError::IsLeaf);
        }

        let next_node = self.has_child.rank(1, offset)?;

        self.key_prefix.push_back(self.next_edge as u8);
        self.nodes.push_back(self.node_index);
        self.edges.push_back(edge as usize);

        self.node_index = next_node;
        self.next_edge = 0;

        Ok(())
    }

    pub fn next(&mut self) -> Result<Vec<u8>, SurfError> {
        loop {
            for _ in self.next_edge..256 {
                match self.go_to_child(self.next_edge as u8) {
                    Ok(_) => {
                        let is_prefix_key = self.is_prefix_key.get(self.node_index)?;

                        if is_prefix_key == 1 {
                            let key = self.key_prefix.iter().cloned().collect();

                            return Ok(key);
                        }
                    }
                    Err(e) => {
                        if e.to_string() == "Cannot move to non-existent edge" {
                            continue;
                        } else if e.to_string() == "Cannot move to leaf node" {
                            let mut key: Vec<u8> = self.key_prefix.iter().cloned().collect();
                            key.push(self.next_edge as u8);
                            self.next_edge += 1;
                            return Ok(key);
                        } else {
                            return Err(e);
                        }
                    }
                }
            }

            if self.node_index == 0 {
                return Err(SurfError::EndOfTrie);
            } else {
                self.node_index = self.nodes.pop_back().unwrap();
                self.next_edge = self.edges.pop_back().unwrap() + 1;
                self.key_prefix.pop_back();
            }
        }
    }
}
