use crate::session::session_types::{
    SessionDisconnectedPayload, SessionEndedPayload, SESSION_DISCONNECTED, SESSION_ENDED,
};
use crate::state::appstate::{AppRole, AppState};
use crate::ws_management::ws_types::DisconnectReason;
use log::{debug, info, warn};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::oneshot;

pub fn spawn_disconnect_handler(app: AppHandle, rx: oneshot::Receiver<DisconnectReason>) {
    tauri::async_runtime::spawn(async move {
        let Ok(reason) = rx.await else {
            debug!("disconnect handler: sender dropped (normal shutdown), exiting");
            return;
        };
        info!("disconnect handler: received reason={reason:?}");
        let state = app.state::<AppState>();
        state.sync_maintenance.stop_all();
        state.roster.clear();
        if matches!(reason, DisconnectReason::ConnectionLost)
            && matches!(state.current_role(), AppRole::Host { .. })
        {
            info!("disconnect handler: connection lost, killing host processes");
            state.kill_host_processes();
        }
        if state.transition_role(AppRole::Undecided).is_err() {
            warn!("disconnect handler: role transition to Undecided failed, aborting");
            return;
        }
        match reason {
            DisconnectReason::SessionEnded => {
                info!("disconnect handler: host ended session; notifying frontend");
                let _ = app.emit(SESSION_ENDED, SessionEndedPayload {});
            }
            DisconnectReason::ConnectionLost => {
                warn!("disconnect handler: connection lost; notifying frontend");
                let _ = app.emit(SESSION_DISCONNECTED, SessionDisconnectedPayload {});
            }
        }
    });
}
