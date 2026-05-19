use crdt_core::store::DeleteSet;
use crdt_core::types::ClientId;
use crdt_core::wire::WireBlock;
use crdt_core::{Document, OpMessage, Snapshot};
use tokio::sync::oneshot;

pub const REMOTE_CHANGE_EVENT: &str = "crdt://remote-change";
pub const DOC_RESET_EVENT: &str = "crdt://document-reset";
pub const SNAPSHOT_APPLIED_EVENT: &str = "crdt://snapshot-applied";

pub const DOC_CHANNEL_CAPACITY: usize = 256;

#[derive(Debug)]
pub enum DocOp {
    LocalInsert {
        position: u64,
        content: String,
        base_seq: u64,
        reply: oneshot::Sender<Result<Option<WireBlock>, String>>,
    },
    LocalDelete {
        position: u64,
        length: u64,
        base_seq: u64,
        reply: oneshot::Sender<Result<DeleteSet, String>>,
    },
    LocalReplace {
        position: u64,
        delete_length: u64,
        content: String,
        base_seq: u64,
        reply: oneshot::Sender<Result<(DeleteSet, Option<WireBlock>), String>>,
    },
    ApplyRemoteOp {
        op: OpMessage,
    },
    ApplyRemoteSnapshot {
        snap: Snapshot,
    },
    GetSnapshot {
        reply: oneshot::Sender<Snapshot>,
    },
    DocumentReplace {
        doc: Box<Document>,
        reply: oneshot::Sender<()>,
    },
    GetText {
        reply: oneshot::Sender<String>,
    },
    GetClientId {
        reply: oneshot::Sender<ClientId>,
    },
    #[cfg(debug_assertions)]
    DebugLinkedList {
        reply: oneshot::Sender<String>,
    },
}
