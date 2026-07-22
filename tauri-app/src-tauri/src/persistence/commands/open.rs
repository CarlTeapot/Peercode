use std::path::PathBuf;

use crdt_core::{encode_snapshot, Document};
use tauri::{AppHandle, State};

use crate::persistence::{self, FileContent};
use crate::state::appstate::{AppState, CurrentFile};
use crate::state::document::{request, DocOp};
use crate::state::ws_state::WsState;

use super::{deny_doc_swap, rebuild_if_crlf, set_current_file};

/// Opens any readable file: .pcdoc natively, everything else as chunked text.
/// Guests are rejected (leave the session first). A host opening mid-session
/// broadcasts the new document to the room as a snapshot frame.
#[tauri::command]
pub async fn open_file(
    app: AppHandle,
    path: String,
    state: State<'_, AppState>,
    ws: State<'_, WsState>,
) -> Result<String, String> {
    deny_doc_swap(&state.current_role(), "open a file", true)?;
    let is_host = state.is_host();

    let path = PathBuf::from(path);
    let content = persistence::read_file(&path).map_err(|e| e.to_string())?;

    let client_id = request(&state.doc_tx, |reply| DocOp::GetClientId { reply }).await?;
    let (doc, had_crlf) = match content {
        FileContent::Pcdoc(doc) => {
            let (doc, _) = rebuild_if_crlf(*doc)?;
            let doc = if is_host { doc.fork(client_id) } else { doc };
            (Box::new(doc), false)
        }
        FileContent::Text { text, had_crlf } => {
            let doc = Document::from_text_chunked(client_id, &text, persistence::OPEN_CHUNK_CHARS)
                .map_err(|e| e.to_string())?;
            (Box::new(doc), had_crlf)
        }
    };

    let text = doc.get_text();
    let snapshot = is_host.then(|| doc.to_snapshot());
    request(&state.doc_tx, |reply| DocOp::DocumentReplace { doc, reply }).await?;
    persistence::record_recent(&app, &path).map_err(|e| e.to_string())?;
    set_current_file(&state, Some(CurrentFile { path, had_crlf }));

    if let Some(snap) = snapshot {
        state.sync_maintenance.document_replaced().await;
        ws.send_raw(encode_snapshot(&snap)).await.map_err(|e| {
            format!(
                "file opened locally, but broadcasting it to peers failed ({e}); \
                 peers may be out of sync — consider re-hosting the session"
            )
        })?;
    }
    Ok(text)
}
