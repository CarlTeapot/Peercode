use crate::index::constants::NODE_CHILDREN;
use crate::index::structs::handles::NodeIdx;

#[derive(Debug, Clone, Copy)]
pub(in crate::index) struct ChildSlot {
    pub idx: u32,
    pub visible_len: u64,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::index) enum ChildType {
    Leaf,
    Node,
}

#[derive(Debug)]
pub(in crate::index) struct Node {
    pub child_slots: [Option<ChildSlot>; NODE_CHILDREN],
    pub num_children: u8,
    pub parent: Option<NodeIdx>,
    pub child_type: ChildType,
}

impl Node {
    pub fn new(child_type: ChildType) -> Self {
        Node {
            child_slots: [None; NODE_CHILDREN],
            num_children: 0,
            parent: None,
            child_type,
        }
    }

    pub fn is_leaf_parent(&self) -> bool {
        matches!(self.child_type, ChildType::Leaf)
    }

    pub fn visible_len(&self) -> u64 {
        self.child_slots
            .iter()
            .filter_map(|s| s.map(|s| s.visible_len))
            .sum()
    }
}
