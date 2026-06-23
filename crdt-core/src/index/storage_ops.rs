use crate::index::constants::{LEAF_CHILDREN, NODE_CHILDREN};
use crate::index::structs::handles::{LeafIdx, NodeIdx};
use crate::index::structs::leaf::{Leaf, LeafEntry};
use crate::index::structs::node::ChildSlot;
use crate::index::structs::root::Root;
use crate::index::structs::storage::Storage;
use crate::types::BlockId;

impl Storage {
    pub fn push_first_entry(&mut self, entry: LeafEntry) {
        debug_assert!(matches!(self.root, Root::Empty));
        let mut leaf = Leaf::new();
        leaf.entries[0] = Some(entry);
        leaf.num_entries = 1;
        let leaf_idx = LeafIdx(self.leaves.len() as u32);
        self.leaves.push(leaf);
        self.root = Root::Leaf(leaf_idx);
        self.id_to_leaf.insert(entry.id, (leaf_idx, 0));
    }

    pub fn insert_into_leaf(
        &mut self,
        leaf_idx: LeafIdx,
        after_slot: Option<u8>,
        entry: LeafEntry,
    ) {
        let leaf = &mut self.leaves[leaf_idx.0 as usize];
        debug_assert!((leaf.num_entries as usize) < LEAF_CHILDREN);
        debug_assert!(
            after_slot.is_none_or(|s| (s as usize) < leaf.num_entries as usize),
            "after_slot must point at an existing entry"
        );
        let insert_pos = match after_slot {
            None => 0,
            Some(s) => s as usize + 1,
        };
        for i in (insert_pos..leaf.num_entries as usize).rev() {
            leaf.entries[i + 1] = leaf.entries[i].take();
        }
        leaf.entries[insert_pos] = Some(entry);
        leaf.num_entries += 1;

        for slot in insert_pos..leaf.num_entries as usize {
            if let Some(e) = &leaf.entries[slot] {
                self.id_to_leaf.insert(e.id, (leaf_idx, slot as u8));
            }
        }
    }

    pub fn locate(&self, id: BlockId) -> (LeafIdx, u8) {
        *self.id_to_leaf.get(&id).expect("block id not in index")
    }

    pub fn insert_child_into_node(
        &mut self,
        node_idx: NodeIdx,
        after_child_idx_u32: u32,
        new_slot: ChildSlot,
    ) {
        let node = &mut self.nodes[node_idx.0 as usize];
        debug_assert!((node.num_children as usize) < NODE_CHILDREN);
        let mut insert_at = 0usize;
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
                break;
            }
        }
        for i in (insert_at..node.num_children as usize).rev() {
            node.child_slots[i + 1] = node.child_slots[i].take();
        }
        node.child_slots[insert_at] = Some(new_slot);
        node.num_children += 1;
    }
}
