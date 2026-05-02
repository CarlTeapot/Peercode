use super::Document;
use crate::error::DocumentError;
use crate::structs::Block;
use crate::types::BlockId;
use log::{debug, trace};

impl Document {
    fn find_insert_position(&self, block: &Block) -> Result<Option<BlockId>, DocumentError> {
        use std::collections::HashSet;

        let mut left = block.origin_left;
        let right = block.origin_right;

        let mut scanning_id = if let Some(id) = left {
            self.store
                .get(&id)
                .ok_or(DocumentError::BlockNotFound(id))?
                .right()
        } else {
            self.head
        };

        let mut seen: HashSet<BlockId> = HashSet::new();

        while let Some(curr_id) = scanning_id {
            if Some(curr_id) == right {
                trace!(
                    "insert-position outcome: reached right boundary {:?}",
                    right
                );
                break;
            }

            let curr_block = self
                .store
                .get(&curr_id)
                .ok_or(DocumentError::BlockNotFound(curr_id))?;

            let o_l = curr_block.origin_left;

            seen.insert(curr_id);

            let ol_is_left_of_ours = match (o_l, block.origin_left) {
                (None, Some(_)) => true,
                (Some(x), _) if Some(x) != block.origin_left && !seen.contains(&x) => true,
                _ => false,
            };

            if ol_is_left_of_ours {
                trace!(
                    "insert-position outcome: stop before {:?} due to non-crossing-origin rule",
                    curr_id
                );
                break;
            }

            if o_l == block.origin_left && block.id.client.value < curr_block.id.client.value {
                trace!(
                    "insert-position outcome: stop before {:?} due to client tie-break (incoming={} < existing={})",
                    curr_id, block.id.client.value, curr_block.id.client.value
                );
                break;
            }

            left = Some(curr_id);
            scanning_id = curr_block.right();
        }

        debug!(
            "insert-position outcome: block {:?} resolved with left neighbor {:?}",
            block.id, left
        );
        Ok(left)
    }

    /// Wire `block_id` into the linked list between `final_left` and `final_right`.
    fn link_block(
        &mut self,
        block_id: BlockId,
        final_left: Option<BlockId>,
        final_right: Option<BlockId>,
    ) -> Result<(), DocumentError> {
        if let Some(l_id) = final_left {
            self.store
                .get_mut(&l_id)
                .ok_or(DocumentError::BlockNotFound(l_id))?
                .set_right(Some(block_id));
        } else {
            self.head = Some(block_id);
        }

        if let Some(r_id) = final_right {
            self.store
                .get_mut(&r_id)
                .ok_or(DocumentError::BlockNotFound(r_id))?
                .set_left(Some(block_id));
        }

        let b_mut = self
            .store
            .get_mut(&block_id)
            .ok_or(DocumentError::BlockNotFound(block_id))?;
        b_mut.set_left(final_left);
        b_mut.set_right(final_right);
        debug!(
            "link-block outcome: {:?} linked between left={:?} and right={:?}",
            block_id,
            b_mut.left(),
            b_mut.right()
        );

        Ok(())
    }

    /// Insert `block` into the store and link it.
    pub(super) fn integrate(&mut self, block: Block) -> Result<BlockId, DocumentError> {
        let block_id = block.id;
        debug!(
            "integrate start: id={:?}, origin_left={:?}, origin_right={:?}, len={}",
            block_id, block.origin_left, block.origin_right, block.len
        );

        let final_left = self.find_insert_position(&block)?;
        let final_right = if let Some(l_id) = final_left {
            self.store
                .get(&l_id)
                .ok_or(DocumentError::BlockNotFound(l_id))?
                .right()
        } else {
            self.head
        };
        self.store.insert(block);
        self.link_block(block_id, final_left, final_right)?;
        debug!(
            "integrate outcome: block {:?} inserted with neighbors left={:?}, right={:?}",
            block_id, final_left, final_right
        );

        Ok(block_id)
    }

    /// Split `block_id` at `offset`. Returns the id of the newly created
    /// right half, or `None` if the split was a no-op (offset 0 or past end).
    pub(super) fn split_block(
        &mut self,
        block_id: BlockId,
        offset: u64,
    ) -> Result<Option<BlockId>, DocumentError> {
        let (right_block_id, new_block) = {
            let block = self
                .store
                .get_mut(&block_id)
                .ok_or(DocumentError::BlockNotFound(block_id))?;

            if offset == 0 || offset >= block.len {
                trace!(
                    "split outcome: no-op for {:?} because offset={} is outside 1..{}",
                    block_id, offset, block.len
                );
                return Ok(None);
            };

            let new_block_content: String = block.content().chars().skip(offset as usize).collect();
            block.set_content(block.content().chars().take(offset as usize).collect());

            let new_block_id = BlockId {
                client: block.id.client,
                clock: block.id.clock.advance(offset),
            };
            let old_right_block_id = block.right();
            let mut new_block: Block = Block::new(
                new_block_id,
                Some(block.id),
                block.origin_right,
                new_block_content,
            );

            new_block.is_deleted = block.is_deleted;
            new_block.set_right(old_right_block_id);

            block.set_right(Some(new_block_id));

            (old_right_block_id, new_block)
        };

        let new_block_id = new_block.id;
        self.store.insert(new_block);

        if let Some(right_id) = right_block_id {
            self.store
                .get_mut(&right_id)
                .ok_or(DocumentError::BlockNotFound(right_id))?
                .set_left(Some(new_block_id));
        }

        debug!(
            "split outcome: {:?} split at offset {} creating {:?}",
            block_id, offset, new_block_id
        );
        Ok(Some(new_block_id))
    }

    /// Split the block that currently contains clock `id.clock` so that a
    /// boundary exists exactly at `id.clock`. No-op if `id` is not found or
    fn ensure_block_split_at(&mut self, id: BlockId) -> Result<(), DocumentError> {
        let block = match self.store.get(&id) {
            Some(b) => b,
            None => {
                trace!("ensure-split outcome: no-op, target {:?} not found", id);
                return Ok(());
            }
        };
        let block_start = block.id.clock.value;
        if block_start == id.clock.value {
            trace!(
                "ensure-split outcome: no-op, target {:?} already aligned",
                id
            );
            return Ok(());
        }
        let offset = id.clock.value - block_start;
        let block_id = block.id;
        self.split_block(block_id, offset)?;
        debug!(
            "ensure-split outcome: split source {:?} at offset {} for target {:?}",
            block_id, offset, id
        );
        Ok(())
    }

    /// Ensure that any local block whose *middle* is referenced by
    /// `block.origin_left` or `block.origin_right` is split at the referenced
    /// clock so that integration sees a block boundary there.
    pub(super) fn pre_split_for_block(&mut self, block: &Block) -> Result<(), DocumentError> {
        if let Some(ol) = block.origin_left {
            let split_point = BlockId::new(ol.client, ol.clock.advance(1));
            self.ensure_block_split_at(split_point)?;
        }
        if let Some(or_id) = block.origin_right {
            self.ensure_block_split_at(or_id)?;
        }
        debug!(
            "pre-split outcome: prepared boundaries for block {:?} (left={:?}, right={:?})",
            block.id, block.origin_left, block.origin_right
        );
        Ok(())
    }
}
