use crdt_core::RemoteChange;
use futures_util::stream::{SplitSink, SplitStream};
use serde::Serialize;
use std::fmt::Display;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub type Sink = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>;
pub type Stream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

pub enum WsConnection {
    Disconnected,
    Connecting,
    Connected {
        #[allow(dead_code)]
        session_id: String,
        receiver: JoinHandle<()>,
        sender: JoinHandle<()>,
        processor: JoinHandle<()>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RemoteChangeEvent {
    Insert {
        seq: u64,
        position: u64,
        content: String,
    },
    Delete {
        seq: u64,
        position: u64,
        length: u64,
    },
}

impl RemoteChangeEvent {
    pub fn from_change(seq: u64, change: RemoteChange) -> Self {
        match change {
            RemoteChange::Insert { position, content } => RemoteChangeEvent::Insert {
                seq,
                position,
                content,
            },
            RemoteChange::Delete { position, length } => RemoteChangeEvent::Delete {
                seq,
                position,
                length,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotAppliedEvent {
    pub text: String,
}

#[derive(Debug)]
pub enum WsError {
    AlreadyConnected,
    Timeout { url: String, secs: u64 },
    Handshake { url: String, cause: String },
    NotConnected,
    Cancelled,
}

impl Display for WsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WsError::AlreadyConnected => {
                write!(f, "WebSocket is already connected or connecting")
            }
            WsError::Timeout { url, secs } => {
                write!(f, "WebSocket connect to {url} timed out after {secs}s")
            }
            WsError::Handshake { url, cause } => {
                write!(f, "WebSocket connect to {url} failed: {cause}")
            }
            WsError::NotConnected => write!(f, "WebSocket is not connected"),
            WsError::Cancelled => write!(f, "WebSocket connection cancelled before handshake"),
        }
    }
}
