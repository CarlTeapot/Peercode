use crate::app_config::config::AppConfig;
use crate::processes::types::TunnelWorkflowResult;
use crate::state::appstate::{AppRole, AppState};
use log::{debug, info, warn};
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

pub async fn run_cloudflared(
    app: &AppHandle,
    port: u16,
    room_id: &str,
) -> Result<Option<TunnelWorkflowResult>, String> {
    info!("cloudflared startup requested: port={port} room_id={room_id}");
    let url_arg = format!("http://localhost:{port}");
    let tunnel_log_level = app.state::<AppConfig>().logging.tunnel_level_for_arg();
    debug!("cloudflared startup using log level: {}", tunnel_log_level);

    let (mut rx, child) = app
        .shell()
        .sidecar("cloudflared")
        .map_err(|e| format!("cloudflared sidecar not found: {e}"))?
        .args([
            "tunnel",
            "--url",
            &url_arg,
            "--no-autoupdate",
            "--loglevel",
            &tunnel_log_level,
            "--http2-origin=false",
        ])
        .spawn()
        .map_err(|e| format!("Failed to spawn cloudflared: {e}"))?;

    app.state::<AppState>().processes.lock().unwrap().tunnel = Some(child);
    debug!("cloudflared process handle stored in app state");

    while let Some(event) = rx.recv().await {
        if let CommandEvent::Stderr(bytes) = event {
            let line = String::from_utf8_lossy(&bytes);
            debug!("cloudflared stderr line received while waiting for tunnel URL");
            if let Some(raw_url) = extract_tunnel_url(&line) {
                let ws_url = if raw_url.starts_with("https://") {
                    raw_url.replacen("https://", "wss://", 1)
                } else {
                    raw_url.replacen("http://", "ws://", 1)
                };
                let public_url = format!("{}/ws?room={}", ws_url, room_id);
                info!("cloudflared tunnel URL discovered");
                store_public_url(app, &public_url);

                return Ok(Some(TunnelWorkflowResult {
                    public_url,
                    log_rx: rx,
                }));
            }
        }
    }

    let is_host = matches!(
        *app.state::<AppState>().role.lock().unwrap(),
        AppRole::Host { .. }
    );
    if is_host {
        warn!("cloudflared exited without producing a tunnel URL while host role is active");
        Err("cloudflared exited without producing a tunnel URL".into())
    } else {
        debug!("cloudflared ended after role changed; treating as cancelled startup");
        Ok(None)
    }
}

fn extract_tunnel_url(line: &str) -> Option<String> {
    let start = line.find("https://").or_else(|| line.find("http://"))?;
    let rest = &line[start..];
    let end = rest
        .find(|c: char| c.is_whitespace() || c == '|' || c == '+' || c == '"' || c == '\x1b')
        .unwrap_or(rest.len());
    let url = rest[..end].trim().to_string();
    url.contains("trycloudflare.com").then_some(url)
}

fn store_public_url(app: &AppHandle, url: &str) {
    let state = app.state::<AppState>();
    let mut role = state.role.lock().unwrap();
    if let AppRole::Host {
        public_url: ref mut stored,
        ..
    } = *role
    {
        *stored = Some(url.to_string());
        info!("host public URL stored from cloudflared");
    } else {
        warn!("cloudflared public URL ignored because role is not Host");
    }
}
