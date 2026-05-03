use std::time::Duration;

use log::LevelFilter;
use serde::Deserialize;

const RAW_CONFIG: &str = include_str!("../../peercode.config.toml");

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub websocket: WebsocketConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    pub show_gateway_logs: bool,
    pub show_cloudflared_logs: bool,
    pub tauri_file_logs: bool,
    pub tauri_log_level: String,
    pub gateway_log_level: String,
    pub tunnel_log_level: String,
}

impl LoggingConfig {
    pub fn level_filter(&self) -> LevelFilter {
        match self.tauri_log_level.trim().to_ascii_lowercase().as_str() {
            "off" => LevelFilter::Off,
            "error" => LevelFilter::Error,
            "warn" | "warning" => LevelFilter::Warn,
            "info" => LevelFilter::Info,
            "debug" => LevelFilter::Debug,
            "trace" => LevelFilter::Trace,
            _ => LevelFilter::Info,
        }
    }

    pub fn gateway_level_for_env(&self) -> String {
        normalize_level(&self.gateway_log_level).to_string()
    }

    pub fn tunnel_level_for_arg(&self) -> String {
        match normalize_level(&self.tunnel_log_level) {
            "off" => "error".to_string(),
            "trace" => "debug".to_string(),
            lvl => lvl.to_string(),
        }
    }
}

fn normalize_level(raw: &str) -> &str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "off" => "off",
        "error" => "error",
        "warn" | "warning" => "warn",
        "info" => "info",
        "debug" => "debug",
        "trace" => "trace",
        _ => "info",
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebsocketConfig {
    pub connect_timeout_ms: u64,
}

impl WebsocketConfig {
    pub fn connect_timeout(&self) -> Duration {
        Duration::from_millis(self.connect_timeout_ms)
    }
}

impl AppConfig {
    pub fn load() -> Self {
        toml::from_str(RAW_CONFIG).expect("invalid peercode.app_config.toml")
    }
}
