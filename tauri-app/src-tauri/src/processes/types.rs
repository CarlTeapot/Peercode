use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tokio::sync::mpsc::Receiver;

#[derive(serde::Serialize, Clone, PartialEq)]
pub enum SidecarStatus {
    Enabled,
    Disabled,
}

pub struct Sidecar {
    pub proc: CommandChild,
    pub name: String,
    pub status: SidecarStatus,
}

pub struct GatewayWorkflowResult {
    pub gateway_auth_token: String,
    pub lan_url: Option<String>,
    pub port: u16,
    pub log_rx: Receiver<CommandEvent>,
}

pub struct TunnelWorkflowResult {
    pub public_url: String,
    pub metrics_url: String,
    pub log_rx: Receiver<CommandEvent>,
}

pub struct CombinedWorkflowResult {
    pub port: u16,
    pub public_url: Option<String>,
    pub lan_url: Option<String>,
    pub gateway_auth_token: String,
}

#[derive(serde::Serialize)]
pub struct ProcessStatusResponse {
    pub gateway: SidecarStatus,
    pub tunnel: SidecarStatus,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct TunnelMetricsResponse {
    pub ha_connections: u64,
    pub register_successes: u64,
    pub request_errors: u64,
    pub edge_location: Option<String>,
}

#[derive(Clone, serde::Serialize)]
pub struct TunnelMetricsEventPayload {
    pub metrics: Option<TunnelMetricsResponse>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct GatewayMetricsResponse {
    pub healthy: bool,
    pub uptime_seconds: u64,
    pub active_rooms: i64,
    pub connected_clients: i64,
    pub active_hosts: i64,
    pub relayed_messages: u64,
    pub relayed_bytes: u64,
    pub replay_successes: u64,
    pub replay_failures: u64,
    pub dropped_frames: u64,
    pub slow_client_disconnects: u64,
}

#[derive(Clone, serde::Serialize)]
pub struct GatewayMetricsEventPayload {
    pub metrics: Option<GatewayMetricsResponse>,
    pub error: Option<String>,
}
