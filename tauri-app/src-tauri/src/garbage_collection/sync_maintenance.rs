use std::future::Future;
use std::sync::Mutex;

use crdt_core::store::StateVector;
use crdt_core::types::ClientId;
use log::{debug, info, warn};
use tauri::async_runtime::JoinHandle;
use tauri::AppHandle;
use tokio::sync::{mpsc, watch};

use crate::garbage_collection::garbage_collector::{GcCoordinator, GcEvent, GcTask};

struct HostGcTask {
    tx: mpsc::Sender<GcEvent>,
    task: JoinHandle<()>,
}

struct GuestSvTask {
    snapshot_ready: watch::Sender<bool>,
    task: JoinHandle<()>,
}

#[derive(Default)]
struct SyncMaintenanceInner {
    host_gc: Option<HostGcTask>,
    guest_sv: Option<GuestSvTask>,
}

#[derive(Default)]
pub struct SyncMaintenance {
    inner: Mutex<SyncMaintenanceInner>,
}

impl SyncMaintenance {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_host_gc(&self, app: AppHandle) {
        let GcTask { tx, task } = GcCoordinator::spawn(app);
        let mut inner = self.inner.lock().unwrap();
        stop_host_gc(&mut inner);
        inner.host_gc = Some(HostGcTask { tx, task });
        info!("sync maintenance: host gc coordinator started");
    }

    pub fn start_guest_sv_reporter(&self, app: AppHandle) {
        let (snapshot_ready, ready_rx) = watch::channel(false);
        let task = spawn_logged(
            "sv reporter",
            crate::garbage_collection::sv_reporter::run(app, ready_rx),
        );

        let mut inner = self.inner.lock().unwrap();
        stop_guest_sv(&mut inner);
        inner.guest_sv = Some(GuestSvTask {
            snapshot_ready,
            task,
        });
        info!("sync maintenance: guest sv reporter waiting for snapshot");
    }

    pub fn mark_guest_snapshot_applied(&self) {
        let ready = self
            .inner
            .lock()
            .unwrap()
            .guest_sv
            .as_ref()
            .map(|task| task.snapshot_ready.clone());

        if let Some(ready) = ready {
            let _ = ready.send(true);
            debug!("sync maintenance: guest snapshot marked ready");
        }
    }

    pub fn stop_all(&self) {
        let mut inner = self.inner.lock().unwrap();
        stop_host_gc(&mut inner);
        stop_guest_sv(&mut inner);
    }

    pub async fn peer_joined(&self, client: ClientId) {
        self.send_gc_event(GcEvent::Joined(client)).await;
    }

    pub async fn peer_left(&self, client: ClientId) {
        self.send_gc_event(GcEvent::Left(client)).await;
    }

    pub async fn peer_state_vector(&self, client: ClientId, sv: StateVector) {
        self.send_gc_event(GcEvent::PeerSvReport { client, sv })
            .await;
    }

    pub async fn document_replaced(&self) {
        self.send_gc_event(GcEvent::DocumentReplaced).await;
    }

    async fn send_gc_event(&self, event: GcEvent) {
        let tx = self
            .inner
            .lock()
            .unwrap()
            .host_gc
            .as_ref()
            .map(|task| task.tx.clone());

        let Some(tx) = tx else {
            return;
        };

        if tx.send(event).await.is_err() {
            warn!("sync maintenance: host gc coordinator channel closed");
        }
    }
}

fn stop_host_gc(inner: &mut SyncMaintenanceInner) {
    if let Some(task) = inner.host_gc.take() {
        task.task.abort();
        info!("sync maintenance: host gc coordinator stopped");
    }
}

fn stop_guest_sv(inner: &mut SyncMaintenanceInner) {
    if let Some(task) = inner.guest_sv.take() {
        task.task.abort();
        info!("sync maintenance: guest sv reporter stopped");
    }
}

fn spawn_logged<F>(name: &'static str, future: F) -> JoinHandle<()>
where
    F: Future<Output = Result<(), String>> + Send + 'static,
{
    tauri::async_runtime::spawn(async move {
        match future.await {
            Ok(()) => debug!("sync maintenance: {name} exited"),
            Err(e) => warn!("sync maintenance: {name} failed: {e}"),
        }
    })
}
