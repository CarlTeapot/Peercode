use crate::index::structs::handles::{LeafIdx, NodeIdx};
use crate::index::structs::node::{ChildSlot, ChildType, Node};
use crate::index::structs::storage::Storage;

impl Storage {
    pub fn make_root_node_for_two_leaves(
        &mut self,
        left: LeafIdx,
        left_visible: u64,
        right: LeafIdx,
        right_visible: u64,
    ) -> NodeIdx {
        let node_idx = self.push_node_with_two_children(
            ChildType::Leaf,
            left.0,
            left_visible,
            right.0,
            right_visible,
        );
        self.leaves[left.0 as usize].parent = Some(node_idx);
        self.leaves[right.0 as usize].parent = Some(node_idx);
        node_idx
    }

    pub fn make_root_node_for_two_nodes(
        &mut self,
        left: NodeIdx,
        left_visible: u64,
        right: NodeIdx,
        right_visible: u64,
    ) -> NodeIdx {
        let node_idx = self.push_node_with_two_children(
            ChildType::Node,
            left.0,
            left_visible,
            right.0,
            right_visible,
        );
        self.nodes[left.0 as usize].parent = Some(node_idx);
        self.nodes[right.0 as usize].parent = Some(node_idx);
        node_idx
    }

    fn push_node_with_two_children(
        &mut self,
        child_type: ChildType,
        left_idx: u32,
        left_visible: u64,
        right_idx: u32,
        right_visible: u64,
    ) -> NodeIdx {
        let mut node = Node::new(child_type);
        node.child_slots[0] = Some(ChildSlot {
            idx: left_idx,
            visible_len: left_visible,
        });
        node.child_slots[1] = Some(ChildSlot {
            idx: right_idx,
            visible_len: right_visible,
        });
        node.num_children = 2;
        let node_idx = NodeIdx(self.nodes.len() as u32);
        self.nodes.push(node);
        node_idx
    }
}
