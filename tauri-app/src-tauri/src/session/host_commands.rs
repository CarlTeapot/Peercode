use crate::gateway::gateway_api::{create_room, destroy_room};
use crate::processes::process_coordinator;
use crate::processes::types::SidecarStatus;
use crate::session::session_types::{SessionReadyPayload, SESSION_READY};
use crate::state::appstate::{AppRole, AppState};
use crate::state::ws_state::WsState;
use log::{debug, error, info, warn};
use tauri::{AppHandle, Emitter, Manager, State};

struct HostSessionSetup {
    room_id: String,
    port: u16,
    lan_url: Option<String>,
    public_url: Option<String>,
    local_room_url: String,
    public_room_url: Option<String>,
}

#[tauri::command]
pub async fn host_session(app: AppHandle) -> Result<(), String> {
    debug!("start_host_session requested");

    {
        let state = app.state::<AppState>();
        let mut role = state.role.lock().unwrap();
        if !matches!(*role, AppRole::Undecided) {
            warn!(
                "start_host_session rejected: expected idle role, got {}",
                role.status()
            );
            return Err("A session is already active".into());
        }
        *role = AppRole::Starting;
        debug!("start_host_session role set to Starting");
    }

    ensure_gateway_spawn_allowed(&app)?;

    let setup = match prepare_host_session(&app).await {
        Ok(result) => result,
        Err(e) => {
            rollback_starting_role(&app);
            return Err(e);
        }
    };

    transition_to_host(&app, &setup);
    emit_session_ready(&app, &setup)?;
    info!(
        "start_host_session workflow ready: room_id={} port={}",
        setup.room_id, setup.port
    );

    connect(app, setup.port, setup.room_id).await;
    info!("start_host_session completed");
    Ok(())
}

#[tauri::command]
pub async fn end_session(state: State<'_, AppState>, ws: State<'_, WsState>) -> Result<(), String> {
    info!("end_session requested");
    let (room_id, local_room_url) = {
        let role = state.role.lock().unwrap();
        match &*role {
            AppRole::Host {
                room_id,
                local_room_url,
                ..
            } => (room_id.clone(), local_room_url.clone()),
            _ => {
                warn!("end_session rejected: not a host");
                return Err("Not currently hosting a session".into());
            }
        }
    };
    state.leave_session(&ws);
    destroy_room(local_room_url).await?;
    info!("end_session completed: room_id={room_id}");
    Ok(())
}

#[tauri::command]
pub fn kill_host_processes(state: State<'_, AppState>) -> Result<(), String> {
    info!("kill_host_processes requested");
    state.kill_host_processes();
    info!("kill_host_processes completed");
    Ok(())
}

fn ensure_gateway_spawn_allowed(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let procs = state.processes.lock().unwrap();
    let should_launch_processes = procs
        .gateway
        .as_ref()
        .map(|s| s.status == SidecarStatus::Disabled)
        .unwrap_or(true);
    if !should_launch_processes {
        rollback_starting_role(app);
        return Err("Gateway sidecar is already running; refusing to launch a duplicate".into());
    }
    Ok(())
}

async fn prepare_host_session(app: &AppHandle) -> Result<HostSessionSetup, String> {
    info!("start_host_session launching gateway/tunnel workflow");
    let workflow = process_coordinator::launch(app.clone()).await?;
    let room_id = create_room(workflow.port).await?;
    let local_room_url = format!("ws://127.0.0.1:{}/ws?room={}", workflow.port, room_id);
    let public_url = workflow.public_url;
    let public_room_url = public_url
        .as_ref()
        .map(|url| format!("{url}?room={room_id}"));

    Ok(HostSessionSetup {
        room_id: room_id.clone(),
        port: workflow.port,
        lan_url: workflow.lan_url,
        public_url,
        local_room_url,
        public_room_url,
    })
}

fn transition_to_host(app: &AppHandle, setup: &HostSessionSetup) {
    let state = app.state::<AppState>();
    let mut role = state.role.lock().unwrap();
    *role = AppRole::Host {
        room_id: setup.room_id.clone(),
        lan_url: setup.lan_url.clone(),
        public_url: setup.public_url.clone(),
        local_room_url: setup.local_room_url.clone(),
        public_room_url: setup.public_room_url.clone(),
    };
}

fn emit_session_ready(app: &AppHandle, setup: &HostSessionSetup) -> Result<(), String> {
    let payload = SessionReadyPayload {
        lan_url: setup.lan_url.clone(),
        public_url: setup.public_url.clone(),
        local_room_url: setup.local_room_url.clone(),
        public_room_url: setup.public_room_url.clone(),
        room_id: setup.room_id.clone(),
        port: setup.port,
    };
    app.emit(SESSION_READY, payload)
        .map_err(|e| format!("failed to emit session ready event: {e}"))
}

fn rollback_starting_role(app: &AppHandle) {
    let state = app.state::<AppState>();
    let mut role = state.role.lock().unwrap();
    if matches!(*role, AppRole::Starting) {
        *role = AppRole::Undecided;
        warn!("start_host_session failed; role rolled back to idle");
    }
}

async fn connect(app: AppHandle, port: u16, room_id: String) {
    debug!(
        "host local connect requested: room_id={} port={}",
        room_id, port
    );
    let host_client_id = {
        let state = app.state::<AppState>();
        let doc = state.document.lock().unwrap();
        doc.client_id.value
    };

    let local_ws_url =
        format!("ws://127.0.0.1:{port}/ws?room={room_id}&client_id={host_client_id}");
    let ws = app.state::<WsState>();
    if let Err(e) = ws
        .connect(&local_ws_url, room_id.clone(), app.clone())
        .await
    {
        error!("local websocket connection failed (session still running): {e}");
    } else {
        info!(
            "local websocket connection established for host session: room_id={}",
            room_id
        );
    }
}
