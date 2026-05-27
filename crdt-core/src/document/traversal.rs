use super::Document;
use crate::types::BlockId;
use log::{debug, trace};

impl Document {
    pub fn get_text(&self) -> String {
        trace!("fetching document visible text");
        let mut text = String::new();
        let mut curr = self.head;

        #[cfg(debug_assertions)]
        let max_steps = self.store.total_blocks().saturating_add(1);
        #[cfg(debug_assertions)]
        let mut steps: usize = 0;

        while let Some(id) = curr {
            #[cfg(debug_assertions)]
            {
                steps += 1;
                debug_assert!(
                    steps <= max_steps,
                    "cycle detected in document linked list at block {id:?}"
                );
            }

            if let Some(block) = self.store.get(&id) {
                if !block.is_deleted {
                    text.push_str(block.content());
                }
                curr = block.right();
            } else {
                break;
            }
        }
        text
    }

    #[cfg(debug_assertions)]
    pub(super) fn assert_index_matches_linked_list(&self) {
        let mut pos = 0u64;
        let mut curr = self.head;

        let max_steps = self.store.total_blocks().saturating_add(1);
        let mut steps: usize = 0;

        while let Some(id) = curr {
            steps += 1;
            debug_assert!(
                steps <= max_steps,
                "cycle detected in document linked list at block {id:?}"
            );

            let block = self
                .store
                .get(&id)
                .expect("linked-list points to block missing from store");
            let tree_pos = self.position_index.position_of(id);
            assert_eq!(
                tree_pos,
                Some(pos),
                "position_of({:?}) = {:?}, linked-list position = {}",
                id,
                tree_pos,
                pos,
            );
            if !block.is_deleted {
                pos += block.len;
            }
            curr = block.right();
        }

        let tree_total = self.position_index.visible_len();
        assert_eq!(
            tree_total, pos,
            "position_index.visible_len() = {}, linked-list visible total = {}",
            tree_total, pos,
        );

        self.position_index
            .debug_validate()
            .expect("position index invariants must hold");
    }

    #[cfg(debug_assertions)]
    pub fn debug_linked_list(&self) -> String {
        trace!("fetching document linked list visualization");
        let mut parts = Vec::new();
        let mut curr = self.head;
        let max_steps = self.store.total_blocks().saturating_add(1);
        let mut steps: usize = 0;

        while let Some(id) = curr {
            steps += 1;
            debug_assert!(
                steps <= max_steps,
                "cycle detected in document linked list at block {id:?}"
            );

            if let Some(block) = self.store.get(&id) {
                let content = if block.content().is_empty() {
                    "<empty>".to_string()
                } else {
                    block.content().replace('\n', "\\n")
                };

                if block.is_deleted {
                    parts.push(format!("[DEL:{content}]"));
                } else {
                    parts.push(content);
                }

                curr = block.right();
            } else {
                parts.push("<broken-link>".to_string());
                break;
            }
        }

        if parts.is_empty() {
            "<empty>".to_string()
        } else {
            parts.join(" --- ")
        }
    }

    /// Visible-text character position of the block identified by `target`.
    /// Backed by the position index: O(log n) instead of an O(n) linked-list walk.
    pub(super) fn visible_position_of(&self, target: BlockId) -> u64 {
        self.position_index
            .position_of(target)
            .unwrap_or_else(|| self.position_index.visible_len())
    }

    /// Locate the block and intra-block offset that corresponds to a visible
    /// character position. Returns `(block_id, offset_within_block, tail_id)`.
    /// Backed by the position index: O(log n) instead of O(n).
    pub(super) fn get_block_and_offset_by_position(
        &self,
        position: u64,
    ) -> (Option<BlockId>, u64, Option<BlockId>) {
        debug!("get_block_and_offset_by_position({:?})", position);
        let r = self.position_index.find_at_position(position);
        (r.id, r.offset, r.tail_id)
    }
}
