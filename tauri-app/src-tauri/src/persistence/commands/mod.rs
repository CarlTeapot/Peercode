mod document_ops;
mod open;
mod recent;
mod save;
#[cfg(test)]
mod tests;

use crate::state::appstate::{AppState, CurrentFile};
use crdt_core::Document;

pub use document_ops::*;
pub use open::*;
pub use recent::*;
pub use save::*;

fn set_current_file(state: &AppState, file: Option<CurrentFile>) {
    *state.current_file.lock().unwrap() = file;
}

fn name_from_path(path: &std::path::Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "untitled".to_string())
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
