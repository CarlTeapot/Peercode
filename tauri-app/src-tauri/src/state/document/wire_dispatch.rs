use crdt_core::store::DeleteSet;
use crdt_core::{
    decode_gc_commit, decode_op, decode_snapshot, OpMessage, Snapshot, PREFIX_GC_COMMIT,
    SNAPSHOT_PREFIX,
};
use log::{info, warn};
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::oneshot;

use crate::state::appstate::AppState;
use crate::state::document::client::DocSender;
use crate::state::document::types::DocOp;

/// Buffered until the initial snapshot applies (guests only), in arrival order.
enum PendingFrame {
    Op(OpMessage),
    Gc(DeleteSet),
}

pub async fn process_loop(mut rx: UnboundedReceiver<Vec<u8>>, app: AppHandle) {
    info!("op processor loop started");
    let doc_tx = app.state::<AppState>().doc_tx.clone();
    let mut snapshot_applied = false;
    let mut pending: Vec<PendingFrame> = Vec::new();
    while let Some(bytes) = rx.recv().await {
        let outcome =
            dispatch_to_actor(&app, &doc_tx, &bytes, &mut snapshot_applied, &mut pending).await;
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
    pending: &mut Vec<PendingFrame>,
) -> Result<(), ()> {
    if is_snapshot_frame(bytes) {
        if let Some(snap) = decode_snapshot_frame(bytes) {
            doc_tx
                .send(DocOp::ApplyRemoteSnapshot { snap })
                .await
                .map_err(|_| ())?;
            *snapshot_applied = true;
            flush_pending(doc_tx, pending).await?;
        }
        return Ok(());
    }

    if is_gc_commit_frame(bytes) {
        let Some(confirmed) = decode_gc_commit_frame(bytes) else {
            return Ok(());
        };
        if should_wait_for_snapshot(app, *snapshot_applied) {
            pending.push(PendingFrame::Gc(confirmed));
            return Ok(());
        }
        return apply_gc_commit(doc_tx, confirmed).await;
    }

    if let Some(op) = decode_op_frame(bytes) {
        if should_wait_for_snapshot(app, *snapshot_applied) {
            pending.push(PendingFrame::Op(op));
            return Ok(());
        }
        return doc_tx
            .send(DocOp::ApplyRemoteOp { op })
            .await
            .map_err(|_| ());
    }
    Ok(())
}

async fn flush_pending(doc_tx: &DocSender, pending: &mut Vec<PendingFrame>) -> Result<(), ()> {
    for frame in pending.drain(..) {
        match frame {
            PendingFrame::Op(op) => doc_tx
                .send(DocOp::ApplyRemoteOp { op })
                .await
                .map_err(|_| ())?,
            PendingFrame::Gc(confirmed) => apply_gc_commit(doc_tx, confirmed).await?,
        }
    }
    Ok(())
}

/// Fire-and-forget; the reply is unused (the handler emits any UI changes itself).
async fn apply_gc_commit(doc_tx: &DocSender, confirmed: DeleteSet) -> Result<(), ()> {
    let (reply_tx, _reply_rx) = oneshot::channel();
    doc_tx
        .send(DocOp::ApplyGcCommit {
            confirmed,
            reply: reply_tx,
        })
        .await
        .map_err(|_| ())
}

fn should_wait_for_snapshot(app: &AppHandle, snapshot_applied: bool) -> bool {
    !snapshot_applied && !app.state::<AppState>().is_host()
}

fn is_snapshot_frame(bytes: &[u8]) -> bool {
    bytes.first().copied() == Some(SNAPSHOT_PREFIX)
}

fn is_gc_commit_frame(bytes: &[u8]) -> bool {
    bytes.first().copied() == Some(PREFIX_GC_COMMIT)
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

fn decode_gc_commit_frame(bytes: &[u8]) -> Option<DeleteSet> {
    match decode_gc_commit(bytes) {
        Ok(ds) => Some(ds),
        Err(e) => {
            warn!("ws recv: gc-commit decode failed: {e}");
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
