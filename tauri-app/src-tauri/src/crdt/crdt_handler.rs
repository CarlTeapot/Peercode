use log::{debug, error};
use tauri::State;

use crate::state::appstate::AppState;
use std::sync::atomic::Ordering;
#[tauri::command]
pub fn insert(state: State<AppState>, position: u64, content: String) -> Result<(), String> {
    debug!(
        "crdt insert request: position={}, content_len={}",
        position,
        content.chars().count()
    );
    let mut document = state.document.lock().map_err(|_| {
        error!("crdt insert failed: could not lock document state");
        "failed to lock document state".to_string()
    })?;

    // TODO(T10): forward the returned `Option<WireBlock>` to the ws writer
    // as an encoded `OpMessage::Insert` frame.
    document
        .local_insert(position, &content)
        .map(|_wire_block| {
            debug!(
                "crdt insert succeeded: position={}, content_len={}",
                position,
                content.chars().count()
            );
        })
        .map_err(|err| {
            error!(
                "crdt insert failed: position={}, content_len={}, error={:?}",
                position,
                content.chars().count(),
                err
            );
            format!("insert failed: {err:?}")
        })
}

#[tauri::command]
pub fn delete(state: State<AppState>, position: u64, length: u64) -> Result<(), String> {
    debug!(
        "crdt delete request: position={}, length={}",
        position, length
    );
    let mut document = state.document.lock().map_err(|_| {
        error!("crdt delete failed: could not lock document state");
        "failed to lock document state".to_string()
    })?;

    // TODO(T10): forward the returned `DeleteSet` diff to the ws writer
    // as an encoded `OpMessage::Delete` frame.
    document
        .delete(position, length)
        .map(|_delete_set_diff| {
            debug!(
                "crdt delete succeeded: position={}, length={}",
                position, length
            );
        })
        .map_err(|err| {
            error!(
                "crdt delete failed: position={}, length={}, error={:?}",
                position, length, err
            );
            format!("delete failed: {err:?}")
        })
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
