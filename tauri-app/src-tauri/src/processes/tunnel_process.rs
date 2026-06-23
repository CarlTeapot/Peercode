use crate::app_config::config::AppConfig;
use crate::processes::types::{Sidecar, SidecarStatus, TunnelWorkflowResult};
use crate::state::appstate::AppState;
use log::{debug, info, warn};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

pub async fn run_cloudflared(
    app: &AppHandle,
    port: u16,
) -> Result<Option<TunnelWorkflowResult>, String> {
    info!("cloudflared startup requested: port={port}");
    let url_arg = format!("http://localhost:{port}");
    let metrics_addr = find_free_metrics_addr()?;
    let metrics_arg = metrics_addr.to_string();
    let metrics_url = format!("http://{metrics_addr}/metrics");
    let tunnel_log_level = app.state::<AppConfig>().logging.tunnel_level_for_arg();
    debug!("cloudflared startup using log level: {}", tunnel_log_level);
    info!("cloudflared metrics endpoint reserved: {metrics_url}");

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
            "--metrics",
            &metrics_arg,
        ])
        .spawn()
        .map_err(|e| format!("Failed to spawn cloudflared: {e}"))?;

    {
        let state = app.state::<AppState>();
        let mut processes = state.processes.lock().unwrap();
        processes.tunnel = Some(Sidecar {
            proc: child,
            name: "cloudflared".to_string(),
            status: SidecarStatus::Enabled,
        });
    }
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
                let public_url = format!("{}/ws", ws_url);
                info!("cloudflared tunnel URL discovered");
                store_public_url(app, &public_url);

                return Ok(Some(TunnelWorkflowResult {
                    public_url,
                    metrics_url,
                    log_rx: rx,
                }));
            }
        }
    }

    let is_host = app.state::<AppState>().is_host();
    if is_host {
        warn!("cloudflared exited without producing a tunnel URL while host role is active");
        Err("cloudflared exited without producing a tunnel URL".into())
    } else {
        debug!("cloudflared ended after role changed; treating as cancelled startup");
        Ok(None)
    }
}

fn find_free_metrics_addr() -> Result<SocketAddrV4, String> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .map_err(|e| format!("Failed to find a free port for cloudflared metrics: {e}"))?;
    let address = listener
        .local_addr()
        .map_err(|e| format!("Failed to read cloudflared metrics address: {e}"))?;
    match address {
        std::net::SocketAddr::V4(address) => Ok(address),
        std::net::SocketAddr::V6(_) => {
            Err("Expected an IPv4 address for cloudflared metrics".to_string())
        }
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
    app.state::<AppState>().store_public_url(url.to_string());
}

#[cfg(test)]
mod tests {
    use super::find_free_metrics_addr;
    use std::net::TcpListener;

    #[test]
    fn finds_available_loopback_port() {
        let address = find_free_metrics_addr().expect("metrics address");

        assert!(address.ip().is_loopback());
        assert_ne!(address.port(), 0);
        TcpListener::bind(address).expect("selected port should be available");
    }
}
