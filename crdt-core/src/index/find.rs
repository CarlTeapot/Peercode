use crate::index::structs::find_result::FindResult;
use crate::index::structs::handles::{LeafIdx, NodeIdx};
use crate::index::structs::position_index::PositionIndex;
use crate::index::structs::root::Root;
use crate::types::BlockId;

impl PositionIndex {
    pub fn visible_len(&self) -> u64 {
        match self.storage.root {
            Root::Empty => 0,
            Root::Leaf(idx) => self.storage.leaves[idx.0 as usize].visible_len(),
            Root::Node(idx) => self.storage.nodes[idx.0 as usize].visible_len(),
        }
    }

    pub fn position_of(&self, id: BlockId) -> Option<u64> {
        let (leaf_idx, slot): (LeafIdx, u8) = *self.storage.id_to_leaf.get(&id)?;
        let pos_within = self.sum_visible_before_slot(leaf_idx, slot);
        Some(pos_within + self.sum_visible_left_of_subtree(leaf_idx))
    }

    pub fn find_at_position(&self, mut pos: u64) -> FindResult {
        let total = self.visible_len();
        if pos >= total {
            return FindResult {
                id: None,
                offset: pos - total,
                tail_id: self.rightmost_entry_id(),
            };
        }
        let leaf_idx = self.descend_to_leaf_at_pos(&mut pos);
        self.scan_leaf_for_pos(leaf_idx, pos)
    }

    fn sum_visible_before_slot(&self, leaf_idx: LeafIdx, slot: u8) -> u64 {
        let leaf = &self.storage.leaves[leaf_idx.0 as usize];
        let mut pos = 0u64;
        for s in 0..slot as usize {
            if let Some(e) = &leaf.entries[s] {
                pos += e.visible_len();
            }
        }
        pos
    }

    fn sum_visible_left_of_subtree(&self, leaf_idx: LeafIdx) -> u64 {
        let mut pos = 0u64;
        let mut child_idx_u32 = leaf_idx.0;
        let mut parent_opt = self.storage.leaves[leaf_idx.0 as usize].parent;
        while let Some(parent) = parent_opt {
            let node = &self.storage.nodes[parent.0 as usize];
            for slot in node.child_slots.iter() {
                match slot {
                    Some(s) if s.idx == child_idx_u32 => break,
                    Some(s) => pos += s.visible_len,
                    None => break,
                }
            }
            child_idx_u32 = parent.0;
            parent_opt = node.parent;
        }
        pos
    }

    fn descend_to_leaf_at_pos(&self, pos: &mut u64) -> LeafIdx {
        let mut current = match self.storage.root {
            Root::Empty => unreachable!("caller guarantees pos < total"),
            Root::Leaf(l) => return l,
            Root::Node(n) => n,
        };
        loop {
            let node = &self.storage.nodes[current.0 as usize];
            let mut chosen: u32 = u32::MAX;
            for slot in node.child_slots.iter().take(node.num_children as usize) {
                let s = slot.expect("populated slot");
                if *pos < s.visible_len {
                    chosen = s.idx;
                    break;
                }
                *pos -= s.visible_len;
            }
            debug_assert_ne!(
                chosen,
                u32::MAX,
                "pos < total but no child matched (augmentation drift?)"
            );
            if node.is_leaf_parent() {
                return LeafIdx(chosen);
            }
            current = NodeIdx(chosen);
        }
    }

    fn scan_leaf_for_pos(&self, leaf_idx: LeafIdx, mut pos: u64) -> FindResult {
        let leaf = &self.storage.leaves[leaf_idx.0 as usize];
        for slot in 0..leaf.num_entries as usize {
            let e = leaf.entries[slot].expect("populated entry");
            if e.is_deleted {
                continue;
            }
            if pos < e.len {
                return FindResult {
                    id: Some(e.id),
                    offset: pos,
                    tail_id: None,
                };
            }
            pos -= e.len;
        }
        unreachable!("pos < total but the chosen leaf didn't contain it");
    }

    fn rightmost_entry_id(&self) -> Option<BlockId> {
        let leaf_idx = self.descend_rightmost_leaf()?;
        let leaf = &self.storage.leaves[leaf_idx.0 as usize];
        (0..leaf.num_entries as usize)
            .rev()
            .find_map(|slot| leaf.entries[slot].map(|e| e.id))
    }

    fn descend_rightmost_leaf(&self) -> Option<LeafIdx> {
        let mut current = match self.storage.root {
            Root::Empty => return None,
            Root::Leaf(l) => return Some(l),
            Root::Node(n) => n,
        };
        loop {
            let node = &self.storage.nodes[current.0 as usize];
            let last = (node.num_children - 1) as usize;
            let last_idx = node.child_slots[last].expect("populated").idx;
            if node.is_leaf_parent() {
                return Some(LeafIdx(last_idx));
            }
            current = NodeIdx(last_idx);
        }
    }
}
