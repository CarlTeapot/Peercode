use crdt_core::store::StateVector;
use log::{debug, error};
use tauri::AppHandle;

use crate::state::document::state::DocState;

pub fn handle_apply_gc_commit(
    state: &mut DocState,
    _app: &AppHandle,
    floor: StateVector,
) -> Result<(), String> {
    match state.doc.collect_garbage_below(&floor) {
        Ok(()) => {
            debug!("doc actor: gc-commit applied");
            Ok(())
        }
        Err(e) => {
            error!("doc actor: gc-commit failed: {e:?}");
            Err(format!("{e:?}"))
        }
    }
}
