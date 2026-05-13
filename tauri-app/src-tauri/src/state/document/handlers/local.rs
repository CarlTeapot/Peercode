use crdt_core::store::DeleteSet;
use crdt_core::wire::WireBlock;
use crdt_core::DocumentError;
use log::{debug, warn};

use crate::state::document::state::DocState;

pub fn handle_local_insert(
    state: &mut DocState,
    position: u64,
    content: &str,
    base_seq: u64,
) -> Result<Option<WireBlock>, String> {
    let transformed = state.op_log.transform(position, base_seq);
    if transformed != position {
        debug!(
            "doc actor: local_insert transformed {} -> {} (base_seq={})",
            position, transformed, base_seq
        );
    }
    insert_with_clamp(state, transformed, content)
}

pub fn handle_local_delete(
    state: &mut DocState,
    position: u64,
    length: u64,
    base_seq: u64,
) -> Result<DeleteSet, String> {
    let transformed = state.op_log.transform(position, base_seq);
    if transformed != position {
        debug!(
            "doc actor: local_delete transformed {} -> {} (base_seq={})",
            position, transformed, base_seq
        );
    }
    delete_with_clamp(state, transformed, length)
}

fn insert_with_clamp(
    state: &mut DocState,
    position: u64,
    content: &str,
) -> Result<Option<WireBlock>, String> {
    match state.doc.local_insert(position, content) {
        Ok(wire) => Ok(wire),
        Err(DocumentError::OutOfBounds(_)) => clamped_retry_insert(state, position, content),
        Err(e) => Err(format!("{e:?}")),
    }
}

fn delete_with_clamp(
    state: &mut DocState,
    position: u64,
    length: u64,
) -> Result<DeleteSet, String> {
    match state.doc.delete(position, length) {
        Ok(ds) => Ok(ds),
        Err(DocumentError::OutOfBounds(_)) => clamped_retry_delete(state, position, length),
        Err(e) => Err(format!("{e:?}")),
    }
}

fn clamped_retry_insert(
    state: &mut DocState,
    position: u64,
    content: &str,
) -> Result<Option<WireBlock>, String> {
    let visible = state.visible_length();
    let clamped = position.min(visible);
    warn!(
        "doc actor: local_insert OOB at {} (visible_len={}); clamping to {}",
        position, visible, clamped
    );
    state
        .doc
        .local_insert(clamped, content)
        .map_err(|e| format!("{e:?}"))
}

fn clamped_retry_delete(
    state: &mut DocState,
    position: u64,
    length: u64,
) -> Result<DeleteSet, String> {
    let visible = state.visible_length();
    if visible == 0 {
        warn!(
            "doc actor: local_delete OOB with empty doc (pos={}, len={})",
            position, length
        );
        return Ok(DeleteSet::new());
    }
    let clamped_pos = position.min(visible.saturating_sub(1));
    let clamped_len = length.min(visible.saturating_sub(clamped_pos));
    warn!(
        "doc actor: local_delete OOB at {}+{} (visible_len={}); clamping to {}+{}",
        position, length, visible, clamped_pos, clamped_len
    );
    if clamped_len == 0 {
        return Ok(DeleteSet::new());
    }
    state
        .doc
        .delete(clamped_pos, clamped_len)
        .map_err(|e| format!("{e:?}"))
}
