//! Host-only state-vector-floor GC coordinator. See docs/garbage-collection-plan.md.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use crdt_core::encode_gc_commit;
use crdt_core::store::{DeleteSet, StateVector};
use crdt_core::types::ClientId;
use log::{debug, info, warn};
use tauri::async_runtime::JoinHandle;
use tauri::{AppHandle, Manager};
use tokio::sync::mpsc;

use crate::state::appstate::AppState;
use crate::state::document::client::{request, request_fallible};
use crate::state::document::DocOp;
use crate::state::ws_state::WsState;

const RECOMPUTE_INTERVAL: Duration = Duration::from_millis(500);
const EVENT_CHANNEL_CAPACITY: usize = 256;

#[derive(Debug)]
pub enum GcEvent {
    PeerSvReport { client: ClientId, sv: StateVector },
    Joined(ClientId),
    Left(ClientId),
    DocumentReplaced,
}

pub struct GcCoordinator {
    app: AppHandle,
    rx: mpsc::Receiver<GcEvent>,
    peer_svs: HashMap<ClientId, StateVector>,
}

pub struct GcTask {
    pub tx: mpsc::Sender<GcEvent>,
    pub task: JoinHandle<()>,
}

impl GcCoordinator {
    pub fn spawn(app: AppHandle) -> GcTask {
        let (tx, rx) = mpsc::channel(EVENT_CHANNEL_CAPACITY);
        let coordinator = GcCoordinator {
            app,
            rx,
            peer_svs: HashMap::new(),
        };
        let task = tauri::async_runtime::spawn(coordinator.run());
        GcTask { tx, task }
    }

    async fn run(mut self) {
        info!("gc coordinator started");
        let mut tick = tokio::time::interval(RECOMPUTE_INTERVAL);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                maybe_event = self.rx.recv() => {
                    match maybe_event {
                        Some(event) => self.handle_event(event),
                        None => break,
                    }
                }
                _ = tick.tick() => {
                    if !self.app.state::<AppState>().is_host() {
                        break;
                    }
                    self.recompute().await;
                }
            }
        }
        info!("gc coordinator stopped");
    }

    fn handle_event(&mut self, event: GcEvent) {
        apply_event(&mut self.peer_svs, event);
    }

    async fn recompute(&mut self) {
        let doc_tx = self.app.state::<AppState>().doc_tx.clone();

        let (own_sv, own_ds) = match request(&doc_tx, |reply| DocOp::FetchGcData { reply }).await {
            Ok(pair) => pair,
            Err(e) => {
                warn!("gc coordinator: failed to read own state vector / delete set: {e}");
                return;
            }
        };

        let floor = compute_min_sv(&self.peer_svs, &own_sv);
        if !has_compactable_deletes(&own_ds, &floor) {
            return;
        }

        if let Err(e) = request_fallible(&doc_tx, |reply| DocOp::ApplyGcCommit {
            floor: floor.clone(),
            reply,
        })
        .await
        {
            warn!("gc coordinator: local gc-commit apply failed: {e}");
            return;
        }

        self.app
            .state::<WsState>()
            .send_raw(encode_gc_commit(&floor))
            .await
            .map_err(|e| warn!("gc coordinator: broadcast failed: {e}"))
            .ok();

        debug!(
            "gc coordinator: emitted gc-commit for {} client(s)",
            floor.iter().count()
        );
    }
}

fn apply_event(peer_svs: &mut HashMap<ClientId, StateVector>, event: GcEvent) {
    match event {
        GcEvent::PeerSvReport { client, sv } => {
            if let Some(entry) = peer_svs.get_mut(&client) {
                merge_into(entry, &sv);
            }
        }
        GcEvent::Joined(client) => {
            peer_svs.entry(client).or_default();
        }
        GcEvent::Left(client) => {
            peer_svs.remove(&client);
        }
        GcEvent::DocumentReplaced => {
            for sv in peer_svs.values_mut() {
                *sv = StateVector::new();
            }
        }
    }
}

/// Per-client minimum over `{own_sv} ∪ peer_svs`, treating absent clients as 0.
fn compute_min_sv(peer_svs: &HashMap<ClientId, StateVector>, own_sv: &StateVector) -> StateVector {
    let mut clients: HashSet<ClientId> = HashSet::new();
    for (c, _) in own_sv.iter() {
        clients.insert(*c);
    }
    for sv in peer_svs.values() {
        for (c, _) in sv.iter() {
            clients.insert(*c);
        }
    }

    let mut min_sv = StateVector::new();
    for c in clients {
        let mut floor = own_sv.get(&c);
        for sv in peer_svs.values() {
            floor = floor.min(sv.get(&c));
        }
        min_sv.update(c, floor);
    }
    min_sv
}

fn merge_into(dst: &mut StateVector, src: &StateVector) {
    for (client, clock) in src.iter() {
        dst.update(*client, *clock);
    }
}

fn has_compactable_deletes(ds: &DeleteSet, floor: &StateVector) -> bool {
    for (client, range) in ds.iter() {
        if range.start < floor.get(client) {
            return true;
        }
    }
    false
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
