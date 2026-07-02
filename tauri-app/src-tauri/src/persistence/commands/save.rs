use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::persistence;
use crate::state::appstate::AppState;
use crate::state::document::{request, DocOp};

use super::{ensure_pcdoc_extension, name_from_path, set_current_file};

#[tauri::command]
pub async fn save_document(
    app: AppHandle,
    name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let snapshot = request(&state.doc_tx, |reply| DocOp::GetSnapshot { reply }).await?;
    persistence::save_snapshot_named(&app, &name, &snapshot).map_err(|e| e.to_string())?;
    let path = persistence::doc_path(&app, &name).map_err(|e| e.to_string())?;
    set_current_file(&state, Some(name), Some(path));
    Ok(())
}

#[tauri::command]
pub async fn save_document_to_path(
    app: AppHandle,
    path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let path = ensure_pcdoc_extension(PathBuf::from(path));
    let snapshot = request(&state.doc_tx, |reply| DocOp::GetSnapshot { reply }).await?;
    persistence::save_snapshot(&path, &snapshot).map_err(|e| e.to_string())?;
    persistence::record_recent(&app, &path).map_err(|e| e.to_string())?;
    set_current_file(&state, Some(name_from_path(&path)), Some(path));
    Ok(())
}

/// Re-save the current document wherever it lives (library or external).
#[tauri::command]
pub async fn save_current_document(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let path = state.current_document_path.lock().unwrap().clone();
    let name = state.current_document_name.lock().unwrap().clone();
    let snapshot = request(&state.doc_tx, |reply| DocOp::GetSnapshot { reply }).await?;
    match (path, name) {
        (Some(path), _) => persistence::save_snapshot(&path, &snapshot).map_err(|e| e.to_string()),
        (None, Some(name)) => {
            persistence::save_snapshot_named(&app, &name, &snapshot).map_err(|e| e.to_string())
        }
        (None, None) => Err("no current document; use Save as".to_string()),
    }
}
