use std::sync::{Arc, RwLock};
use std::time::Duration;

use futures_util::StreamExt;
use log::{debug, info, warn};
use tokio::sync::{mpsc, Mutex};
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

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

    pub async fn connect(&self, url: &str, session_id: String) -> Result<(), WsError> {
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
        debug!("ws channel created: write_buffer_capacity=64");
        let sender = tokio::task::spawn(write_loop(sink, write_rx));
        let receiver = tokio::task::spawn(receive_loop(
            stream,
            Arc::clone(&self.connection),
            Arc::clone(&self.write_tx),
        ));
        debug!("ws sender/receiver tasks spawned");

        let mut guard = self.connection.lock().await;
        if !matches!(*guard, WsConnection::Connecting) {
            receiver.abort();
            sender.abort();
            warn!("ws connect cancelled before finalizing state");
            return Err(WsError::Cancelled);
        }
        *self.write_tx.write().unwrap() = Some(Arc::new(write_tx));
        *guard = WsConnection::Connected {
            session_id: session_id.clone(),
            receiver,
            sender,
        };

        info!("websocket connected: url={url} room={session_id}");
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<(), WsError> {
        debug!("ws disconnect request received");
        let mut guard = self.connection.lock().await;
        match &mut *guard {
            WsConnection::Connected {
                receiver, sender, ..
            } => {
                debug!("ws disconnect: aborting sender/receiver tasks");
                receiver.abort();
                sender.abort();
                *self.write_tx.write().unwrap() = None;
                *guard = WsConnection::Disconnected;
                info!("websocket disconnected");
                Ok(())
            }
            _ => {
                warn!("ws disconnect rejected: no active connection");
                Err(WsError::NotConnected)
            }
        }
    }
}
