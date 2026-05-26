use crate::processes::types::{CombinedWorkflowResult, Sidecar, SidecarStatus};
use crate::state::document::DocSender;
use crate::state::ws_state::WsState;
use log::{info, warn};
#[cfg(debug_assertions)]
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

pub use crate::state::app_role::AppRole;

pub struct AppState {
    pub doc_tx: DocSender,
    pub(crate) role: Mutex<AppRole>,
    pub processes: Mutex<HostProcesses>,
    pub current_document_name: Mutex<Option<String>>,
    #[cfg(debug_assertions)]
    pub crdt_logging_enabled: AtomicBool,
}

pub struct HostProcesses {
    pub gateway: Option<Sidecar>,
    pub tunnel: Option<Sidecar>,
    pub gateway_auth_token: Option<String>,
    pub gateway_port: Option<u16>,
    pub gateway_lan_url: Option<String>,
    pub tunnel_public_url: Option<String>,
}

impl HostProcesses {
    pub fn combined_workflow_result(&self) -> Option<CombinedWorkflowResult> {
        match (
            self.gateway
                .as_ref()
                .filter(|s| s.status == SidecarStatus::Enabled),
            self.gateway_port,
            self.gateway_auth_token.clone(),
        ) {
            (Some(_), Some(port), Some(gateway_auth_token)) => Some(CombinedWorkflowResult {
                port,
                gateway_auth_token,
                lan_url: self.gateway_lan_url.clone(),
                public_url: self.tunnel_public_url.clone(),
            }),
            _ => None,
        }
    }
}

impl AppState {
    pub fn new(doc_tx: DocSender) -> Self {
        Self {
            doc_tx,
            role: Mutex::new(crate::state::app_role::AppRole::Undecided),
            processes: Mutex::new(HostProcesses {
                gateway: None,
                tunnel: None,
                gateway_auth_token: None,
                gateway_port: None,
                gateway_lan_url: None,
                tunnel_public_url: None,
            }),
            current_document_name: Mutex::new(None),
            #[cfg(debug_assertions)]
            crdt_logging_enabled: AtomicBool::new(false),
        }
    }

    pub fn leave_session(&self, ws: &WsState) {
        ws.disconnect_nowait();
    }

    pub fn gateway_auth_token(&self) -> Option<String> {
        self.processes.lock().unwrap().gateway_auth_token.clone()
    }

    pub fn combined_workflow_result(&self) -> Option<CombinedWorkflowResult> {
        self.processes.lock().unwrap().combined_workflow_result()
    }

    pub fn kill_host_processes(&self) -> bool {
        let mut procs = self.processes.lock().unwrap();
        let had_tunnel = procs.tunnel.is_some();
        let had_gateway = procs.gateway.is_some();
        self.kill_proc(procs.tunnel.take());
        self.kill_proc(procs.gateway.take());
        if had_gateway {
            procs.gateway_auth_token = None;
            procs.gateway_port = None;
            procs.gateway_lan_url = None;
            procs.tunnel_public_url = None;
        }
        had_gateway || had_tunnel
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
