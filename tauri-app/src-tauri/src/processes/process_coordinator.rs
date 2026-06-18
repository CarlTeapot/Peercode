use crate::app_config::config::AppConfig;
use crate::processes::error::emit_error;
use crate::processes::gateway_process::run_gateway;
use crate::processes::process_logger::pipe_process_logs;
use crate::processes::process_metrics_aggregator::spawn_metrics_aggregator;
use crate::processes::tunnel_process::run_cloudflared;
use crate::processes::types::{CombinedWorkflowResult, GatewayMetricsSource, TunnelMetricsSource};
use crate::state::appstate::AppState;
use log::{debug, error, info, warn};
use tauri::{AppHandle, Manager};

pub async fn launch(app: AppHandle) -> Result<CombinedWorkflowResult, String> {
    let config = app.state::<AppConfig>().inner().clone();
    let logging = config.logging;
    info!("process coordinator launch requested");
    debug!(
        "process coordinator log pipe config: gateway={}, cloudflared={}",
        logging.show_gateway_logs, logging.show_cloudflared_logs
    );

    let gateway = match run_gateway(&app).await {
        Ok(Some(r)) => {
            info!(
                "gateway started: port={} lan_url_present={}",
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
    let gateway_auth_token = gateway.gateway_auth_token.clone();
    spawn_gateway_metrics_task(
        &app,
        port,
        gateway_auth_token.clone(),
        config.metrics.poll_interval(),
    );
    let mut public_url: Option<String> = None;

    match run_cloudflared(&app, port).await {
        Ok(Some(tunnel)) => {
            info!(
                "cloudflared started: public_url={} metrics_url={}",
                tunnel.public_url, tunnel.metrics_url
            );
            spawn_tunnel_metrics_task(&app, tunnel.metrics_url, config.metrics.poll_interval());
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

    Ok(CombinedWorkflowResult {
        port,
        lan_url,
        public_url,
        gateway_auth_token,
    })
}

fn spawn_gateway_metrics_task(
    app: &AppHandle,
    port: u16,
    auth_token: String,
    poll_interval: std::time::Duration,
) {
    let gateway_metrics_url = format!("http://127.0.0.1:{port}/metrics");
    let metrics_task = spawn_metrics_aggregator(
        app.clone(),
        GatewayMetricsSource::new(gateway_metrics_url, auth_token),
        poll_interval,
    );
    let previous_task = app
        .state::<AppState>()
        .processes
        .lock()
        .unwrap()
        .gateway_metrics_task
        .replace(metrics_task);
    if let Some(task) = previous_task {
        task.abort();
    }
}

fn spawn_tunnel_metrics_task(
    app: &AppHandle,
    metrics_url: String,
    poll_interval: std::time::Duration,
) {
    let metrics_task = spawn_metrics_aggregator(
        app.clone(),
        TunnelMetricsSource::new(metrics_url),
        poll_interval,
    );
    let previous_task = app
        .state::<AppState>()
        .processes
        .lock()
        .unwrap()
        .tunnel_metrics_task
        .replace(metrics_task);
    if let Some(task) = previous_task {
        task.abort();
    }
}
