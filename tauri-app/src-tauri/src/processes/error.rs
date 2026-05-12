use crate::session::session_types::{SessionErrorPayload, SESSION_ERROR};
use crate::state::appstate::AppState;
use crate::state::ws_state::WsState;
use tauri::{AppHandle, Emitter, Manager};

pub fn emit_error(app: &AppHandle, message: String) {
    let state = app.state::<AppState>();
    state.leave_session(&app.state::<WsState>());
    state.kill_host_processes();
    let _ = app.emit(SESSION_ERROR, SessionErrorPayload { message });
}
