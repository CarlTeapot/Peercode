use crate::processes::types::{TunnelMetricsEventPayload, TunnelMetricsResponse};
use log::{debug, warn};
use std::time::Duration;
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Emitter};
use tokio::time::{interval, MissedTickBehavior};

pub const TUNNEL_METRICS_EVENT: &str = "process://tunnel-metrics";

pub fn spawn_tunnel_metrics_aggregator(
    app: AppHandle,
    metrics_url: String,
    poll_interval: Duration,
) -> JoinHandle<()> {
    tauri::async_runtime::spawn(async move {
        let client = reqwest::Client::new();
        let request_timeout = poll_interval.min(Duration::from_secs(2));
        let mut ticker = interval(poll_interval);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        debug!(
            "tunnel metrics aggregator started: interval_ms={}",
            poll_interval.as_millis()
        );

        loop {
            ticker.tick().await;
            let payload = match fetch_tunnel_metrics(&client, &metrics_url, request_timeout).await {
                Ok(metrics) => TunnelMetricsEventPayload {
                    metrics: Some(metrics),
                    error: None,
                },
                Err(error) => {
                    warn!("tunnel metrics scrape failed: {error}");
                    TunnelMetricsEventPayload {
                        metrics: None,
                        error: Some(error),
                    }
                }
            };

            if let Err(error) = app.emit(TUNNEL_METRICS_EVENT, payload) {
                warn!("failed to emit tunnel metrics event: {error}");
            }
        }
    })
}

async fn fetch_tunnel_metrics(
    client: &reqwest::Client,
    metrics_url: &str,
    request_timeout: Duration,
) -> Result<TunnelMetricsResponse, String> {
    let body = client
        .get(metrics_url)
        .timeout(request_timeout)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Cloudflare tunnel metrics: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Cloudflare tunnel metrics returned an error: {e}"))?
        .text()
        .await
        .map_err(|e| format!("Failed to read Cloudflare tunnel metrics: {e}"))?;

    parse_tunnel_metrics(&body)
}

fn parse_tunnel_metrics(body: &str) -> Result<TunnelMetricsResponse, String> {
    let mut ha_connections = None;
    let mut register_successes = 0;
    let mut request_errors = None;
    let mut edge_location = None;

    for line in body.lines().filter(|line| !line.starts_with('#')) {
        let Some((series, raw_value)) = line.rsplit_once(char::is_whitespace) else {
            continue;
        };
        let Ok(value) = raw_value.parse::<f64>() else {
            continue;
        };
        let metric_name = series.split_once('{').map_or(series, |(name, _)| name);

        match metric_name {
            "cloudflared_tunnel_ha_connections" => {
                ha_connections = Some(metric_u64(metric_name, value)?);
            }
            "cloudflared_tunnel_tunnel_register_success" => {
                register_successes += metric_u64(metric_name, value)?;
            }
            "cloudflared_tunnel_request_errors" => {
                request_errors = Some(metric_u64(metric_name, value)?);
            }
            "cloudflared_tunnel_server_locations" if value > 0.0 => {
                edge_location = label_value(series, "edge_location");
            }
            _ => {}
        }
    }

    Ok(TunnelMetricsResponse {
        ha_connections: ha_connections
            .ok_or_else(|| "cloudflared_tunnel_ha_connections metric is missing".to_string())?,
        register_successes,
        request_errors: request_errors
            .ok_or_else(|| "cloudflared_tunnel_request_errors metric is missing".to_string())?,
        edge_location,
    })
}

fn metric_u64(name: &str, value: f64) -> Result<u64, String> {
    if value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value <= u64::MAX as f64 {
        Ok(value as u64)
    } else {
        Err(format!("Invalid value for {name}: {value}"))
    }
}

fn label_value(series: &str, label: &str) -> Option<String> {
    let labels = series.split_once('{')?.1.strip_suffix('}')?;
    labels.split(',').find_map(|entry| {
        let (name, value) = entry.split_once('=')?;
        (name == label).then(|| value.trim_matches('"').to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::parse_tunnel_metrics;
    use crate::processes::types::TunnelMetricsResponse;

    #[test]
    fn parses_host_relevant_tunnel_metrics() {
        let body = r#"
# HELP cloudflared_tunnel_ha_connections Number of active ha connections
cloudflared_tunnel_ha_connections 1
cloudflared_tunnel_tunnel_register_success{rpcName="registerConnection"} 3
cloudflared_tunnel_request_errors 2
cloudflared_tunnel_server_locations{connection_id="0",edge_location="ist07"} 1
cloudflared_tunnel_server_locations{connection_id="0",edge_location="fra03"} 0
"#;

        assert_eq!(
            parse_tunnel_metrics(body).unwrap(),
            TunnelMetricsResponse {
                ha_connections: 1,
                register_successes: 3,
                request_errors: 2,
                edge_location: Some("ist07".to_string()),
            }
        );
    }
}
