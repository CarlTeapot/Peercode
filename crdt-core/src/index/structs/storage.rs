use std::collections::HashMap;

use crate::index::structs::handles::LeafIdx;
use crate::index::structs::leaf::Leaf;
use crate::index::structs::node::Node;
use crate::index::structs::root::Root;
use crate::types::BlockId;

#[derive(Debug)]
pub(in crate::index) struct Storage {
    pub leaves: Vec<Leaf>,
    pub nodes: Vec<Node>,
    pub root: Root,
    pub id_to_leaf: HashMap<BlockId, (LeafIdx, u8)>,
}

impl Storage {
    pub fn new() -> Self {
        Storage {
            leaves: Vec::new(),
            nodes: Vec::new(),
            root: Root::Empty,
            id_to_leaf: HashMap::new(),
        }
    }
}
