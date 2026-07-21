use crdt_core::wire::{encode_op, OpMessage};
use log::{debug, error};
#[cfg(debug_assertions)]
use std::sync::atomic::Ordering;
use tauri::State;

use crate::state::appstate::AppState;
use crate::state::document::client::request_fallible;
use crate::state::document::types::DocOp;
use crate::state::ws_state::WsState;

/// The CRDT must never contain `\r`: positions on the wire are offsets into
/// LF-only text, so a stray `\r\n` would shift every later position by one
/// per newline. The frontend already normalizes; this is the final guard.
fn normalize_to_lf(content: String) -> String {
    if content.contains('\r') {
        content.replace("\r\n", "\n").replace('\r', "\n")
    } else {
        content
    }
}

fn ensure_can_write(state: &AppState) -> Result<(), String> {
    if state.can_write() {
        Ok(())
    } else {
        Err("Write permission denied: the host has made you read-only".into())
    }
}

#[tauri::command]
pub async fn insert(
    state: State<'_, AppState>,
    ws: State<'_, WsState>,
    position: u64,
    content: String,
    base_seq: u64,
) -> Result<(), String> {
    ensure_can_write(&state)?;
    let content = normalize_to_lf(content);
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
        let _ = ws.send_raw(frame).await;
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
    ensure_can_write(&state)?;
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
        let _ = ws.send_raw(frame).await;
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
    ensure_can_write(&state)?;
    let content = normalize_to_lf(content);
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
        let _ = ws.send_raw(frame).await;
    }
    if let Some(wire_block) = wire_block_opt {
        let frame = encode_op(&OpMessage::Insert(wire_block));
        let _ = ws.send_raw(frame).await;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::normalize_to_lf;

    #[test]
    fn normalize_to_lf_converts_crlf_and_bare_cr() {
        assert_eq!(normalize_to_lf("a\r\nb\rc\nd".into()), "a\nb\nc\nd");
    }

    #[test]
    fn normalize_to_lf_leaves_lf_only_text_untouched() {
        assert_eq!(normalize_to_lf("a\nb\nc".into()), "a\nb\nc");
        assert_eq!(normalize_to_lf(String::new()), "");
    }
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
