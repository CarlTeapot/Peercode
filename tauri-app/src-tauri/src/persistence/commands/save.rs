use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::persistence::{self, FILE_EXTENSION};
use crate::state::appstate::{AppState, CurrentFile};
use crate::state::document::{request, DocOp};

use super::set_current_file;

/// Saves to the current file in its own format
#[tauri::command]
pub async fn save_file(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let current = state
        .current_file
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "no current file; use Save as…".to_string())?;
    write_to(&app, &state, &current).await
}

/// Saves to a new path chosen in the native dialog and makes it current.
#[tauri::command]
pub async fn save_file_as(
    app: AppHandle,
    path: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let had_crlf = state
        .current_file
        .lock()
        .unwrap()
        .as_ref()
        .is_some_and(|f| f.had_crlf);
    let file = CurrentFile {
        path: PathBuf::from(path),
        had_crlf,
    };
    write_to(&app, &state, &file).await?;
    set_current_file(&state, Some(file));
    Ok(())
}

async fn write_to(
    app: &AppHandle,
    state: &State<'_, AppState>,
    file: &CurrentFile,
) -> Result<(), String> {
    let is_pcdoc = file
        .path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(FILE_EXTENSION));
    if is_pcdoc {
        let snapshot = request(&state.doc_tx, |reply| DocOp::GetSnapshot { reply }).await?;
        persistence::save_snapshot(&file.path, &snapshot).map_err(|e| e.to_string())?;
    } else {
        let text = request(&state.doc_tx, |reply| DocOp::GetText { reply }).await?;
        persistence::write_text_file(&file.path, &text, file.had_crlf)
            .map_err(|e| e.to_string())?;
    }
    persistence::record_recent(app, &file.path).map_err(|e| e.to_string())
}
