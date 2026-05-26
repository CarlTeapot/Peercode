use crate::gateway::gateway_api::{create_room, destroy_room};
use crate::processes::process_coordinator;
use crate::session::session_types::{HostSessionSetup, SessionReadyPayload, SESSION_READY};
use crate::state::appstate::{AppRole, AppState};
use crate::state::document::{request, DocOp};
use crate::state::ws_state::WsState;
use crate::ws_management::disconnect_handler::spawn_disconnect_handler;
use crate::ws_management::ws_types::DisconnectReason;
use log::{debug, error, info, warn};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::oneshot;

#[tauri::command]
pub async fn host_session(app: AppHandle) -> Result<(), String> {
    debug!("start_host_session requested");

    let guard = app
        .state::<AppState>()
        .begin_session(app.clone())
        .map_err(|e| {
            warn!("start_host_session rejected: {e}");
            e
        })?;
    debug!("start_host_session role set to Starting");

    let setup = prepare_host_session(&app).await?;

    let disconnect_rx = connect(&app, setup.port, setup.room_id.clone()).await?;

    app.state::<AppState>().complete_host(
        guard,
        setup.room_id.clone(),
        setup.lan_url.clone(),
        setup.public_url.clone(),
        setup.local_room_url.clone(),
        setup.public_room_url.clone(),
    )?;
    emit_session_ready(&app, &setup)?;
    info!(
        "start_host_session completed: room_id={} port={}",
        setup.room_id, setup.port
    );
    spawn_disconnect_handler(app, disconnect_rx);
    Ok(())
}

#[tauri::command]
pub async fn end_session(state: State<'_, AppState>, ws: State<'_, WsState>) -> Result<(), String> {
    info!("end_session requested");
    let (room_id, local_room_url) = match state.current_role() {
        AppRole::Host {
            room_id,
            local_room_url,
            ..
        } => (room_id, local_room_url),
        _ => {
            warn!("end_session rejected: not a host");
            return Err("Not currently hosting a session".into());
        }
    };
    let gateway_auth_token = state
        .gateway_auth_token()
        .ok_or_else(|| "Gateway auth token missing for running gateway process".to_string())?;

    destroy_room(local_room_url, &gateway_auth_token).await?;
    state.leave_session(&ws);

    match state.transition_role(AppRole::Undecided) {
        Ok(prev) => info!("role reset to idle from status={}", prev.status()),
        Err(e) => warn!("end_session: role already reset by disconnect handler ({e})"),
    }

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

async fn prepare_host_session(app: &AppHandle) -> Result<HostSessionSetup, String> {
    let wf = match app.state::<AppState>().combined_workflow_result() {
        Some(wf) => {
            info!("reusing existing gateway/tunnel: port={}", wf.port);
            wf
        }
        None => {
            info!("launching gateway/tunnel workflow");
            let wf = process_coordinator::launch(app.clone()).await?;
            {
                let state = app.state::<AppState>();
                let mut procs = state.processes.lock().unwrap();
                procs.gateway_port = Some(wf.port);
                procs.gateway_lan_url = wf.lan_url.clone();
            }
            wf
        }
    };

    let room_id = create_room(wf.port, &wf.gateway_auth_token).await?;
    let local_room_url = format!("ws://127.0.0.1:{}/ws?room={room_id}", wf.port);
    let public_room_url = wf
        .public_url
        .as_ref()
        .map(|u| format!("{u}?room={room_id}"));

    Ok(HostSessionSetup {
        room_id,
        port: wf.port,
        lan_url: wf.lan_url,
        public_url: wf.public_url,
        local_room_url,
        public_room_url,
    })
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

async fn connect(
    app: &AppHandle,
    port: u16,
    room_id: String,
) -> Result<oneshot::Receiver<DisconnectReason>, String> {
    debug!(
        "host local connect requested: room_id={} port={}",
        room_id, port
    );
    let host_client_id = read_client_id(app).await.map_err(|e| {
        error!("host connect: failed to read client_id from doc actor: {e}");
        e
    })?;

    let local_ws_url =
        format!("ws://127.0.0.1:{port}/ws?room={room_id}&client_id={host_client_id}");
    let ws = app.state::<WsState>();
    let disconnect_rx = ws
        .connect(&local_ws_url, room_id.clone(), app.clone())
        .await
        .map_err(|e| {
            error!("local websocket connection failed: {e}");
            e.to_string()
        })?;
    info!(
        "local websocket connection established for host session: room_id={}",
        room_id
    );
    Ok(disconnect_rx)
}

async fn read_client_id(app: &AppHandle) -> Result<u64, String> {
    let state = app.state::<AppState>();
    let id = request(&state.doc_tx, |reply| DocOp::GetClientId { reply }).await?;
    Ok(id.value)
}
