use super::Document;
use log::trace;

impl Document {
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
}
