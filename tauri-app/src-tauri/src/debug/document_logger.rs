use crate::state::appstate::AppState;
use crate::state::document::{request, DocOp};
use log::{info, warn};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::Manager;

#[cfg(debug_assertions)]
pub fn spawn_linked_list_logger(app_handle: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            let state = app_handle.state::<AppState>();
            if !state.crdt_logging_enabled.load(Ordering::Relaxed) {
                continue;
            }
            match request(&state.doc_tx, |reply| DocOp::DebugLinkedList { reply }).await {
                Ok(text) => info!("CRDT linked list: {}", text),
                Err(e) => warn!("CRDT linked list dump failed: {e}"),
            }
        }
    });
}
