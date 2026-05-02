use super::Document;
use crate::snapshot::{SNAPSHOT_VERSION, Snapshot, SnapshotBlock, SnapshotError};
use crate::store::{StateVector, StructStore};
use crate::structs::Block;
use crate::types::ClientId;

impl Document {
    pub fn to_snapshot(&self) -> Snapshot {
        let blocks = self.store.all_blocks().map(SnapshotBlock::from).collect();
        let state_vector = self.state_vector.iter().map(|(&c, &v)| (c, v)).collect();
        let pending_blocks = self
            .pending_blocks
            .iter()
            .map(SnapshotBlock::from)
            .collect();

        Snapshot {
            version: SNAPSHOT_VERSION,
            client_id: self.client_id,
            blocks,
            state_vector,
            delete_set: self.delete_set.clone(),
            seen_delete_set: self.seen_delete_set.clone(),
            head: self.head,
            pending_blocks,
            pending_delete_sets: self.pending_delete_sets.clone(),
        }
    }

    pub fn from_snapshot(snap: Snapshot) -> Result<Self, SnapshotError> {
        if snap.version != SNAPSHOT_VERSION {
            return Err(SnapshotError::VersionMismatch {
                expected: SNAPSHOT_VERSION,
                got: snap.version,
            });
        }

        let blocks: Vec<Block> = snap.blocks.into_iter().map(Block::from).collect();
        let store = StructStore::from_blocks(blocks);
        let state_vector = StateVector::from_entries(snap.state_vector);
        let pending: Vec<Block> = snap.pending_blocks.into_iter().map(Block::from).collect();

        Ok(Document::restore(
            snap.client_id,
            store,
            state_vector,
            snap.delete_set,
            snap.seen_delete_set,
            snap.head,
            pending,
            snap.pending_delete_sets,
        ))
    }

    pub fn fork(&self, new_client_id: ClientId) -> Self {
        let mut snap = self.to_snapshot();
        snap.client_id = new_client_id;
        snap.pending_blocks.clear();
        snap.pending_delete_sets.clear();
        Document::from_snapshot(snap).expect("fork: snapshot round-trip failed")
    }
}
