use std::collections::VecDeque;

const DEFAULT_CAPACITY: usize = 512;

#[derive(Debug, Clone, Copy)]
pub enum PositionDelta {
    Insert { at: u64, len: u64 },
    Delete { at: u64, len: u64 },
}

impl PositionDelta {
    pub fn shift(&self, pos: u64) -> u64 {
        match *self {
            PositionDelta::Insert { at, len } => {
                if pos >= at {
                    pos.saturating_add(len)
                } else {
                    pos
                }
            }
            PositionDelta::Delete { at, len } => {
                if pos <= at {
                    pos
                } else if pos >= at.saturating_add(len) {
                    pos.saturating_sub(len)
                } else {
                    at
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct OpLog {
    capacity: usize,
    entries: VecDeque<(u64, PositionDelta)>,
}

impl OpLog {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        OpLog {
            capacity,
            entries: VecDeque::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, seq: u64, delta: PositionDelta) {
        if self.entries.len() == self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back((seq, delta));
    }

    pub fn transform(&self, position: u64, base_seq: u64) -> u64 {
        let mut pos = position;
        for (seq, delta) in &self.entries {
            if *seq > base_seq {
                pos = delta.shift(pos);
            }
        }
        pos
    }
}

impl Default for OpLog {
    fn default() -> Self {
        Self::new()
    }
}
