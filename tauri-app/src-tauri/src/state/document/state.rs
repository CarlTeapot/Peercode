use crdt_core::Document;

use crate::state::document::op_log::OpLog;

pub struct DocState {
    pub doc: Document,
    pub op_log: OpLog,
    pub next_seq: u64,
}

impl DocState {
    pub fn new(doc: Document) -> Self {
        DocState {
            doc,
            op_log: OpLog::new(),
            next_seq: 1,
        }
    }

    pub fn mint_seq(&mut self) -> u64 {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.saturating_add(1);
        seq
    }

    pub fn visible_length(&self) -> u64 {
        self.doc.get_text().chars().count() as u64
    }

    pub fn reset_history(&mut self) {
        self.op_log = OpLog::new();
        self.next_seq = 1;
    }
}
