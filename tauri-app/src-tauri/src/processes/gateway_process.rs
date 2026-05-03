use crate::app_config::config::AppConfig;
use crate::processes::types::GatewayWorkflowResult;
use crate::state::appstate::{AppRole, AppState};
use log::{debug, info, warn};
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;
use tokio::sync::mpsc::Receiver;

#[derive(serde::Deserialize)]
struct RoomResponse {
    room_id: String,
}

pub async fn run_gateway(app: &AppHandle) -> Result<Option<GatewayWorkflowResult>, String> {
    info!("gateway startup requested");
    let gateway_log_level = app.state::<AppConfig>().logging.gateway_level_for_env();
    debug!("gateway startup using log level: {}", gateway_log_level);
    let (mut rx, child) = app
        .shell()
        .sidecar("peercode-gateway")
        .map_err(|e| format!("Gateway sidecar not found: {e}"))?
        .env("GATEWAY_LOG_LEVEL", gateway_log_level)
        .spawn()
        .map_err(|e| format!("Failed to spawn gateway: {e}"))?;

    {
        let state = app.state::<AppState>();
        let role = state.role.lock().unwrap();
        debug!("gateway start requested while role={}", role.status());
        if !matches!(*role, AppRole::Starting) {
            warn!("gateway startup cancelled: role changed before registration");
            let _ = child.kill();
            return Ok(None);
        }
        drop(role);
        state.processes.lock().unwrap().gateway = Some(child);
        debug!("gateway process handle stored in app state");
    }

    while let Some(event) = rx.recv().await {
        if let CommandEvent::Stdout(bytes) = event {
            let line = String::from_utf8_lossy(&bytes);
            debug!("gateway stdout line received while waiting for port");
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line.trim()) {
                if let Some(port) = json.get("port").and_then(|v| v.as_u64()).map(|v| v as u16) {
                    info!("gateway reported listening port={port}");
                    let room_id = fetch_room_id(port).await?;
                    let result = on_gateway_ready(app, port, &room_id, rx).await;
                    info!("gateway ready on port={port}, room_id={room_id}");
                    return Ok(result);
                }
            }
        }
    }

    let still_starting = matches!(
        *app.state::<AppState>().role.lock().unwrap(),
        AppRole::Starting
    );
    if still_starting {
        warn!("gateway exited before reporting port while session still starting");
        Err("Gateway exited before reporting its port".into())
    } else {
        debug!("gateway ended after role changed; treating as cancelled startup");
        Ok(None)
    }
}

async fn on_gateway_ready(
    app: &AppHandle,
    port: u16,
    room_id: &str,
    log_rx: Receiver<CommandEvent>,
) -> Option<GatewayWorkflowResult> {
    debug!("finalizing gateway readiness: port={port} room_id={room_id}");
    let lan_url = get_lan_url(port, room_id).await;

    {
        let state = app.state::<AppState>();
        let mut role = state.role.lock().unwrap();
        if !matches!(*role, AppRole::Starting) {
            warn!("gateway ready ignored: role changed before host transition");
            return None;
        }
        *role = AppRole::Host {
            room_id: room_id.to_string(),
            lan_url: lan_url.clone(),
            public_url: None,
        };
        info!("role transitioned to Host: room_id={room_id}");
    }

    Some(GatewayWorkflowResult {
        lan_url,
        port,
        room_id: room_id.to_string(),
        log_rx,
    })
}

async fn get_lan_url(port: u16, room_id: &str) -> Option<String> {
    debug!("resolving LAN websocket URL: port={port} room_id={room_id}");
    let room_id = room_id.to_string();
    let url = tauri::async_runtime::spawn_blocking(move || {
        local_ip_address::local_ip()
            .ok()
            .map(|ip| format!("ws://{}:{}/ws?room={}", ip, port, room_id))
    })
    .await
    .ok()
    .flatten();
    if url.is_some() {
        info!("LAN websocket URL resolved");
    } else {
        warn!("LAN websocket URL resolution failed; LAN URL will be absent");
    }
    url
}

const FETCH_ROOM_TIMEOUT: Duration = Duration::from_secs(5);

async fn fetch_room_id(port: u16) -> Result<String, String> {
    debug!("fetching gateway room id via /rooms: port={port}");
    tokio::time::timeout(FETCH_ROOM_TIMEOUT, fetch_room_id_inner(port))
        .await
        .map_err(|_| {
            format!(
                "gateway /rooms: timed out after {}s",
                FETCH_ROOM_TIMEOUT.as_secs()
            )
        })?
}

async fn fetch_room_id_inner(port: u16) -> Result<String, String> {
    reqwest::Client::new()
        .post(format!("http://127.0.0.1:{port}/rooms"))
        .send()
        .await
        .map_err(|e| format!("gateway /rooms: {e}"))?
        .error_for_status()
        .map_err(|e| format!("gateway /rooms: {e}"))?
        .json::<RoomResponse>()
        .await
        .map(|r| r.room_id)
        .map_err(|e| format!("gateway /rooms: {e}"))
}
