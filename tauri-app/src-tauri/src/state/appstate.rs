use crate::processes::types::Sidecar;
use crate::state::ws_state::WsState;
use crdt_core::types::ClientId;
use crdt_core::Document;
use log::{info, warn};
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
pub struct AppState {
    pub document: Mutex<Document>,
    pub role: Mutex<AppRole>,
    pub processes: Mutex<HostProcesses>,
    pub current_document_name: Mutex<Option<String>>,
    #[cfg(debug_assertions)]
    pub crdt_logging_enabled: AtomicBool,
}

#[derive(Clone)]
pub enum AppRole {
    Undecided,
    Starting,
    Host {
        room_id: String,
        lan_url: Option<String>,
        public_url: Option<String>,
        local_room_url: String,
        public_room_url: Option<String>,
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
        matches!(self, Self::Undecided)
    }
}

pub struct HostProcesses {
    pub gateway: Option<Sidecar>,
    pub tunnel: Option<Sidecar>,
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

    pub fn leave_session(&self, ws: &WsState) {
        let previous_role = {
            let mut role = self.role.lock().unwrap();
            let prev = role.clone();
            *role = AppRole::Undecided;
            prev
        };
        info!("role reset to idle from status={}", previous_role.status());

        ws.disconnect_nowait();
    }

    pub fn kill_host_processes(&self) {
        let mut procs = self.processes.lock().unwrap();
        self.kill_proc(procs.tunnel.take());
        self.kill_proc(procs.gateway.take());
    }

    fn kill_proc(&self, proc: Option<Sidecar>) {
        if let Some(sidecar) = proc {
            if let Err(e) = sidecar.proc.kill() {
                warn!("failed to kill sidecar '{}': {}", sidecar.name, e);
            } else {
                info!("killed sidecar '{}'", sidecar.name);
            }
        }
    }
}
