use crate::index::structs::storage::Storage;

#[derive(Debug)]
pub struct PositionIndex {
    pub(in crate::index) storage: Storage,
}

impl PositionIndex {
    pub fn new() -> Self {
        PositionIndex {
            storage: Storage::new(),
        }
    }
}

impl Default for PositionIndex {
    fn default() -> Self {
        Self::new()
    }
}
