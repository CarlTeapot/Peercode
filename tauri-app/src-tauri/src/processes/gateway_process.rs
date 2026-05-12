use crate::app_config::config::AppConfig;
use crate::processes::types::{GatewayWorkflowResult, Sidecar, SidecarStatus};
use crate::state::appstate::{AppRole, AppState};
use log::{debug, info, warn};
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

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
        state.processes.lock().unwrap().gateway = Some(Sidecar {
            proc: child,
            name: "peercode-gateway".to_string(),
            status: SidecarStatus::Enabled,
        });
        debug!("gateway process handle stored in app state");
    }

    while let Some(event) = rx.recv().await {
        if let CommandEvent::Stdout(bytes) = event {
            let line = String::from_utf8_lossy(&bytes);
            debug!("gateway stdout line received while waiting for port");
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(line.trim()) {
                if let Some(port) = json.get("port").and_then(|v| v.as_u64()).map(|v| v as u16) {
                    info!("gateway ready on port={port}");

                    return Ok(Some(GatewayWorkflowResult {
                        lan_url: get_lan_url(port).await,
                        port,
                        log_rx: rx,
                    }));
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

async fn get_lan_url(port: u16) -> Option<String> {
    debug!("resolving LAN websocket URL: port={port}");
    let url = tauri::async_runtime::spawn_blocking(move || {
        local_ip_address::local_ip()
            .ok()
            .map(|ip| format!("ws://{}:{}/ws", ip, port))
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
