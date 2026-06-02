use crate::index::constants::{LEAF_CHILDREN, NODE_CHILDREN};
use crate::index::structs::handles::{LeafIdx, NodeIdx};
use crate::index::structs::leaf::{Leaf, LeafEntry};
use crate::index::structs::node::{ChildSlot, ChildType, Node};
use crate::index::structs::storage::Storage;

impl Storage {
    /// Split a full leaf into two, with the new entry inserted at the
    /// appropriate slot. Returns the new right-hand leaf's index and the
    /// left/right visible-len values. The original leaf becomes the left half.
    pub fn split_leaf(
        &mut self,
        leaf_idx: LeafIdx,
        after_slot: Option<u8>,
        entry: LeafEntry,
    ) -> (LeafIdx, u64, u64) {
        let mut all = self.build_leaf_overflow_buffer(leaf_idx, after_slot, entry);
        let mid = all.len() / 2;
        let right_entries: Vec<LeafEntry> = all.split_off(mid);
        let left_entries: Vec<LeafEntry> = all;

        let right_idx =
            self.push_new_leaf(&right_entries, self.leaves[leaf_idx.0 as usize].next_leaf);
        self.overwrite_leaf_entries(leaf_idx, &left_entries, Some(right_idx));

        self.reindex_leaf_entries(leaf_idx, &left_entries);
        self.reindex_leaf_entries(right_idx, &right_entries);

        let left_visible = sum_leaf_visible(&left_entries);
        let right_visible = sum_leaf_visible(&right_entries);
        (right_idx, left_visible, right_visible)
    }

    pub fn split_node(
        &mut self,
        node_idx: NodeIdx,
        after_child_idx_u32: u32,
        new_slot: ChildSlot,
    ) -> (NodeIdx, u64, u64) {
        let child_type = self.nodes[node_idx.0 as usize].child_type;

        let mut all = self.build_node_overflow_buffer(node_idx, after_child_idx_u32, new_slot);
        let mid = all.len() / 2;
        let right_slots: Vec<ChildSlot> = all.split_off(mid);
        let left_slots: Vec<ChildSlot> = all;

        let right_idx = self.push_new_node(&right_slots, child_type);
        self.overwrite_node_slots(node_idx, &left_slots);
        self.reparent_children_under(right_idx, &right_slots, child_type);

        let left_visible = sum_slot_visible(&left_slots);
        let right_visible = sum_slot_visible(&right_slots);
        (right_idx, left_visible, right_visible)
    }

    fn build_leaf_overflow_buffer(
        &self,
        leaf_idx: LeafIdx,
        after_slot: Option<u8>,
        entry: LeafEntry,
    ) -> Vec<LeafEntry> {
        let leaf = &self.leaves[leaf_idx.0 as usize];
        let insert_pos = match after_slot {
            None => 0,
            Some(s) => s as usize + 1,
        };
        let mut all = Vec::with_capacity(LEAF_CHILDREN + 1);
        for slot in 0..leaf.num_entries as usize {
            if slot == insert_pos {
                all.push(entry);
            }
            if let Some(e) = &leaf.entries[slot] {
                all.push(*e);
            }
        }
        if insert_pos >= leaf.num_entries as usize {
            all.push(entry);
        }
        debug_assert_eq!(all.len(), LEAF_CHILDREN + 1);
        all
    }

    /// Materialise `node_idx`'s child slots plus `new_slot` into a single
    /// vector, with `new_slot` placed immediately after the slot pointing
    /// at `after_child_idx_u32`
    fn build_node_overflow_buffer(
        &self,
        node_idx: NodeIdx,
        after_child_idx_u32: u32,
        new_slot: ChildSlot,
    ) -> Vec<ChildSlot> {
        let node = &self.nodes[node_idx.0 as usize];
        let mut insert_at = node.num_children as usize;
        for (i, slot) in node
            .child_slots
            .iter()
            .enumerate()
            .take(node.num_children as usize)
        {
            if let Some(s) = slot
                && s.idx == after_child_idx_u32
            {
                insert_at = i + 1;
            }
        }
        let mut all = Vec::with_capacity(NODE_CHILDREN + 1);
        for (i, slot) in node
            .child_slots
            .iter()
            .enumerate()
            .take(node.num_children as usize)
        {
            if i == insert_at {
                all.push(new_slot);
            }
            if let Some(s) = slot {
                all.push(*s);
            }
        }
        if insert_at >= node.num_children as usize {
            all.push(new_slot);
        }
        debug_assert_eq!(all.len(), NODE_CHILDREN + 1);
        all
    }

    fn push_new_leaf(&mut self, entries: &[LeafEntry], next_leaf: Option<LeafIdx>) -> LeafIdx {
        let mut leaf = Leaf::new();
        for (i, e) in entries.iter().enumerate() {
            leaf.entries[i] = Some(*e);
        }
        leaf.num_entries = entries.len() as u8;
        leaf.next_leaf = next_leaf;
        let idx = LeafIdx(self.leaves.len() as u32);
        self.leaves.push(leaf);
        idx
    }

    fn overwrite_leaf_entries(
        &mut self,
        leaf_idx: LeafIdx,
        entries: &[LeafEntry],
        next_leaf: Option<LeafIdx>,
    ) {
        let leaf = &mut self.leaves[leaf_idx.0 as usize];
        leaf.entries = [const { None }; LEAF_CHILDREN];
        for (i, e) in entries.iter().enumerate() {
            leaf.entries[i] = Some(*e);
        }
        leaf.num_entries = entries.len() as u8;
        leaf.next_leaf = next_leaf;
    }

    /// Update `id_to_leaf` so every `(id → (leaf_idx, slot))` mapping in the
    /// given range is correct. Must be called after a leaf's entries are
    /// rewritten, even if the leaf's index didn't change.
    fn reindex_leaf_entries(&mut self, leaf_idx: LeafIdx, entries: &[LeafEntry]) {
        for (i, e) in entries.iter().enumerate() {
            self.id_to_leaf.insert(e.id, (leaf_idx, i as u8));
        }
    }

    /// Build a fresh internal node out of `slots` and push it onto the pool.
    fn push_new_node(&mut self, slots: &[ChildSlot], child_type: ChildType) -> NodeIdx {
        let mut node = Node::new(child_type);
        for (i, s) in slots.iter().enumerate() {
            node.child_slots[i] = Some(*s);
        }
        node.num_children = slots.len() as u8;
        let idx = NodeIdx(self.nodes.len() as u32);
        self.nodes.push(node);
        idx
    }

    /// Overwrite `node_idx`'s child slots in place with `slots`. The node's
    /// `child_type` is preserved.
    fn overwrite_node_slots(&mut self, node_idx: NodeIdx, slots: &[ChildSlot]) {
        let node = &mut self.nodes[node_idx.0 as usize];
        node.child_slots = [None; NODE_CHILDREN];
        for (i, s) in slots.iter().enumerate() {
            node.child_slots[i] = Some(*s);
        }
        node.num_children = slots.len() as u8;
    }

    /// Set `parent` on each child referenced by `slots`. Whether the children
    /// are leaves or nodes is dictated by the parent's `child_type`.
    fn reparent_children_under(
        &mut self,
        parent: NodeIdx,
        slots: &[ChildSlot],
        child_type: ChildType,
    ) {
        if child_type == ChildType::Leaf {
            for s in slots {
                self.leaves[s.idx as usize].parent = Some(parent);
            }
        } else {
            for s in slots {
                self.nodes[s.idx as usize].parent = Some(parent);
            }
        }
    }
}

fn sum_leaf_visible(entries: &[LeafEntry]) -> u64 {
    entries.iter().map(LeafEntry::visible_len).sum()
}

fn sum_slot_visible(slots: &[ChildSlot]) -> u64 {
    slots.iter().map(|s| s.visible_len).sum()
}
