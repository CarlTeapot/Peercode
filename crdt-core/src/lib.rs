pub mod document;
pub mod error;
pub mod index;
pub mod snapshot;
pub mod store;
pub mod structs;
pub mod types;
pub mod wire;

pub use document::{Document, RemoteChange};
pub use error::DocumentError;
pub use index::{FindResult, PositionIndex};
pub use snapshot::{Snapshot, SnapshotBlock, SnapshotError};
pub use wire::{
    OP_PREFIX, OpMessage, SNAPSHOT_PREFIX, WireBlock, WireError, decode_op, decode_snapshot,
    encode_op, encode_snapshot,
};
