use std::sync::{Arc, RwLock};
use std::time::Duration;

use futures_util::StreamExt;
use log::{debug, info, warn};
use tauri::AppHandle;
use tokio::sync::{mpsc, Mutex};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::state::document::wire_dispatch::process_loop;
use crate::ws_management::ws_receiver::receive_loop;
use crate::ws_management::ws_types::{WsConnection, WsError};
use crate::ws_management::ws_writer::write_loop;

pub struct WsState {
    connection: Arc<Mutex<WsConnection>>,
    connect_timeout: Duration,
    write_tx: Arc<RwLock<Option<Arc<mpsc::Sender<Message>>>>>,
}

impl WsState {
    pub fn new(connect_timeout: Duration) -> Self {
        info!(
            "ws state initialized: connect_timeout_ms={}",
            connect_timeout.as_millis()
        );
        Self {
            connection: Arc::new(Mutex::new(WsConnection::Disconnected)),
            connect_timeout,
            write_tx: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn connect(
        &self,
        url: &str,
        session_id: String,
        app: AppHandle,
    ) -> Result<(), WsError> {
        debug!("starting ws connection request: url={url} session_id={session_id}");
        {
            let mut guard = self.connection.lock().await;
            if !matches!(*guard, WsConnection::Disconnected) {
                warn!("ws connect rejected: already connected or connecting");
                return Err(WsError::AlreadyConnected);
            }
            *guard = WsConnection::Connecting;
            debug!("ws connection state set to Connecting");
        }

        let outcome = timeout(self.connect_timeout, connect_async(url)).await;

        let (ws_stream, _response) = match outcome {
            Ok(Ok(conn)) => {
                debug!("ws handshake succeeded: url={url}");
                conn
            }
            Ok(Err(e)) => {
                *self.connection.lock().await = WsConnection::Disconnected;
                warn!("ws handshake failed: url={url} error={e}");
                return Err(WsError::Handshake {
                    url: url.to_string(),
                    cause: e.to_string(),
                });
            }
            Err(_) => {
                *self.connection.lock().await = WsConnection::Disconnected;
                warn!(
                    "ws connect timed out: url={url} timeout_secs={}",
                    self.connect_timeout.as_secs()
                );
                return Err(WsError::Timeout {
                    url: url.to_string(),
                    secs: self.connect_timeout.as_secs(),
                });
            }
        };

        let (sink, stream) = ws_stream.split();
        let (write_tx, write_rx) = mpsc::channel::<Message>(64);
        let (op_tx, op_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        debug!("ws channel created: write_buffer_capacity=64");
        let processor = tokio::task::spawn(process_loop(op_rx, app.clone()));
        let sender = tokio::task::spawn(write_loop(sink, write_rx));
        let receiver = tokio::task::spawn(receive_loop(
            stream,
            Arc::clone(&self.connection),
            Arc::clone(&self.write_tx),
            op_tx,
            app.clone(),
        ));
        debug!("ws sender/receiver/processor tasks spawned");

        let mut guard = self.connection.lock().await;
        if !matches!(*guard, WsConnection::Connecting) {
            receiver.abort();
            sender.abort();
            processor.abort();
            warn!("ws connect cancelled before finalizing state");
            return Err(WsError::Cancelled);
        }
        *self.write_tx.write().unwrap() = Some(Arc::new(write_tx));
        *guard = WsConnection::Connected {
            session_id: session_id.clone(),
            receiver,
            sender,
            processor,
        };

        info!("websocket connected: url={url} room={session_id}");
        Ok(())
    }

    pub async fn send_raw(&self, bytes: Vec<u8>) {
        let tx = {
            let guard = self.write_tx.read().unwrap();
            guard.as_ref().map(Arc::clone)
        };
        match tx {
            Some(tx) => {
                if tx.send(Message::Binary(bytes.into())).await.is_err() {
                    warn!("ws send_raw: writer channel closed; frame dropped");
                }
            }
            None => {
                warn!("ws send_raw: no active connection; frame dropped");
            }
        }
    }

    fn do_disconnect(
        guard: &mut WsConnection,
        write_tx: &RwLock<Option<Arc<mpsc::Sender<Message>>>>,
    ) -> bool {
        if let WsConnection::Connected {
            receiver,
            sender,
            processor,
            ..
        } = guard
        {
            receiver.abort();
            sender.abort();
            processor.abort();
            *write_tx.write().unwrap() = None;
            *guard = WsConnection::Disconnected;
            info!("websocket disconnected");
            true
        } else {
            false
        }
    }

    pub fn disconnect_nowait(&self) {
        let connection = Arc::clone(&self.connection);
        let write_tx = Arc::clone(&self.write_tx);
        tauri::async_runtime::spawn(async move {
            let mut guard = connection.lock().await;
            Self::do_disconnect(&mut guard, &write_tx);
        });
    }

    pub async fn disconnect(&self) -> Result<(), WsError> {
        debug!("ws disconnect request received");
        let mut guard = self.connection.lock().await;
        if Self::do_disconnect(&mut guard, &self.write_tx) {
            Ok(())
        } else {
            warn!("ws disconnect rejected: no active connection");
            Err(WsError::NotConnected)
        }
    }
}
