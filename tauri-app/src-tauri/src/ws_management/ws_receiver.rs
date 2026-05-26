use crdt_core::encode_snapshot;
use crdt_core::wire::{CONTROL_SESSION_ENDED, CONTROL_SNAPSHOT_REQUEST, PREFIX_CONTROL};
use futures_util::StreamExt;
use log::{debug, error, info, warn};
use std::sync::{Arc, RwLock};
use tauri::{AppHandle, Manager};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::tungstenite::Message;

use crate::state::appstate::AppState;
use crate::state::document::client::request;
use crate::state::document::types::DocOp;
use crate::ws_management::ws_types::{DisconnectReason, Stream, WsConnection};

pub async fn receive_loop(
    mut stream: Stream,
    connection: Arc<Mutex<WsConnection>>,
    write_tx: Arc<RwLock<Option<Arc<mpsc::Sender<Message>>>>>,
    op_tx: mpsc::UnboundedSender<Vec<u8>>,
    disconnect_tx: oneshot::Sender<DisconnectReason>,
    app: AppHandle,
) {
    info!("ws receiver loop started");
    let mut reason = DisconnectReason::ConnectionLost;
    while let Some(result) = stream.next().await {
        match result {
            Ok(Message::Text(text)) => {
                debug!("ws recv text (len={}): {text}", text.len());
            }
            Ok(Message::Binary(bytes)) => {
                if bytes.first().copied() == Some(PREFIX_CONTROL) {
                    match bytes.get(1).copied() {
                        Some(CONTROL_SESSION_ENDED) => {
                            info!("ws recv: session ended by host");
                            reason = DisconnectReason::SessionEnded;
                            break;
                        }
                        Some(CONTROL_SNAPSHOT_REQUEST) => {
                            handle_snapshot_request(&app, &write_tx).await;
                        }
                        other => {
                            warn!("ws recv: unknown control frame type={:?}; ignoring", other);
                        }
                    }
                    continue;
                }
                debug!("ws receiver binary message (bytes={})", bytes.len());
                if op_tx.send(bytes.into()).is_err() {
                    warn!("ws receiver: op processor channel closed; dropping frame");
                }
            }
            Ok(Message::Ping(_)) => {
                debug!("ws receiver ping");
            }
            Ok(Message::Pong(_)) => {
                debug!("ws receiver pong");
            }
            Ok(Message::Frame(_)) => {
                debug!("ws receiver raw frame");
            }
            Ok(Message::Close(_)) => {
                info!("ws recv: server closed connection");
                break;
            }
            Err(e) => {
                warn!("ws recv error: {e}");
                break;
            }
        }
    }

    {
        let mut guard = connection.lock().await;
        if matches!(*guard, WsConnection::Connected { .. }) {
            *write_tx.write().unwrap() = None;
            *guard = WsConnection::Disconnected;
            match reason {
                DisconnectReason::SessionEnded => {
                    info!("ws recv: connection closed after session ended")
                }
                DisconnectReason::ConnectionLost => {
                    warn!("ws recv: connection lost; state reset to Disconnected")
                }
            }
        }
    }
    let _ = disconnect_tx.send(reason);
    info!("ws recv loop stopped");
}

async fn handle_snapshot_request(
    app: &AppHandle,
    write_tx: &Arc<RwLock<Option<Arc<mpsc::Sender<Message>>>>>,
) {
    let state = app.state::<AppState>();
    if !state.is_host() {
        debug!("ws recv: snapshot request ignored by non-host");
        return;
    }

    let snap = match request(&state.doc_tx, |reply| DocOp::GetSnapshot { reply }).await {
        Ok(snap) => snap,
        Err(e) => {
            error!("snapshot request: failed to read snapshot from doc actor: {e}");
            return;
        }
    };

    let tx = {
        let guard = write_tx.read().unwrap();
        guard.as_ref().map(Arc::clone)
    };
    match tx {
        Some(tx) => {
            if tx
                .send(Message::Binary(encode_snapshot(&snap).into()))
                .await
                .is_err()
            {
                warn!("snapshot request: writer channel closed; snapshot dropped");
            } else {
                info!("snapshot request: host sent snapshot response");
            }
        }
        None => warn!("snapshot request: no active writer; snapshot dropped"),
    }
}
