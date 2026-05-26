use crate::snapshot::{Snapshot, SnapshotError};
use crate::store::DeleteSet;
use crate::structs::Block;
use crate::types::BlockId;
use log::trace;
use std::error::Error;
use std::fmt;

pub const OP_PREFIX: u8 = 0x00;
pub const SNAPSHOT_PREFIX: u8 = 0x01;
pub const PREFIX_CONTROL: u8 = 0x02;
pub const CONTROL_SESSION_ENDED: u8 = 0x01;
pub const CONTROL_SNAPSHOT_REQUEST: u8 = 0x02;

#[derive(Debug, Clone, PartialEq, Eq, bitcode::Encode, bitcode::Decode)]
pub struct WireBlock {
    pub id: BlockId,
    pub origin_left: Option<BlockId>,
    pub origin_right: Option<BlockId>,
    pub content: String,
}

impl From<&Block> for WireBlock {
    fn from(b: &Block) -> Self {
        WireBlock {
            id: b.id,
            origin_left: b.origin_left,
            origin_right: b.origin_right,
            content: b.content().to_string(),
        }
    }
}

impl From<WireBlock> for Block {
    fn from(w: WireBlock) -> Self {
        Block::new(w.id, w.origin_left, w.origin_right, w.content)
    }
}

#[derive(Debug, Clone, PartialEq, bitcode::Encode, bitcode::Decode)]
pub enum OpMessage {
    Insert(WireBlock),
    Delete(DeleteSet),
}

#[derive(Debug)]
pub enum WireError {
    EmptyFrame,
    UnknownPrefix(u8),
    NotAnOp,
    NotASnapshot,
    Decode(bitcode::Error),
    SnapshotDecode(SnapshotError),
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WireError::EmptyFrame => write!(f, "wire frame is empty"),
            WireError::UnknownPrefix(b) => {
                write!(f, "unknown wire prefix byte: 0x{b:02X}")
            }
            WireError::NotAnOp => {
                write!(f, "wire frame carries a snapshot, not an op")
            }
            WireError::NotASnapshot => {
                write!(f, "wire frame carries an op, not a snapshot")
            }
            WireError::Decode(e) => write!(f, "bitcode decode failed: {e}"),
            WireError::SnapshotDecode(e) => write!(f, "snapshot decode failed: {e}"),
        }
    }
}

impl Error for WireError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            WireError::Decode(e) => Some(e),
            WireError::SnapshotDecode(e) => Some(e),
            _ => None,
        }
    }
}

pub fn encode_op(msg: &OpMessage) -> Vec<u8> {
    trace!("encode operation requested: {:?}", msg);
    let payload = bitcode::encode(msg);
    let mut frame = Vec::with_capacity(1 + payload.len());
    frame.push(OP_PREFIX);
    frame.extend_from_slice(&payload);
    trace!("encode frame encoded: {frame:?}");
    frame
}

pub fn decode_op(frame: &[u8]) -> Result<OpMessage, WireError> {
    trace!("decode operation requested: {:?}", frame);
    let (&prefix, payload) = frame.split_first().ok_or(WireError::EmptyFrame)?;
    match prefix {
        OP_PREFIX => bitcode::decode(payload).map_err(WireError::Decode),
        SNAPSHOT_PREFIX => Err(WireError::NotAnOp),
        b => Err(WireError::UnknownPrefix(b)),
    }
}

pub fn encode_snapshot(snap: &Snapshot) -> Vec<u8> {
    trace!("encode snapshot requested");
    let payload = snap.encode();
    let mut frame = Vec::with_capacity(1 + payload.len());
    frame.push(SNAPSHOT_PREFIX);
    frame.extend_from_slice(&payload);
    trace!("encode snapshot frame: {} bytes", frame.len());
    frame
}

pub fn decode_snapshot(frame: &[u8]) -> Result<Snapshot, WireError> {
    trace!("decode snapshot requested: {} bytes", frame.len());
    let (&prefix, payload) = frame.split_first().ok_or(WireError::EmptyFrame)?;
    match prefix {
        SNAPSHOT_PREFIX => Snapshot::decode(payload).map_err(WireError::SnapshotDecode),
        OP_PREFIX => Err(WireError::NotASnapshot),
        b => Err(WireError::UnknownPrefix(b)),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod protocol_drift_tests;
