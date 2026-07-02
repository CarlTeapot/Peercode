use std::fs;
use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::persistence;
use crate::persistence::FILE_EXTENSION;
use crate::state::appstate::AppState;
use crate::state::document::{request, DocOp};

use super::{name_from_path, set_current_file, set_export_path};

#[tauri::command]
pub async fn load_document(
    app: AppHandle,
    name: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let loaded = persistence::load_named(&app, &name).map_err(|e| e.to_string())?;
    let text = loaded.get_text();

    request(&state.doc_tx, |reply| DocOp::DocumentReplace {
        doc: Box::new(loaded),
        reply,
    })
    .await?;
    let path = persistence::doc_path(&app, &name).map_err(|e| e.to_string())?;
    set_current_file(&state, Some(name), Some(path));
    set_export_path(&state, None);

    Ok(text)
}

#[tauri::command]
pub async fn load_document_from_path(
    app: AppHandle,
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let path = PathBuf::from(path);
    let loaded = persistence::load_document(&path).map_err(|e| e.to_string())?;
    let text = loaded.get_text();

    request(&state.doc_tx, |reply| DocOp::DocumentReplace {
        doc: Box::new(loaded),
        reply,
    })
    .await?;
    persistence::record_recent(&app, &path).map_err(|e| e.to_string())?;
    set_current_file(&state, Some(name_from_path(&path)), Some(path));
    set_export_path(&state, None);

    Ok(text)
}

#[tauri::command]
pub fn list_saved_documents(app: AppHandle) -> Result<Vec<persistence::DocumentMeta>, String> {
    persistence::list_documents(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_documents_dir(app: AppHandle) -> Result<String, String> {
    persistence::documents_dir(&app)
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_document(app: AppHandle, name: String) -> Result<(), String> {
    let dir = persistence::documents_dir(&app).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{name}.{FILE_EXTENSION}"));
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
