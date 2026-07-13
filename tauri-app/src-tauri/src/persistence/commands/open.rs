use std::path::PathBuf;

use crdt_core::Document;
use tauri::{AppHandle, State};

use crate::persistence::{self, FileContent};
use crate::state::appstate::{AppState, CurrentFile};
use crate::state::document::{request, DocOp};

use super::{rebuild_if_crlf, set_current_file};

/// Opens any readable file: .pcdoc natively, everything else as chunked text.
#[tauri::command]
pub async fn open_file(
    app: AppHandle,
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let path = PathBuf::from(path);
    let content = persistence::read_file(&path).map_err(|e| e.to_string())?;

    let (doc, had_crlf) = match content {
        FileContent::Pcdoc(doc) => {
            let (doc, _) = rebuild_if_crlf(*doc)?;
            (Box::new(doc), false)
        }
        FileContent::Text { text, had_crlf } => {
            let client_id = request(&state.doc_tx, |reply| DocOp::GetClientId { reply }).await?;
            let doc = Document::from_text_chunked(client_id, &text, persistence::OPEN_CHUNK_CHARS)
                .map_err(|e| e.to_string())?;
            (Box::new(doc), had_crlf)
        }
    };

    let text = doc.get_text();
    request(&state.doc_tx, |reply| DocOp::DocumentReplace { doc, reply }).await?;
    persistence::record_recent(&app, &path).map_err(|e| e.to_string())?;
    set_current_file(&state, Some(CurrentFile { path, had_crlf }));
    Ok(text)
}
