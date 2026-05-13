use crdt_core::structs::Block;
use crdt_core::{OpMessage, RemoteChange};
use log::{debug, error, warn};
use tauri::{AppHandle, Emitter};

use crate::state::document::op_log::PositionDelta;
use crate::state::document::state::DocState;
use crate::state::document::types::REMOTE_CHANGE_EVENT;
use crate::ws_management::ws_types::RemoteChangeEvent;

pub fn handle_remote_op(state: &mut DocState, app: &AppHandle, op: OpMessage) {
    let changes = match op {
        OpMessage::Insert(wire_block) => apply_insert(state, wire_block),
        OpMessage::Delete(delete_set) => apply_delete(state, delete_set),
    };
    let Some(changes) = changes else { return };
    emit_changes(state, app, changes);
}

fn apply_insert(
    state: &mut DocState,
    wire_block: crdt_core::wire::WireBlock,
) -> Option<Vec<RemoteChange>> {
    let block: Block = wire_block.into();
    match state.doc.remote_insert(block) {
        Ok(changes) => {
            debug!(
                "doc actor: remote_insert applied, changes={}",
                changes.len()
            );
            Some(changes)
        }
        Err(e) => {
            error!("doc actor: remote_insert failed: {e:?}");
            None
        }
    }
}

fn apply_delete(
    state: &mut DocState,
    delete_set: crdt_core::store::DeleteSet,
) -> Option<Vec<RemoteChange>> {
    match state.doc.apply_delete_set(&delete_set) {
        Ok(changes) => {
            debug!(
                "doc actor: apply_delete_set applied, changes={}",
                changes.len()
            );
            Some(changes)
        }
        Err(e) => {
            error!("doc actor: apply_delete_set failed: {e:?}");
            None
        }
    }
}

fn emit_changes(state: &mut DocState, app: &AppHandle, changes: Vec<RemoteChange>) {
    for change in changes {
        let seq = state.mint_seq();
        state.op_log.push(seq, position_delta_of(&change));
        let event = RemoteChangeEvent::from_change(seq, change);
        if let Err(e) = app.emit(REMOTE_CHANGE_EVENT, &event) {
            warn!("doc actor: failed to emit remote change event: {e}");
        }
    }
}

fn position_delta_of(change: &RemoteChange) -> PositionDelta {
    match change {
        RemoteChange::Insert { position, content } => PositionDelta::Insert {
            at: *position,
            len: content.chars().count() as u64,
        },
        RemoteChange::Delete { position, length } => PositionDelta::Delete {
            at: *position,
            len: *length,
        },
    }
}
