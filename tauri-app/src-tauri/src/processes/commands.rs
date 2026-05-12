use crate::processes::types::{ProcessStatusResponse, SidecarStatus};
use crate::state::appstate::AppState;
use tauri::State;

#[tauri::command]
pub fn get_process_status(state: State<'_, AppState>) -> ProcessStatusResponse {
    let procs = state.processes.lock().unwrap();
    ProcessStatusResponse {
        gateway: procs
            .gateway
            .as_ref()
            .map(|s| s.status.clone())
            .unwrap_or(SidecarStatus::Disabled),
        tunnel: procs
            .tunnel
            .as_ref()
            .map(|s| s.status.clone())
            .unwrap_or(SidecarStatus::Disabled),
    }
}
