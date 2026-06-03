use crdt_core::store::DeleteSet;
use log::{debug, error};
use tauri::AppHandle;

use crate::state::document::handlers::remote::emit_changes;
use crate::state::document::state::DocState;

/// Apply a host gc-commit. Any returned changes are deletes this peer had not
/// yet seen; route them through the normal remote-change path.
pub fn handle_apply_gc_commit(
    state: &mut DocState,
    app: &AppHandle,
    confirmed: DeleteSet,
) -> Result<(), String> {
    match state.doc.collect_garbage(&confirmed) {
        Ok(changes) => {
            debug!(
                "doc actor: gc-commit applied, replayed_changes={}",
                changes.len()
            );
            emit_changes(state, app, changes);
            Ok(())
        }
        Err(e) => {
            error!("doc actor: gc-commit failed: {e:?}");
            Err(format!("{e:?}"))
        }
    }
}
