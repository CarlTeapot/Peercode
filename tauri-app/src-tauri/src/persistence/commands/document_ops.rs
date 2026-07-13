use crdt_core::types::ClientId;
use crdt_core::Document;
use rand::random;
use serde::Serialize;
use tauri::State;

use crate::state::appstate::AppState;
use crate::state::document::{request, DocOp};

use super::{name_from_path, rebuild_if_crlf, set_current_file};

#[derive(Serialize)]
pub struct CurrentFileInfo {
    pub name: String,
    pub path: String,
}

/// Duplicates the document under a new CRDT identity as an untitled buffer;
/// the first Save asks where to put it.
#[tauri::command]
pub async fn fork_document(state: State<'_, AppState>) -> Result<String, String> {
    let mut snapshot = request(&state.doc_tx, |reply| DocOp::GetSnapshot { reply }).await?;
    snapshot.client_id = ClientId::new(random::<u64>());
    snapshot.pending_blocks.clear();
    snapshot.pending_delete_sets.clear();

    let forked = Document::from_snapshot(snapshot);
    let (forked, text) = rebuild_if_crlf(forked)?;
    request(&state.doc_tx, |reply| DocOp::DocumentReplace {
        doc: Box::new(forked),
        reply,
    })
    .await?;
    set_current_file(&state, None);
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
    set_current_file(&state, None);
    Ok(())
}

#[tauri::command]
pub fn get_current_file(state: State<'_, AppState>) -> Result<Option<CurrentFileInfo>, String> {
    Ok(state
        .current_file
        .lock()
        .unwrap()
        .as_ref()
        .map(|f| CurrentFileInfo {
            name: name_from_path(&f.path),
            path: f.path.to_string_lossy().into_owned(),
        }))
}
