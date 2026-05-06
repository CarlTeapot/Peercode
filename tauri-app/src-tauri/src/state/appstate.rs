use crdt_core::types::ClientId;
use crdt_core::Document;
use log::{info, warn};
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use tauri_plugin_shell::process::CommandChild;

pub struct AppState {
    pub document: Mutex<Document>,
    pub role: Mutex<AppRole>,
    pub processes: Mutex<HostProcesses>,
    pub current_document_name: Mutex<Option<String>>,
    #[cfg(debug_assertions)]
    pub crdt_logging_enabled: AtomicBool,
}

pub enum AppRole {
    Undecided,
    Starting,
    Host {
        room_id: String,
        lan_url: Option<String>,
        public_url: Option<String>,
    },
    Guest {
        room_id: String,
        server_url: String,
    },
}

impl AppRole {
    pub fn status(&self) -> &'static str {
        match self {
            Self::Undecided => "idle",
            Self::Starting => "starting",
            Self::Host { .. } => "host",
            Self::Guest { .. } => "guest",
        }
    }

    pub fn can_initiate_session(&self) -> bool {
        matches!(self, Self::Undecided | Self::Starting)
    }
}

pub struct HostProcesses {
    pub gateway: Option<CommandChild>,
    pub tunnel: Option<CommandChild>,
}

impl AppState {
    pub fn new(client_id: ClientId) -> Self {
        Self {
            document: Mutex::new(Document::new(client_id)),
            role: Mutex::new(AppRole::Undecided),
            processes: Mutex::new(HostProcesses {
                gateway: None,
                tunnel: None,
            }),
            current_document_name: Mutex::new(None),
            #[cfg(debug_assertions)]
            crdt_logging_enabled: AtomicBool::new(false),
        }
    }

    pub fn replace_document(&self, doc: Document) {
        let mut current = self.document.lock().unwrap();
        *current = doc;
    }

    pub fn teardown_host(&self) {
        info!("teardown_host requested");
        let mut role = self.role.lock().unwrap();
        let previous_status = role.status();
        if matches!(*role, AppRole::Starting | AppRole::Host { .. }) {
            *role = AppRole::Undecided;
            info!(
                "teardown_host role reset to idle from status={}",
                previous_status
            );
        }

        let mut procs = self.processes.lock().unwrap();
        if let Some(child) = procs.tunnel.take() {
            if let Err(e) = child.kill() {
                warn!("teardown_host failed to kill tunnel process: {}", e);
            } else {
                info!("teardown_host killed tunnel process");
            }
        }
        if let Some(child) = procs.gateway.take() {
            if let Err(e) = child.kill() {
                warn!("teardown_host failed to kill gateway process: {}", e);
            } else {
                info!("teardown_host killed gateway process");
            }
        }
        info!("teardown_host completed");
    }
}
