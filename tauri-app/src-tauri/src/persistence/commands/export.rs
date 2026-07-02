use std::fs;
use std::path::PathBuf;

use crdt_core::Document;
use tauri::{AppHandle, State};

use crate::persistence;
use crate::state::appstate::AppState;
use crate::state::document::{request, DocOp};

use super::{name_from_path, set_current_file, set_export_path};

/// Opens any readable text file as a fresh CRDT document
#[tauri::command]
pub async fn import_text_file(
    app: AppHandle,
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let path = PathBuf::from(path);
    let text = persistence::read_text_for_import(&path).map_err(|e| e.to_string())?;

    let client_id = request(&state.doc_tx, |reply| DocOp::GetClientId { reply }).await?;
    let doc = Document::from_text_chunked(client_id, &text, persistence::IMPORT_CHUNK_CHARS)
        .map_err(|e| e.to_string())?;

    request(&state.doc_tx, |reply| DocOp::DocumentReplace {
        doc: Box::new(doc),
        reply,
    })
    .await?;

    persistence::record_recent(&app, &path).map_err(|e| e.to_string())?;
    set_current_file(&state, Some(name_from_path(&path)), None);
    set_export_path(&state, Some(path));
    Ok(text)
}

/// Writes the document's plain text to the exact path the user chose
#[tauri::command]
pub async fn export_document_to_path(
    app: AppHandle,
    path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let path = PathBuf::from(path);
    let text = request(&state.doc_tx, |reply| DocOp::GetText { reply }).await?;
    fs::write(&path, text).map_err(|e| e.to_string())?;
    persistence::record_recent(&app, &path).map_err(|e| e.to_string())?;
    set_export_path(&state, Some(path));
    Ok(())
}

/// Writes plain text back to the linked export file.
#[tauri::command]
pub async fn export_current_document(state: State<'_, AppState>) -> Result<String, String> {
    let path = state
        .current_export_path
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "no export target; use Export as…".to_string())?;
    let text = request(&state.doc_tx, |reply| DocOp::GetText { reply }).await?;
    fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn get_current_export_path(state: State<'_, AppState>) -> Result<Option<String>, String> {
    Ok(state
        .current_export_path
        .lock()
        .unwrap()
        .as_ref()
        .map(|p| p.to_string_lossy().into_owned()))
}
