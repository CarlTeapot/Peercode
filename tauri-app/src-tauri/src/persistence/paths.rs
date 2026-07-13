use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use super::PersistError;

/// Default location offered by the save/open dialogs.
pub fn documents_dir(app: &AppHandle) -> Result<PathBuf, PersistError> {
    match app.path().document_dir() {
        Ok(docs) => Ok(docs.join("PeerCode")),
        Err(_) => fallback_documents_dir(app),
    }
}

fn fallback_documents_dir(app: &AppHandle) -> Result<PathBuf, PersistError> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| PersistError::Io(std::io::Error::other(e.to_string())))?;
    Ok(base.join("documents"))
}
