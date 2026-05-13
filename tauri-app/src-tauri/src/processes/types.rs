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
