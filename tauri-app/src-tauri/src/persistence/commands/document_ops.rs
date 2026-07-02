use crdt_core::types::ClientId;
use crdt_core::Document;
use rand::random;
use tauri::{AppHandle, State};

use crate::persistence;
use crate::state::appstate::AppState;
use crate::state::document::{request, DocOp};

use super::{set_current_file, set_export_path};

#[tauri::command]
pub async fn fork_document(
    app: AppHandle,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let original_snapshot = request(&state.doc_tx, |reply| DocOp::GetSnapshot { reply }).await?;
    let original_path = state.current_document_path.lock().unwrap().clone();

    if let Some(ref path) = original_path {
        persistence::save_snapshot(path, &original_snapshot).map_err(|e| e.to_string())?;
    }

    let mut fork_snapshot = original_snapshot;
    fork_snapshot.client_id = ClientId::new(random::<u64>());
    fork_snapshot.pending_blocks.clear();
    fork_snapshot.pending_delete_sets.clear();

    let forked = Document::from_snapshot(fork_snapshot);
    let text = forked.get_text();

    persistence::save_named(&app, &new_name, &forked).map_err(|e| e.to_string())?;

    request(&state.doc_tx, |reply| DocOp::DocumentReplace {
        doc: Box::new(forked),
        reply,
    })
    .await?;
    let fork_path = persistence::doc_path(&app, &new_name).map_err(|e| e.to_string())?;
    set_current_file(&state, Some(new_name), Some(fork_path));
    set_export_path(&state, None);

    Ok(text)
}

#[tauri::command]
pub async fn reset_document(state: State<'_, AppState>) -> Result<(), String> {
    let client_id = request(&state.doc_tx, |reply| DocOp::GetClientId { reply }).await?;
    let fresh = Document::new(client_id);
    request(&state.doc_tx, |reply| DocOp::DocumentReplace {
        doc: Box::new(fresh),
        reply,
    })
    .await?;
    set_current_file(&state, None, None);
    set_export_path(&state, None);
    Ok(())
}

#[tauri::command]
pub fn get_current_document_name(state: State<'_, AppState>) -> Result<Option<String>, String> {
    Ok(state.current_document_name.lock().unwrap().clone())
}

#[tauri::command]
pub fn get_current_document_path(state: State<'_, AppState>) -> Result<Option<String>, String> {
    Ok(state
        .current_document_path
        .lock()
        .unwrap()
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned()))
}
