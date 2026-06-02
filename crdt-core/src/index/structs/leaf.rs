use crate::index::constants::LEAF_CHILDREN;
use crate::index::structs::handles::{LeafIdx, NodeIdx};
use crate::types::BlockId;

#[derive(Debug, Clone, Copy)]
pub(in crate::index) struct LeafEntry {
    pub id: BlockId,
    pub len: u64,
    pub is_deleted: bool,
}

impl LeafEntry {
    pub fn visible_len(&self) -> u64 {
        if self.is_deleted { 0 } else { self.len }
    }
}

#[derive(Debug)]
pub(in crate::index) struct Leaf {
    pub entries: [Option<LeafEntry>; LEAF_CHILDREN],
    pub num_entries: u8,
    pub next_leaf: Option<LeafIdx>,
    pub parent: Option<NodeIdx>,
}

impl Leaf {
    pub fn new() -> Self {
        Leaf {
            entries: [const { None }; LEAF_CHILDREN],
            num_entries: 0,
            next_leaf: None,
            parent: None,
        }
    }

    pub fn visible_len(&self) -> u64 {
        self.entries
            .iter()
            .filter_map(|e| e.as_ref().map(LeafEntry::visible_len))
            .sum()
    }
}
