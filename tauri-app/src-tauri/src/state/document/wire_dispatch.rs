use crdt_core::{decode_op, decode_snapshot, OpMessage, Snapshot, SNAPSHOT_PREFIX};
use log::{info, warn};
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::state::appstate::AppState;
use crate::state::document::client::DocSender;
use crate::state::document::types::DocOp;

pub async fn process_loop(mut rx: UnboundedReceiver<Vec<u8>>, app: AppHandle) {
    info!("op processor loop started");
    let doc_tx = app.state::<AppState>().doc_tx.clone();
    let mut snapshot_applied = false;
    let mut pending_ops = Vec::new();
    while let Some(bytes) = rx.recv().await {
        let outcome = dispatch_to_actor(
            &app,
            &doc_tx,
            &bytes,
            &mut snapshot_applied,
            &mut pending_ops,
        )
        .await;
        if outcome.is_err() {
            warn!("op processor: doc actor channel closed; stopping loop");
            break;
        }
    }
    info!("op processor loop stopped");
}

async fn dispatch_to_actor(
    app: &AppHandle,
    doc_tx: &DocSender,
    bytes: &[u8],
    snapshot_applied: &mut bool,
    pending_ops: &mut Vec<OpMessage>,
) -> Result<(), ()> {
    if is_snapshot_frame(bytes) {
        if let Some(snap) = decode_snapshot_frame(bytes) {
            doc_tx
                .send(DocOp::ApplyRemoteSnapshot { snap })
                .await
                .map_err(|_| ())?;
            *snapshot_applied = true;
            flush_pending_ops(doc_tx, pending_ops).await?;
        }
        return Ok(());
    }

    if let Some(op) = decode_op_frame(bytes) {
        if should_wait_for_snapshot(app, *snapshot_applied) {
            pending_ops.push(op);
            return Ok(());
        }
        return doc_tx
            .send(DocOp::ApplyRemoteOp { op })
            .await
            .map_err(|_| ());
    }
    Ok(())
}

async fn flush_pending_ops(doc_tx: &DocSender, pending_ops: &mut Vec<OpMessage>) -> Result<(), ()> {
    for op in pending_ops.drain(..) {
        doc_tx
            .send(DocOp::ApplyRemoteOp { op })
            .await
            .map_err(|_| ())?;
    }
    Ok(())
}

fn should_wait_for_snapshot(app: &AppHandle, snapshot_applied: bool) -> bool {
    !snapshot_applied && !app.state::<AppState>().is_host()
}

fn is_snapshot_frame(bytes: &[u8]) -> bool {
    bytes.first().copied() == Some(SNAPSHOT_PREFIX)
}

fn decode_snapshot_frame(bytes: &[u8]) -> Option<Snapshot> {
    match decode_snapshot(bytes) {
        Ok(s) => Some(s),
        Err(e) => {
            warn!("ws recv: snapshot decode failed: {e}");
            None
        }
    }
}

fn decode_op_frame(bytes: &[u8]) -> Option<OpMessage> {
    match decode_op(bytes) {
        Ok(op) => Some(op),
        Err(e) => {
            warn!("ws recv: op decode failed: {e}");
            None
        }
    }
}
