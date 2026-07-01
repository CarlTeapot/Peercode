use crate::index::PositionIndex;
use crate::store::{DeleteSet, StateVector, StructStore};
use crate::structs::Block;
use crate::types::{BlockId, ClientId};

#[cfg(debug_assertions)]
mod debug;
mod integrate;
mod ops;
mod pending;
mod persist;
mod traversal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteChange {
    Insert { position: u64, content: String },
    Delete { position: u64, length: u64 },
}

#[derive(Debug)]
pub struct Document {
    pub client_id: ClientId,
    pub store: StructStore,
    pub state_vector: StateVector,
    pub delete_set: DeleteSet,
    pub seen_delete_set: DeleteSet,
    pub head: Option<BlockId>,
    pub(crate) position_index: PositionIndex,
    pending_blocks: Vec<Block>,
    pending_delete_sets: Vec<DeleteSet>,
}

impl Document {
    pub fn new(client_id: ClientId) -> Self {
        Document {
            client_id,
            store: StructStore::new(),
            state_vector: StateVector::new(),
            delete_set: DeleteSet::new(),
            seen_delete_set: DeleteSet::new(),
            head: None,
            position_index: PositionIndex::new(),
            pending_blocks: Vec::new(),
            pending_delete_sets: Vec::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn restore(
        client_id: ClientId,
        store: StructStore,
        state_vector: StateVector,
        delete_set: DeleteSet,
        seen_delete_set: DeleteSet,
        head: Option<BlockId>,
        pending_blocks: Vec<Block>,
        pending_delete_sets: Vec<DeleteSet>,
    ) -> Self {
        let mut doc = Document {
            client_id,
            store,
            state_vector,
            delete_set,
            seen_delete_set,
            head,
            position_index: PositionIndex::new(),
            pending_blocks,
            pending_delete_sets,
        };
        doc.rebuild_position_index_from_links();
        doc
    }

    pub(super) fn rebuild_position_index_from_links(&mut self) {
        let mut entries: Vec<(BlockId, u64, bool)> = Vec::new();
        let mut curr = self.head;
        while let Some(id) = curr {
            match self.store.get(&id) {
                Some(block) => {
                    entries.push((id, block.len, block.is_deleted));
                    curr = block.right();
                }
                None => break,
            }
        }
        self.position_index.rebuild_from_order(entries.into_iter());
    }

    pub(super) fn unlink_block(&mut self, id: BlockId) -> bool {
        let Some(block) = self.store.get(&id).cloned() else {
            return false;
        };
        let left = block.left();
        let right = block.right();

        if let Some(left_id) = left {
            if let Some(left_block) = self.store.get_mut(&left_id) {
                left_block.set_right(right);
            }
        } else {
            self.head = right;
        }

        if let Some(right_id) = right
            && let Some(right_block) = self.store.get_mut(&right_id)
        {
            right_block.set_left(left);
        }

        self.store.remove(&id).is_some()
    }
}

#[cfg(test)]
impl Document {
    fn pending_delete_set_count(&self) -> usize {
        self.pending_delete_sets.len()
    }
}

#[cfg(test)]
mod tests;
