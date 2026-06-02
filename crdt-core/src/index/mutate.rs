use crate::index::constants::LEAF_CHILDREN;
use crate::index::structs::handles::LeafIdx;
use crate::index::structs::leaf::LeafEntry;
use crate::index::structs::position_index::PositionIndex;
use crate::index::structs::root::Root;
use crate::index::structs::storage::Storage;
use crate::types::BlockId;

enum InsertTarget {
    DoneEmpty,
    At {
        leaf_idx: LeafIdx,
        after_slot: Option<u8>,
    },
}

impl PositionIndex {
    pub fn insert_after(&mut self, prev: Option<BlockId>, id: BlockId, len: u64) {
        debug_assert!(
            !self.storage.id_to_leaf.contains_key(&id),
            "insert_after called with id {id:?} already present in index"
        );
        let entry = LeafEntry {
            id,
            len,
            is_deleted: false,
        };

        let (leaf_idx, after_slot) = match self.resolve_insert_target(prev, entry) {
            InsertTarget::DoneEmpty => return,
            InsertTarget::At {
                leaf_idx,
                after_slot,
            } => (leaf_idx, after_slot),
        };

        let leaf = &self.storage.leaves[leaf_idx.0 as usize];
        if (leaf.num_entries as usize) < LEAF_CHILDREN {
            self.storage.insert_into_leaf(leaf_idx, after_slot, entry);
            self.storage.bubble_visible_len_delta(leaf_idx, len as i64);
            return;
        }

        let (right_idx, left_visible, right_visible) =
            self.storage.split_leaf(leaf_idx, after_slot, entry);
        self.attach_after_leaf_split(leaf_idx, left_visible, right_idx, right_visible);
    }

    pub fn split_entry(&mut self, id: BlockId, offset: u64, new_id: BlockId) {
        debug_assert!(
            !self.storage.id_to_leaf.contains_key(&new_id),
            "split_entry called with new_id {new_id:?} already present in index"
        );
        let (leaf_idx, slot) = self.storage.locate(id);
        let new_entry = self.shrink_left_half_for_split(leaf_idx, slot, offset, new_id);

        let leaf_has_room =
            (self.storage.leaves[leaf_idx.0 as usize].num_entries as usize) < LEAF_CHILDREN;
        if leaf_has_room {
            self.storage
                .insert_into_leaf(leaf_idx, Some(slot), new_entry);
            return;
        }

        let (right_idx, left_visible, right_visible) =
            self.storage.split_leaf(leaf_idx, Some(slot), new_entry);
        self.storage
            .propagate_leaf_split(leaf_idx, left_visible, right_idx, right_visible);
    }

    pub fn set_deleted(&mut self, id: BlockId) {
        let (leaf_idx, slot) = self.storage.locate(id);
        let leaf = &mut self.storage.leaves[leaf_idx.0 as usize];
        let entry = leaf.entries[slot as usize]
            .as_mut()
            .expect("located entry must exist");
        if entry.is_deleted {
            return;
        }
        let delta = -(entry.len as i64);
        entry.is_deleted = true;
        self.storage.bubble_visible_len_delta(leaf_idx, delta);
    }

    pub fn rebuild_from_order(&mut self, entries: impl Iterator<Item = (BlockId, u64, bool)>) {
        self.storage = Storage::new();
        let mut prev: Option<BlockId> = None;
        for (id, len, is_deleted) in entries {
            self.insert_after(prev, id, len);
            if is_deleted {
                self.set_deleted(id);
            }
            prev = Some(id);
        }
    }

    fn resolve_insert_target(&mut self, prev: Option<BlockId>, entry: LeafEntry) -> InsertTarget {
        match self.storage.root {
            Root::Empty => {
                assert!(
                    prev.is_none(),
                    "cannot insert after a block in an empty tree"
                );
                self.storage.push_first_entry(entry);
                InsertTarget::DoneEmpty
            }
            Root::Leaf(idx) => {
                let after_slot = prev.map(|p| {
                    let (l, slot) = self.storage.locate(p);
                    debug_assert_eq!(l, idx);
                    slot
                });
                InsertTarget::At {
                    leaf_idx: idx,
                    after_slot,
                }
            }
            Root::Node(root_idx) => match prev {
                Some(p) => {
                    let (l, slot) = self.storage.locate(p);
                    InsertTarget::At {
                        leaf_idx: l,
                        after_slot: Some(slot),
                    }
                }
                None => {
                    let leftmost = self.storage.descend_leftmost_leaf(root_idx);
                    InsertTarget::At {
                        leaf_idx: leftmost,
                        after_slot: None,
                    }
                }
            },
        }
    }

    fn attach_after_leaf_split(
        &mut self,
        left_idx: LeafIdx,
        left_visible: u64,
        right_idx: LeafIdx,
        right_visible: u64,
    ) {
        if matches!(self.storage.root, Root::Leaf(_)) {
            let node_idx = self.storage.make_root_node_for_two_leaves(
                left_idx,
                left_visible,
                right_idx,
                right_visible,
            );
            self.storage.root = Root::Node(node_idx);
            return;
        }
        self.storage
            .propagate_leaf_split(left_idx, left_visible, right_idx, right_visible);
    }

    fn shrink_left_half_for_split(
        &mut self,
        leaf_idx: LeafIdx,
        slot: u8,
        offset: u64,
        new_id: BlockId,
    ) -> LeafEntry {
        let leaf = &mut self.storage.leaves[leaf_idx.0 as usize];
        let entry = leaf.entries[slot as usize]
            .as_mut()
            .expect("located entry must exist");
        assert!(
            offset > 0 && offset < entry.len,
            "split offset {} out of range for entry len {}",
            offset,
            entry.len
        );
        let was_deleted = entry.is_deleted;
        let original_len = entry.len;
        entry.len = offset;
        LeafEntry {
            id: new_id,
            len: original_len - offset,
            is_deleted: was_deleted,
        }
    }
}
