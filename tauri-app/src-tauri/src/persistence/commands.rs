use std::fs;

use crate::persistence;
use crate::state::appstate::AppState;
use crate::state::document::{request, DocOp};
use crdt_core::types::ClientId;
use crdt_core::Document;
use rand::random;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn save_document(
    app: AppHandle,
    name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let snapshot = request(&state.doc_tx, |reply| DocOp::GetSnapshot { reply }).await?;
    persistence::save_snapshot_named(&app, &name, &snapshot).map_err(|e| e.to_string())?;
    *state.current_document_name.lock().unwrap() = Some(name);
    Ok(())
}

#[tauri::command]
pub async fn load_document(
    app: AppHandle,
    name: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let loaded = persistence::load_named(&app, &name).map_err(|e| e.to_string())?;
    let text = loaded.get_text();

    request(&state.doc_tx, |reply| DocOp::Replace {
        doc: Box::new(loaded),
        reply,
    })
    .await?;
    *state.current_document_name.lock().unwrap() = Some(name);

    Ok(text)
}

#[tauri::command]
pub fn list_saved_documents(app: AppHandle) -> Result<Vec<persistence::DocumentMeta>, String> {
    persistence::list_documents(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fork_document(
    app: AppHandle,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let original_snapshot = request(&state.doc_tx, |reply| DocOp::GetSnapshot { reply }).await?;
    let original_name = state.current_document_name.lock().unwrap().clone();

    if let Some(ref current_name) = original_name {
        persistence::save_snapshot_named(&app, current_name, &original_snapshot)
            .map_err(|e| e.to_string())?;
    }

    let mut fork_snapshot = original_snapshot;
    fork_snapshot.client_id = ClientId::new(random::<u64>());
    fork_snapshot.pending_blocks.clear();
    fork_snapshot.pending_delete_sets.clear();

    let forked = Document::from_snapshot(fork_snapshot);
    let text = forked.get_text();

    persistence::save_named(&app, &new_name, &forked).map_err(|e| e.to_string())?;

    request(&state.doc_tx, |reply| DocOp::Replace {
        doc: Box::new(forked),
        reply,
    })
    .await?;
    *state.current_document_name.lock().unwrap() = Some(new_name);

    Ok(text)
}

#[tauri::command]
pub fn delete_document(app: AppHandle, name: String) -> Result<(), String> {
    let dir = persistence::documents_dir(&app).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{name}.pcdoc"));
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn get_document_text(state: State<'_, AppState>) -> Result<String, String> {
    request(&state.doc_tx, |reply| DocOp::GetText { reply }).await
}

#[tauri::command]
pub fn get_current_document_name(state: State<'_, AppState>) -> Result<Option<String>, String> {
    Ok(state.current_document_name.lock().unwrap().clone())
}

#[tauri::command]
pub fn save_text_file(path: String, content: String) -> Result<(), String> {
    fs::write(&path, &content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reset_document(state: State<'_, AppState>) -> Result<(), String> {
    let client_id = request(&state.doc_tx, |reply| DocOp::GetClientId { reply }).await?;
    let fresh = Document::new(client_id);
    request(&state.doc_tx, |reply| DocOp::Replace {
        doc: Box::new(fresh),
        reply,
    })
    .await?;
    *state.current_document_name.lock().unwrap() = None;
    Ok(())
}
