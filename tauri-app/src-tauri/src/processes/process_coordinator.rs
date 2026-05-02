use crate::app_config::config::AppConfig;
use crate::processes::error::emit_error;
use crate::processes::gateway_process::run_gateway;
use crate::processes::tunnel_process::run_cloudflared;
use crate::processes::types::CombinedWorkflowResult;
use crate::session::session_types::{SessionReadyPayload, SESSION_READY};
use log::{debug, error, info, warn};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_shell::process::CommandEvent;
use tokio::sync::mpsc::Receiver;

pub async fn launch(app: AppHandle) -> Result<CombinedWorkflowResult, String> {
    let logging = app.state::<AppConfig>().logging.clone();
    info!("process coordinator launch requested");
    debug!(
        "process coordinator log pipe config: gateway={}, cloudflared={}",
        logging.show_gateway_logs, logging.show_cloudflared_logs
    );

    let gateway = match run_gateway(&app).await {
        Ok(Some(r)) => {
            info!(
                "gateway started: room_id={} port={} lan_url_present={}",
                r.room_id,
                r.port,
                r.lan_url.is_some()
            );
            r
        }
        Ok(None) => {
            warn!("gateway did not start (no process handle returned)");
            return Err("Gateway did not start".into());
        }
        Err(msg) => {
            error!("gateway startup failed: {msg}");
            emit_error(&app, msg.clone());
            return Err(msg);
        }
    };

    if logging.show_gateway_logs {
        debug!("spawning gateway log pipe task");
        tauri::async_runtime::spawn(pipe_process_logs("gateway", gateway.log_rx));
    } else {
        debug!("gateway log piping disabled by config");
    }

    let lan_url = gateway.lan_url;
    let port = gateway.port;
    let resolved_room_id = gateway.room_id.clone();
    let mut public_url: Option<String> = None;

    match run_cloudflared(&app, port, &gateway.room_id).await {
        Ok(Some(tunnel)) => {
            info!("cloudflared started: public_url={}", tunnel.public_url);
            if logging.show_cloudflared_logs {
                debug!("spawning cloudflared log pipe task");
                tauri::async_runtime::spawn(pipe_process_logs("cloudflared", tunnel.log_rx));
            } else {
                debug!("cloudflared log piping disabled by config");
            }
            public_url = Some(tunnel.public_url);
        }
        Ok(None) => {
            info!("cloudflared not started; continuing with LAN-only session");
        }
        Err(msg) => {
            error!("cloudflared startup failed: {msg}");
            emit_error(&app, msg.clone());
            return Err(msg);
        }
    }

    match app.emit(
        SESSION_READY,
        SessionReadyPayload {
            lan_url,
            public_url,
            room_id: resolved_room_id.clone(),
            port,
        },
    ) {
        Ok(()) => info!(
            "session ready event emitted: room_id={} port={}",
            resolved_room_id, port
        ),
        Err(e) => warn!("failed to emit session ready event: {e}"),
    }

    info!(
        "process coordinator launch completed: room_id={} port={}",
        resolved_room_id, port
    );
    Ok(CombinedWorkflowResult {
        port,
        room_id: resolved_room_id,
    })
}

async fn pipe_process_logs(name: &str, mut rx: Receiver<CommandEvent>) {
    debug!("process log pipe started: name={name}");
    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(b) => {
                info!("[{name}] {}", String::from_utf8_lossy(&b).trim_end());
            }
            CommandEvent::Stderr(b) => {
                error!("[{name}] {}", String::from_utf8_lossy(&b).trim_end());
            }
            CommandEvent::Terminated(status) => {
                info!("[{name}] terminated: {status:?}");
                break;
            }
            _ => {
                debug!("process log pipe ignored non-output event: name={name}");
            }
        }
    }
    debug!("process log pipe stopped: name={name}");
}
