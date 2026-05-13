use crdt_core::{Document, Snapshot};
use log::{info, warn};
use tauri::{AppHandle, Emitter};

use crate::state::document::state::DocState;
use crate::state::document::types::{DOC_RESET_EVENT, SNAPSHOT_APPLIED_EVENT};
use crate::ws_management::ws_types::SnapshotAppliedEvent;

pub fn handle_remote_snapshot(state: &mut DocState, app: &AppHandle, snap: Snapshot) {
    let local_client_id = state.doc.client_id;
    let forked = Document::from_snapshot(snap).fork(local_client_id);
    let text = forked.get_text();
    state.doc = forked;
    state.reset_history();
    info!(
        "doc actor: remote snapshot applied, text_len={}",
        text.len()
    );
    emit_reset(app);
    emit_snapshot_applied(app, text);
}

pub fn handle_replace(state: &mut DocState, app: &AppHandle, doc: Document) {
    state.doc = doc;
    state.reset_history();
    emit_reset(app);
}

fn emit_reset(app: &AppHandle) {
    if let Err(e) = app.emit(DOC_RESET_EVENT, ()) {
        warn!("doc actor: failed to emit doc reset event: {e}");
    }
}

fn emit_snapshot_applied(app: &AppHandle, text: String) {
    if let Err(e) = app.emit(SNAPSHOT_APPLIED_EVENT, SnapshotAppliedEvent { text }) {
        warn!("doc actor: failed to emit snapshot applied event: {e}");
    }
}
