use futures_util::SinkExt;
use log::{debug, info, warn};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::ws_management::ws_types::Sink;

pub async fn write_loop(mut sink: Sink, mut rx: mpsc::Receiver<Message>) {
    info!("ws write loop started");
    while let Some(msg) = rx.recv().await {
        let kind = match &msg {
            Message::Text(_) => "text",
            Message::Binary(_) => "binary",
            Message::Ping(_) => "ping",
            Message::Pong(_) => "pong",
            Message::Close(_) => "close",
            Message::Frame(_) => "frame",
        };
        debug!("ws write sending message: kind={kind}");
        if let Err(e) = sink.send(msg).await {
            warn!("ws write send failed: {e}");
            break;
        }
    }
    debug!("ws write input channel closed; closing sink");
    if let Err(e) = sink.close().await {
        warn!("ws write sink close failed: {e}");
    } else {
        info!("ws write sink closed successfully");
    }
    info!("ws write loop stopped");
}
