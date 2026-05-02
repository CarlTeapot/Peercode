use std::error::Error;
use std::fmt;

use crate::store::DeleteSet;
use crate::structs::Block;
use crate::types::{BlockId, ClientId};

pub const SNAPSHOT_VERSION: u8 = 1;

#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct SnapshotBlock {
    pub id: BlockId,
    pub origin_left: Option<BlockId>,
    pub origin_right: Option<BlockId>,
    pub left: Option<BlockId>,
    pub right: Option<BlockId>,
    pub content: String,
    pub is_deleted: bool,
    pub len: u64,
}

impl From<&Block> for SnapshotBlock {
    fn from(b: &Block) -> Self {
        SnapshotBlock {
            id: b.id,
            origin_left: b.origin_left,
            origin_right: b.origin_right,
            left: b.left(),
            right: b.right(),
            content: b.content().to_string(),
            is_deleted: b.is_deleted,
            len: b.len,
        }
    }
}

impl From<SnapshotBlock> for Block {
    fn from(s: SnapshotBlock) -> Self {
        Block::restore(
            s.id,
            s.origin_left,
            s.origin_right,
            s.left,
            s.right,
            s.content,
            s.is_deleted,
            s.len,
        )
    }
}

#[derive(Debug, Clone, bitcode::Encode, bitcode::Decode)]
pub struct Snapshot {
    pub version: u8,
    pub client_id: ClientId,
    pub blocks: Vec<SnapshotBlock>,
    pub state_vector: Vec<(ClientId, u64)>,
    pub delete_set: DeleteSet,
    pub seen_delete_set: DeleteSet,
    pub head: Option<BlockId>,
    pub pending_blocks: Vec<SnapshotBlock>,
    pub pending_delete_sets: Vec<DeleteSet>,
}

#[derive(Debug)]
pub enum SnapshotError {
    VersionMismatch { expected: u8, got: u8 },
    Decode(bitcode::Error),
}

impl fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SnapshotError::VersionMismatch { expected, got } => {
                write!(
                    f,
                    "snapshot version mismatch: expected {expected}, got {got}"
                )
            }
            SnapshotError::Decode(e) => write!(f, "snapshot decode failed: {e}"),
        }
    }
}

impl Error for SnapshotError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SnapshotError::Decode(e) => Some(e),
            _ => None,
        }
    }
}

impl Snapshot {
    pub fn encode(&self) -> Vec<u8> {
        bitcode::encode(self)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, SnapshotError> {
        let snap: Snapshot = bitcode::decode(bytes).map_err(SnapshotError::Decode)?;
        if snap.version != SNAPSHOT_VERSION {
            return Err(SnapshotError::VersionMismatch {
                expected: SNAPSHOT_VERSION,
                got: snap.version,
            });
        }
        Ok(snap)
    }
}
