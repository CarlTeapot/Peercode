use tauri::AppHandle;

use crate::persistence::{self, DocumentMeta};

#[tauri::command]
pub fn list_recent_files(app: AppHandle) -> Result<Vec<DocumentMeta>, String> {
    Ok(persistence::list_recent_meta(&app))
}

#[tauri::command]
pub fn remove_recent_file(app: AppHandle, path: String) -> Result<(), String> {
    persistence::remove_recent(&app, std::path::Path::new(&path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_documents_dir(app: AppHandle) -> Result<String, String> {
    persistence::documents_dir(&app)
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}
