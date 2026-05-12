use crate::session::session_types::SessionInfo;
use crate::state::appstate::{AppRole, AppState};
use crate::state::ws_state::WsState;
use log::{debug, info};
use tauri::State;

#[tauri::command]
pub fn get_session_info(state: State<'_, AppState>) -> SessionInfo {
    let role = state.role.lock().unwrap();
    let (lan_url, public_url, local_room_url, public_room_url, room_id) = match &*role {
        AppRole::Host {
            room_id,
            lan_url,
            public_url,
            local_room_url,
            public_room_url,
            ..
        } => (
            lan_url.clone(),
            public_url.clone(),
            Some(local_room_url.clone()),
            public_room_url.clone(),
            Some(room_id.clone()),
        ),
        AppRole::Guest {
            room_id,
            server_url,
        } => (
            None,
            Some(server_url.clone()),
            None,
            None,
            Some(room_id.clone()),
        ),
        _ => (None, None, None, None, None),
    };
    let info = SessionInfo {
        status: role.status().into(),
        lan_url,
        public_url,
        local_room_url,
        public_room_url,
        room_id,
    };
    debug!("get_session_info returned status={}", info.status);
    info
}

#[tauri::command]
pub fn leave_session(state: State<'_, AppState>, ws: State<'_, WsState>) -> Result<(), String> {
    info!("leave session requested");
    state.leave_session(&ws);
    info!("leave session completed");
    Ok(())
}
