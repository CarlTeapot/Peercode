use super::{Document, RemoteChange};
use crate::error::DocumentError;
use crate::store::DeleteSet;
use crate::structs::Block;
use crate::types::{BlockId, Clock};
use std::mem;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockReadiness {
    Ready,
    Pending,
    Duplicate,
}

impl Document {
    /// Integrate a remote block. Returns the list of visible-text changes
    pub fn remote_insert(&mut self, block: Block) -> Result<Vec<RemoteChange>, DocumentError> {
        match self.classify_block(&block) {
            BlockReadiness::Duplicate => return Ok(Vec::new()),
            BlockReadiness::Pending => {
                self.pending_blocks.push(block);
                return Ok(Vec::new());
            }
            BlockReadiness::Ready => {}
        }

        let mut changes = Vec::new();
        changes.push(self.integrate_ready_block(block)?);
        self.drain_pending(&mut changes)?;
        Ok(changes)
    }

    /// Apply a remote delete set. Returns a list of visible-text Delete events
    /// the UI should replay in order.
    pub fn apply_delete_set(
        &mut self,
        remote: &DeleteSet,
    ) -> Result<Vec<RemoteChange>, DocumentError> {
        let mut changes = Vec::new();
        let unapplied = self.try_apply_delete_set(remote, &mut changes)?;
        if !unapplied.is_empty() {
            self.pending_delete_sets.push(unapplied);
        }
        self.seen_delete_set.merge(remote);
        Ok(changes)
    }

    /// Integrate a block that has already been classified as `Ready`, and
    /// advance the state vector.
    fn integrate_ready_block(&mut self, block: Block) -> Result<RemoteChange, DocumentError> {
        let client = block.id.client;
        let block_id = block.id;
        let end_clock = block.id.clock.value + block.len;
        let content = block.content().to_string();

        self.pre_split_for_block(&block)?;
        self.integrate(block)?;
        self.state_vector.update(client, end_clock);

        let position = self.visible_position_of(block_id);
        Ok(RemoteChange::Insert { position, content })
    }

    fn classify_block(&self, block: &Block) -> BlockReadiness {
        let seen = self.state_vector.get(&block.id.client);
        let clock = block.id.clock.value;

        if seen > clock {
            return BlockReadiness::Duplicate;
        }
        if seen < clock {
            return BlockReadiness::Pending;
        }
        if let Some(ol) = block.origin_left
            && !self.store.contains_key(&ol)
        {
            return BlockReadiness::Pending;
        }
        if let Some(or) = block.origin_right
            && !self.store.contains_key(&or)
        {
            return BlockReadiness::Pending;
        }
        BlockReadiness::Ready
    }

    /// Repeatedly drain pending blocks and pending delete sets until a pass
    /// makes no further progress.
    fn drain_pending(&mut self, changes: &mut Vec<RemoteChange>) -> Result<(), DocumentError> {
        loop {
            let mut progress = false;

            let candidates: Vec<Block> = mem::take(&mut self.pending_blocks);
            let mut still_pending_blocks: Vec<Block> = Vec::new();
            for block in candidates {
                match self.classify_block(&block) {
                    BlockReadiness::Ready => {
                        changes.push(self.integrate_ready_block(block)?);
                        progress = true;
                    }
                    BlockReadiness::Duplicate => {
                        progress = true;
                    }
                    BlockReadiness::Pending => {
                        still_pending_blocks.push(block);
                    }
                }
            }
            self.pending_blocks = still_pending_blocks;

            let candidate_ds: Vec<DeleteSet> = mem::take(&mut self.pending_delete_sets);
            let mut still_pending_ds: Vec<DeleteSet> = Vec::new();
            for ds in candidate_ds {
                let unapplied = self.try_apply_delete_set(&ds, changes)?;
                if unapplied.is_empty() {
                    progress = true;
                } else {
                    still_pending_ds.push(unapplied);
                }
            }
            self.pending_delete_sets = still_pending_ds;

            if !progress {
                break;
            }
        }
        Ok(())
    }

    /// Attempt to apply every range in `remote`,
    fn try_apply_delete_set(
        &mut self,
        remote: &DeleteSet,
        changes: &mut Vec<RemoteChange>,
    ) -> Result<DeleteSet, DocumentError> {
        let mut unapplied = DeleteSet::new();

        for (client, range) in remote.iter() {
            let mut current_clock = range.start;
            let end_clock = range.end();

            while current_clock < end_clock {
                let id = BlockId::new(*client, Clock::new(current_clock));

                let (block_start, block_len, block_id, was_deleted) = match self.store.get(&id) {
                    Some(b) => (b.id.clock.value, b.len, b.id, b.is_deleted),
                    None => {
                        unapplied.add(id, end_clock - current_clock);
                        break;
                    }
                };

                let offset = current_clock - block_start;
                if offset > 0 {
                    self.split_block(block_id, offset)?;
                    continue;
                }

                let remaining_delete = end_clock - current_clock;
                if block_len > remaining_delete {
                    self.split_block(block_id, remaining_delete)?;
                }

                let position_before = if !was_deleted {
                    Some(self.visible_position_of(block_id))
                } else {
                    None
                };

                let actual_len = self
                    .store
                    .mark_deleted(&block_id)
                    .ok_or(DocumentError::BlockNotFound(block_id))?
                    .len;

                if let Some(position) = position_before {
                    changes.push(RemoteChange::Delete {
                        position,
                        length: actual_len,
                    });
                }

                current_clock += actual_len;
            }
        }

        Ok(unapplied)
    }
}
