use crate::types::BlockId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindResult {
    pub id: Option<BlockId>,
    pub offset: u64,
    pub tail_id: Option<BlockId>,
}
