use crate::index::structs::handles::{LeafIdx, NodeIdx};
use crate::index::structs::storage::Storage;

impl Storage {
    pub fn descend_leftmost_leaf(&self, start: NodeIdx) -> LeafIdx {
        let mut current = start;
        loop {
            let node = &self.nodes[current.0 as usize];
            debug_assert!(node.num_children > 0);
            let first = node.child_slots[0].expect("at least one child");
            if node.is_leaf_parent() {
                return LeafIdx(first.idx);
            } else {
                current = NodeIdx(first.idx);
            }
        }
    }

    pub fn bubble_visible_len_delta(&mut self, leaf_idx: LeafIdx, delta: i64) {
        let parent = self.leaves[leaf_idx.0 as usize].parent;
        self.bubble_visible_len_delta_from_child(parent, leaf_idx.0, delta);
    }

    pub fn bubble_visible_len_delta_from_node(&mut self, node_idx: NodeIdx, delta: i64) {
        let parent = self.nodes[node_idx.0 as usize].parent;
        self.bubble_visible_len_delta_from_child(parent, node_idx.0, delta);
    }

    fn bubble_visible_len_delta_from_child(
        &mut self,
        mut parent_opt: Option<NodeIdx>,
        mut child_idx: u32,
        delta: i64,
    ) {
        while let Some(parent) = parent_opt {
            let node = &mut self.nodes[parent.0 as usize];
            let slot = node
                .child_slots
                .iter_mut()
                .take(node.num_children as usize)
                .flatten()
                .find(|s| s.idx == child_idx)
                .expect("parent slot pointing to child not found");
            slot.visible_len = (slot.visible_len as i64 + delta) as u64;
            parent_opt = node.parent;
            child_idx = parent.0;
        }
    }
}
