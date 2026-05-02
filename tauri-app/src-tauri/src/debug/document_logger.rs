use crate::state::appstate::AppState;
use log::info;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;
use tauri::Manager;

#[cfg(debug_assertions)]
pub fn spawn_linked_list_logger(app_handle: tauri::AppHandle) {
    thread::spawn::<_, ()>(move || loop {
        let state = app_handle.state::<AppState>();

        if state.crdt_logging_enabled.load(Ordering::Relaxed) {
            let text = {
                let document = state.document.lock().unwrap();
                document.debug_linked_list()
            };
            info!("CRDT linked list: {}", text);
        }
        thread::sleep(Duration::from_secs(1));
    });
}
