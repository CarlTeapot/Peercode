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

fn rebuild_if_crlf(doc: Document) -> Result<(Document, String), String> {
    let text = doc.get_text();
    if !text.contains('\r') {
        return Ok((doc, text));
    }
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut fresh = Document::new(doc.client_id);
    if !normalized.is_empty() {
        fresh
            .local_insert(0, &normalized)
            .map_err(|e| format!("failed to normalize legacy document: {e}"))?;
    }
    Ok((fresh, normalized))
}

#[tauri::command]
pub async fn load_document(
    app: AppHandle,
    name: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let loaded = persistence::load_named(&app, &name).map_err(|e| e.to_string())?;
    let (doc, text) = rebuild_if_crlf(loaded)?;

    request(&state.doc_tx, |reply| DocOp::DocumentReplace {
        doc: Box::new(doc),
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
    let (forked, text) = rebuild_if_crlf(forked)?;

    persistence::save_named(&app, &new_name, &forked).map_err(|e| e.to_string())?;

    request(&state.doc_tx, |reply| DocOp::DocumentReplace {
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

#[cfg(test)]
mod tests {
    use super::rebuild_if_crlf;
    use crdt_core::types::ClientId;
    use crdt_core::Document;

    #[test]
    fn rebuild_if_crlf_keeps_lf_document_intact() {
        let mut doc = Document::new(ClientId::new(7));
        doc.local_insert(0, "line one\nline two").unwrap();
        let original_sv = doc.state_vector.clone();

        let (doc, text) = rebuild_if_crlf(doc).unwrap();

        assert_eq!(text, "line one\nline two");
        assert_eq!(doc.get_text(), "line one\nline two");
        assert_eq!(doc.state_vector, original_sv);
    }

    #[test]
    fn rebuild_if_crlf_normalizes_legacy_crlf_document() {
        let mut doc = Document::new(ClientId::new(7));
        doc.local_insert(0, "line one\r\nline two\rend").unwrap();

        let (doc, text) = rebuild_if_crlf(doc).unwrap();

        assert_eq!(text, "line one\nline two\nend");
        assert_eq!(doc.get_text(), "line one\nline two\nend");
        assert_eq!(doc.client_id, ClientId::new(7));
    }
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
    *state.current_document_name.lock().unwrap() = None;
    Ok(())
}
