use tokio::sync::{mpsc, oneshot};

use crate::state::document::types::DocOp;

pub type DocSender = mpsc::Sender<DocOp>;

pub async fn request<T>(
    tx: &DocSender,
    build: impl FnOnce(oneshot::Sender<T>) -> DocOp,
) -> Result<T, String> {
    let (reply_tx, reply_rx) = oneshot::channel();
    let op = build(reply_tx);
    tx.send(op)
        .await
        .map_err(|_| "document actor channel closed".to_string())?;
    reply_rx
        .await
        .map_err(|_| "document actor dropped reply".to_string())
}

pub async fn request_fallible<T>(
    tx: &DocSender,
    build: impl FnOnce(oneshot::Sender<Result<T, String>>) -> DocOp,
) -> Result<T, String> {
    request(tx, build).await.and_then(|inner| inner)
}
