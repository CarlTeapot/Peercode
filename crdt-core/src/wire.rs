use crate::snapshot::{Snapshot, SnapshotError};
use crate::store::{DeleteSet, StateVector};
use crate::structs::Block;
use crate::types::{BlockId, ClientId};
use log::trace;
use std::error::Error;
use std::fmt;

pub const OP_PREFIX: u8 = 0x00;
pub const SNAPSHOT_PREFIX: u8 = 0x01;
pub const PREFIX_CONTROL: u8 = 0x02;
pub const CONTROL_SESSION_ENDED: u8 = 0x01;
pub const CONTROL_SNAPSHOT_REQUEST: u8 = 0x02;

pub const PREFIX_GC_COMMIT: u8 = 0x04;
pub const PREFIX_MEMBERSHIP: u8 = 0x05;
pub const PREFIX_SV_REPORT: u8 = 0x06;

pub const PEER_JOINED: u8 = 0x01;
pub const PEER_LEFT: u8 = 0x02;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MembershipEvent {
    Joined,
    Left,
}

/// A membership change for a single client, carried by a `0x05` frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MembershipFrame {
    pub client_id: ClientId,
    pub event: MembershipEvent,
}

#[derive(Debug)]
pub enum WireError {
    EmptyFrame,
    UnknownPrefix(u8),
    NotAnOp,
    NotASnapshot,
    NotAGcCommit,
    NotAMember,
    NotAnSvReport,
    MalformedMembership,
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
            WireError::NotAGcCommit => {
                write!(f, "wire frame is not a gc-commit")
            }
            WireError::NotAMember => {
                write!(f, "wire frame is not a leave/join frame")
            }
            WireError::NotAnSvReport => {
                write!(f, "wire frame is not a state-vector report")
            }
            WireError::MalformedMembership => {
                write!(f, "membership frame has an invalid layout")
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

#[derive(Debug, Clone, PartialEq, bitcode::Encode, bitcode::Decode)]
pub struct GcCommit {
    pub floor: StateVector,
}

pub fn encode_gc_commit(floor: &StateVector) -> Vec<u8> {
    trace!("encode gc-commit requested");
    let payload = bitcode::encode(&GcCommit {
        floor: floor.clone(),
    });
    let mut frame = Vec::with_capacity(1 + payload.len());
    frame.push(PREFIX_GC_COMMIT);
    frame.extend_from_slice(&payload);
    frame
}

pub fn decode_gc_commit(frame: &[u8]) -> Result<StateVector, WireError> {
    trace!("decode gc-commit requested: {} bytes", frame.len());
    let (&prefix, payload) = frame.split_first().ok_or(WireError::EmptyFrame)?;
    match prefix {
        PREFIX_GC_COMMIT => bitcode::decode::<GcCommit>(payload)
            .map(|frame| frame.floor)
            .map_err(WireError::Decode),
        b if b == OP_PREFIX || b == SNAPSHOT_PREFIX => Err(WireError::NotAGcCommit),
        b => Err(WireError::UnknownPrefix(b)),
    }
}

pub fn encode_sv_report(sender: ClientId, entries: &[(ClientId, u64)]) -> Vec<u8> {
    trace!("encode sv-report requested: {} entries", entries.len());
    let payload = bitcode::encode(&(sender, entries.to_vec()));
    let mut frame = Vec::with_capacity(1 + payload.len());
    frame.push(PREFIX_SV_REPORT);
    frame.extend_from_slice(&payload);
    frame
}

#[allow(clippy::type_complexity)]
pub fn decode_sv_report(frame: &[u8]) -> Result<(ClientId, Vec<(ClientId, u64)>), WireError> {
    trace!("decode sv-report requested: {} bytes", frame.len());
    let (&prefix, payload) = frame.split_first().ok_or(WireError::EmptyFrame)?;
    match prefix {
        PREFIX_SV_REPORT => bitcode::decode(payload).map_err(WireError::Decode),
        b if b == OP_PREFIX || b == SNAPSHOT_PREFIX => Err(WireError::NotAnSvReport),
        b => Err(WireError::UnknownPrefix(b)),
    }
}

pub fn encode_membership(frame: &MembershipFrame) -> Vec<u8> {
    let event = match frame.event {
        MembershipEvent::Joined => PEER_JOINED,
        MembershipEvent::Left => PEER_LEFT,
    };
    let mut out = Vec::with_capacity(10);
    out.push(PREFIX_MEMBERSHIP);
    out.push(event);
    out.extend_from_slice(&frame.client_id.value.to_be_bytes());
    out
}

pub fn decode_membership(frame: &[u8]) -> Result<MembershipFrame, WireError> {
    let (&prefix, payload) = frame.split_first().ok_or(WireError::EmptyFrame)?;
    if prefix != PREFIX_MEMBERSHIP {
        return Err(WireError::NotAMember);
    }
    if payload.len() != 9 {
        return Err(WireError::MalformedMembership);
    }
    let event = match payload[0] {
        PEER_JOINED => MembershipEvent::Joined,
        PEER_LEFT => MembershipEvent::Left,
        _ => return Err(WireError::MalformedMembership),
    };
    let mut client_bytes = [0u8; 8];
    client_bytes.copy_from_slice(&payload[1..9]);
    Ok(MembershipFrame {
        client_id: ClientId::new(u64::from_be_bytes(client_bytes)),
        event,
    })
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod protocol_drift_tests;
