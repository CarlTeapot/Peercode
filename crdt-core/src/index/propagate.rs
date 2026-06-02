use crate::index::constants::NODE_CHILDREN;
use crate::index::structs::handles::{LeafIdx, NodeIdx};
use crate::index::structs::node::ChildSlot;
use crate::index::structs::root::Root;
use crate::index::structs::storage::Storage;

impl Storage {
    pub fn propagate_leaf_split(
        &mut self,
        left_idx: LeafIdx,
        left_visible: u64,
        right_idx: LeafIdx,
        right_visible: u64,
    ) {
        let Some(parent) = self.leaves[left_idx.0 as usize].parent else {
            let node_idx = self.make_root_node_for_two_leaves(
                left_idx,
                left_visible,
                right_idx,
                right_visible,
            );
            self.root = Root::Node(node_idx);
            return;
        };

        let parent_delta =
            self.overwrite_parent_slot_for_left(parent, left_idx.0, left_visible, right_visible);
        let new_slot = ChildSlot {
            idx: right_idx.0,
            visible_len: right_visible,
        };

        if self.parent_has_room(parent) {
            self.insert_child_into_node(parent, left_idx.0, new_slot);
            self.leaves[right_idx.0 as usize].parent = Some(parent);
            self.bubble_visible_len_delta_from_node(parent, parent_delta);
            return;
        }

        let (right_node_idx, left_node_visible, right_node_visible) =
            self.split_node(parent, left_idx.0, new_slot);
        let new_parent_for_right_leaf =
            self.parent_after_node_split(parent, right_node_idx, right_idx.0);
        self.leaves[right_idx.0 as usize].parent = Some(new_parent_for_right_leaf);

        self.propagate_node_split(
            parent,
            left_node_visible,
            right_node_idx,
            right_node_visible,
        );
    }

    pub fn propagate_node_split(
        &mut self,
        left_idx: NodeIdx,
        left_visible: u64,
        right_idx: NodeIdx,
        right_visible: u64,
    ) {
        let Some(parent) = self.nodes[left_idx.0 as usize].parent else {
            let node_idx =
                self.make_root_node_for_two_nodes(left_idx, left_visible, right_idx, right_visible);
            self.root = Root::Node(node_idx);
            return;
        };

        let parent_delta =
            self.overwrite_parent_slot_for_left(parent, left_idx.0, left_visible, right_visible);
        let new_slot = ChildSlot {
            idx: right_idx.0,
            visible_len: right_visible,
        };

        if self.parent_has_room(parent) {
            self.insert_child_into_node(parent, left_idx.0, new_slot);
            self.nodes[right_idx.0 as usize].parent = Some(parent);
            self.bubble_visible_len_delta_from_node(parent, parent_delta);
            return;
        }

        let (right_node_idx, left_node_visible, right_node_visible) =
            self.split_node(parent, left_idx.0, new_slot);
        let new_parent_for_right_node =
            self.parent_after_node_split(parent, right_node_idx, right_idx.0);
        self.nodes[right_idx.0 as usize].parent = Some(new_parent_for_right_node);

        self.propagate_node_split(
            parent,
            left_node_visible,
            right_node_idx,
            right_node_visible,
        );
    }

    fn overwrite_parent_slot_for_left(
        &mut self,
        parent: NodeIdx,
        left_idx_u32: u32,
        new_left_visible: u64,
        right_visible: u64,
    ) -> i64 {
        let node = &mut self.nodes[parent.0 as usize];
        let mut old = 0u64;
        for slot in node.child_slots.iter_mut() {
            if let Some(s) = slot.as_mut()
                && s.idx == left_idx_u32
            {
                old = s.visible_len;
                s.visible_len = new_left_visible;
                break;
            }
        }
        (new_left_visible as i64 + right_visible as i64) - old as i64
    }

    fn parent_has_room(&self, parent: NodeIdx) -> bool {
        (self.nodes[parent.0 as usize].num_children as usize) < NODE_CHILDREN
    }

    fn parent_after_node_split(
        &self,
        parent: NodeIdx,
        post_split_right: NodeIdx,
        right_child_idx_u32: u32,
    ) -> NodeIdx {
        let right_node = &self.nodes[post_split_right.0 as usize];
        let in_right = right_node
            .child_slots
            .iter()
            .take(right_node.num_children as usize)
            .flatten()
            .any(|s| s.idx == right_child_idx_u32);
        if in_right { post_split_right } else { parent }
    }
}
