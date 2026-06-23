use super::{Document, RemoteChange};
use crate::error::DocumentError;
use crate::store::DeleteSet;
use crate::structs::Block;
use crate::types::{BlockId, Clock};
use crate::wire::WireBlock;
use log::{debug, trace, warn};

impl Document {
    fn resolve_origins(
        &mut self,
        position: u64,
    ) -> Result<(Option<BlockId>, Option<BlockId>), DocumentError> {
        debug!("resolve-origins start: position={position}");
        let (block, offset, tail_id) = self.get_block_and_offset_by_position(position);

        let Some(block_id) = block else {
            if offset > 0 {
                warn!(
                    "resolve-origins outcome: out of bounds at position {} (offset={})",
                    position, offset
                );
                return Err(DocumentError::OutOfBounds(position));
            }
            debug!(
                "resolve-origins outcome: append at tail (left={:?}, right=None)",
                tail_id
            );
            return Ok((tail_id, None));
        };

        if offset == 0 {
            let block_ref = self
                .store
                .get(&block_id)
                .ok_or(DocumentError::BlockNotFound(block_id))?;
            debug!(
                "resolve-origins outcome: boundary before block {:?} (left={:?}, right={:?})",
                block_id,
                block_ref.left(),
                Some(block_id)
            );
            return Ok((block_ref.left(), Some(block_id)));
        }

        self.split_block(block_id, offset)?;
        let left_ref = self
            .store
            .get(&block_id)
            .ok_or(DocumentError::BlockNotFound(block_id))?;
        let origin_left_id = BlockId::new(block_id.client, block_id.clock.advance(offset - 1));
        debug!(
            "resolve-origins outcome: split block {:?} at offset {} (left={:?}, right={:?})",
            block_id,
            offset,
            Some(origin_left_id),
            left_ref.right()
        );
        Ok((Some(origin_left_id), left_ref.right()))
    }

    pub fn local_insert(
        &mut self,
        position: u64,
        content: &str,
    ) -> Result<Option<WireBlock>, DocumentError> {
        if content.is_empty() {
            trace!("local_insert skipped: empty content");
            return Ok(None);
        }
        debug!(
            "local_insert start: position={position}, len={}",
            content.len()
        );

        let (left_origin, right_origin) = self.resolve_origins(position)?;

        let next_clock = self.state_vector.get(&self.client_id);
        let new_id = BlockId::new(self.client_id, Clock::new(next_clock));
        let new_block = Block::new(new_id, left_origin, right_origin, content.to_string());
        let block_len = new_block.len;
        let wire = WireBlock::from(&new_block);

        self.integrate(new_block)?;
        self.state_vector
            .update(self.client_id, next_clock + block_len);
        debug!(
            "local_insert integrated: id={:?}, left={:?}, right={:?}, len={}",
            new_id, left_origin, right_origin, block_len
        );

        Ok(Some(wire))
    }

    /// Delete `length` visible characters starting at `position`.
    /// Returns a `DeleteSet` containing only the ranges this call tombstoned
    /// (the diff, not the cumulative document `delete_set`). Empty when
    /// `length == 0`.
    pub fn delete(&mut self, position: u64, length: u64) -> Result<DeleteSet, DocumentError> {
        if length == 0 {
            trace!("delete skipped: zero length");
            return Ok(DeleteSet::new());
        }
        debug!("delete start: position={position}, length={length}");

        let (first_id, start_offset, _) = self.get_block_and_offset_by_position(position);

        let Some(mut current_id) = first_id else {
            warn!(
                "delete outcome: start position {} is out of bounds (no visible block)",
                position
            );
            return Err(DocumentError::OutOfBounds(position));
        };

        if start_offset > 0
            && let Some(new_id) = self.split_block(current_id, start_offset)?
        {
            trace!(
                "delete path: start block {:?} split at offset {}, continuing at {:?}",
                current_id, start_offset, new_id
            );
            current_id = new_id;
        }

        let mut remaining = length;
        let mut diff = DeleteSet::new();

        while remaining > 0 {
            let (is_deleted, block_len, right_id) = {
                let block = match self.store.get(&current_id) {
                    Some(b) => b,
                    None => break,
                };
                (block.is_deleted, block.len, block.right())
            };

            if is_deleted {
                match right_id {
                    Some(next) => {
                        current_id = next;
                        continue;
                    }
                    None => break,
                }
            }

            if block_len > remaining {
                self.split_block(current_id, remaining)?;
            }

            let (deleted_len, next_id) = self.mark_block_deleted(&current_id)?;

            self.delete_set.add(current_id, deleted_len);
            diff.add(current_id, deleted_len);
            remaining = remaining.saturating_sub(deleted_len);

            match next_id {
                Some(next) => current_id = next,
                None => break,
            }
        }

        if remaining > 0 {
            warn!(
                "delete out of bounds: requested_end={}, remaining={remaining}",
                position + length
            );
            return Err(DocumentError::OutOfBounds(position + length - remaining));
        }

        debug!("delete finished: tombstoned_ranges={}", diff.iter().count());
        Ok(diff)
    }

    /// Reclaim tombstones covered by `confirmed`: defensively re-apply the
    /// deletes (returning any this peer hadn't seen), erase the tombstone content
    /// (blocks are kept, so ids still resolve and integration is unaffected), and
    /// prune the snapshot-only delete sets. Idempotent.
    pub fn collect_garbage(
        &mut self,
        confirmed: &DeleteSet,
    ) -> Result<Vec<RemoteChange>, DocumentError> {
        debug!(
            "garbage-collect start: confirmed_ranges={}",
            confirmed.iter().count()
        );

        let changes = self.apply_delete_set(confirmed)?;

        let mut erased_blocks = 0_u64;
        for (client, range) in confirmed.iter() {
            let mut current_clock = range.start;
            let end_clock = range.end();

            while current_clock < end_clock {
                let id = BlockId::new(*client, Clock::new(current_clock));

                let Some(block) = self.store.get(&id) else {
                    break;
                };
                let next_clock = block.id.clock.value + block.len;

                if self.store.erase_content(&id) {
                    erased_blocks += 1;
                }
                current_clock = next_clock;
            }
        }

        self.prune_after_gc(confirmed);

        debug!(
            "garbage-collect outcome: erased {} tombstoned blocks, {} replayed change(s)",
            erased_blocks,
            changes.len()
        );
        Ok(changes)
    }

    /// Drop `confirmed` from the snapshot-only `delete_set`/`seen_delete_set`
    /// (the deletion stays encoded by each block's `is_deleted` flag). Idempotent.
    pub fn prune_after_gc(&mut self, confirmed: &DeleteSet) {
        self.delete_set.subtract(confirmed);
        self.seen_delete_set.subtract(confirmed);
    }
}
