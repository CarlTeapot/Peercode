use crdt_core::encode_snapshot;
use crdt_core::wire::{encode_op, OpMessage};
use log::{debug, error, info};
use std::sync::atomic::Ordering;
use tauri::State;

use crate::state::appstate::{AppRole, AppState};
use crate::state::document::client::{request, request_fallible};
use crate::state::document::types::DocOp;
use crate::state::ws_state::WsState;

const SNAPSHOT_REFRESH_INTERVAL: u32 = 100;

#[tauri::command]
pub async fn insert(
    state: State<'_, AppState>,
    ws: State<'_, WsState>,
    position: u64,
    content: String,
    base_seq: u64,
) -> Result<(), String> {
    debug!(
        "crdt insert request: position={}, content_len={}, base_seq={}",
        position,
        content.chars().count(),
        base_seq
    );

    let content_len = content.chars().count();
    let wire_block_opt = request_fallible(&state.doc_tx, |reply| DocOp::LocalInsert {
        position,
        content,
        base_seq,
        reply,
    })
    .await
    .map_err(|err| {
        error!("crdt insert failed: position={position}, content_len={content_len}, error={err}");
        format!("insert failed: {err}")
    })?;

    if let Some(wire_block) = wire_block_opt {
        let frame = encode_op(&OpMessage::Insert(wire_block));
        ws.send_raw(frame).await;
        maybe_send_snapshot(&state, &ws).await;
    }

    Ok(())
}

#[tauri::command]
pub async fn delete(
    state: State<'_, AppState>,
    ws: State<'_, WsState>,
    position: u64,
    length: u64,
    base_seq: u64,
) -> Result<(), String> {
    debug!(
        "crdt delete request: position={}, length={}, base_seq={}",
        position, length, base_seq
    );

    let delete_set = request_fallible(&state.doc_tx, |reply| DocOp::LocalDelete {
        position,
        length,
        base_seq,
        reply,
    })
    .await
    .map_err(|err| {
        error!("crdt delete failed: position={position}, length={length}, error={err}");
        format!("delete failed: {err}")
    })?;

    if !delete_set.is_empty() {
        let frame = encode_op(&OpMessage::Delete(delete_set));
        ws.send_raw(frame).await;
        maybe_send_snapshot(&state, &ws).await;
    }

    Ok(())
}

#[tauri::command]
pub async fn replace(
    state: State<'_, AppState>,
    ws: State<'_, WsState>,
    position: u64,
    delete_length: u64,
    content: String,
    base_seq: u64,
) -> Result<(), String> {
    debug!(
        "crdt replace request: position={}, delete_length={}, content_len={}, base_seq={}",
        position,
        delete_length,
        content.chars().count(),
        base_seq
    );

    let (delete_set, wire_block_opt) =
        request_fallible(&state.doc_tx, |reply| DocOp::LocalReplace {
            position,
            delete_length,
            content,
            base_seq,
            reply,
        })
        .await
        .map_err(|err| {
            error!(
                "crdt replace failed: position={position}, delete_length={delete_length}, \
                 error={err}"
            );
            format!("replace failed: {err}")
        })?;

    if !delete_set.is_empty() {
        let frame = encode_op(&OpMessage::Delete(delete_set));
        ws.send_raw(frame).await;
        maybe_send_snapshot(&state, &ws).await;
    }
    if let Some(wire_block) = wire_block_opt {
        let frame = encode_op(&OpMessage::Insert(wire_block));
        ws.send_raw(frame).await;
        maybe_send_snapshot(&state, &ws).await;
    }

    Ok(())
}

async fn maybe_send_snapshot(state: &State<'_, AppState>, ws: &State<'_, WsState>) {
    if !is_host(state) {
        return;
    }
    let count = state.ops_since_snapshot.fetch_add(1, Ordering::Relaxed) + 1;
    if count < SNAPSHOT_REFRESH_INTERVAL {
        return;
    }
    state.ops_since_snapshot.store(0, Ordering::Relaxed);
    let snap = match request(&state.doc_tx, |reply| DocOp::GetSnapshot { reply }).await {
        Ok(s) => s,
        Err(e) => {
            error!("periodic snapshot: failed to read snapshot from doc actor: {e}");
            return;
        }
    };
    ws.send_raw(encode_snapshot(&snap)).await;
    info!("periodic snapshot sent (after {} ops)", count);
}

fn is_host(state: &State<'_, AppState>) -> bool {
    matches!(*state.role.lock().unwrap(), AppRole::Host { .. })
}

#[cfg(debug_assertions)]
#[tauri::command]
pub fn toggle_crdt_logging(state: tauri::State<AppState>) {
    let current = state.crdt_logging_enabled.load(Ordering::Relaxed);
    debug!("toggle_crdt_logging request: current={}", current);
    state
        .crdt_logging_enabled
        .store(!current, Ordering::Relaxed);
    debug!("toggle_crdt_logging succeeded: enabled={}", !current);
}
