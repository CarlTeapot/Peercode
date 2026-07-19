use crate::session::session_types::{CanWritePayload, SessionInfo, CAN_WRITE_CHANGED};
use crate::state::appstate::{AppRole, AppState};
use crate::state::ws_state::WsState;
use log::{debug, info, warn};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub fn get_session_info(state: State<'_, AppState>) -> SessionInfo {
    let role = state.current_role();
    let status = role.status().to_string();
    let (lan_url, public_url, local_room_url, public_room_url, room_id) = match role {
        AppRole::Host {
            room_id,
            lan_url,
            public_url,
            local_room_url,
            public_room_url,
        } => (
            lan_url,
            public_url,
            Some(local_room_url),
            public_room_url,
            Some(room_id),
        ),
        AppRole::Guest {
            room_id,
            server_url,
            ..
        } => (None, Some(server_url), None, None, Some(room_id)),
        _ => (None, None, None, None, None),
    };
    let info = SessionInfo {
        status,
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
pub fn leave_session(
    app: AppHandle,
    state: State<'_, AppState>,
    ws: State<'_, WsState>,
) -> Result<(), String> {
    info!("leave session requested");
    state.leave_session(&ws);
    match state.transition_role(AppRole::Undecided) {
        Ok(prev) => info!(
            "leave_session: role reset to idle from status={}",
            prev.status()
        ),
        Err(e) => warn!("leave_session: {e}"),
    }
    let _ = app.emit(CAN_WRITE_CHANGED, CanWritePayload { can_write: true });
    info!("leave session completed");
    Ok(())
}
