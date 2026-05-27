use crate::index::PositionIndex;
use crate::store::{DeleteSet, StateVector, StructStore};
use crate::structs::Block;
use crate::types::{BlockId, ClientId};

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
    pub position_index: PositionIndex,
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
        let mut position_index = PositionIndex::new();
        let mut entries: Vec<(BlockId, u64, bool)> = Vec::new();
        let mut curr = head;
        while let Some(id) = curr {
            match store.get(&id) {
                Some(block) => {
                    entries.push((id, block.len, block.is_deleted));
                    curr = block.right();
                }
                None => break,
            }
        }
        position_index.rebuild_from_order(entries.into_iter());

        Document {
            client_id,
            store,
            state_vector,
            delete_set,
            seen_delete_set,
            head,
            position_index,
            pending_blocks,
            pending_delete_sets,
        }
    }
}

#[cfg(test)]
mod tests;
