//! Host-only state-vector-floor GC coordinator. See docs/garbage-collection-plan.md.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use crdt_core::encode_gc_commit;
use crdt_core::store::{DeleteSet, StateVector};
use crdt_core::types::{BlockId, ClientId, Clock};
use log::{debug, info, warn};
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
}

pub struct GcCoordinator {
    app: AppHandle,
    rx: mpsc::Receiver<GcEvent>,
    peer_svs: HashMap<ClientId, StateVector>,
}

impl GcCoordinator {
    pub fn spawn(app: AppHandle) -> mpsc::Sender<GcEvent> {
        let (tx, rx) = mpsc::channel(EVENT_CHANNEL_CAPACITY);
        let coordinator = GcCoordinator {
            app,
            rx,
            peer_svs: HashMap::new(),
        };
        tauri::async_runtime::spawn(coordinator.run());
        tx
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

        let min_sv = compute_min_sv(&self.peer_svs, &own_sv);
        let confirmed = intersect(&own_ds, &min_sv);
        if confirmed.is_empty() {
            return;
        }

        if let Err(e) = request_fallible(&doc_tx, |reply| DocOp::ApplyGcCommit {
            confirmed: confirmed.clone(),
            reply,
        })
        .await
        {
            warn!("gc coordinator: local gc-commit apply failed: {e}");
            return;
        }

        self.app
            .state::<WsState>()
            .send_raw(encode_gc_commit(&confirmed))
            .await;

        debug!(
            "gc coordinator: emitted gc-commit covering {} range(s)",
            confirmed.iter().count()
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

/// Ranges of `ds` confirmed by `min_sv` (`sv[c] = N` ⇒ clocks `[0, N)` seen).
fn intersect(ds: &DeleteSet, min_sv: &StateVector) -> DeleteSet {
    let mut out = DeleteSet::new();
    for (client, range) in ds.iter() {
        let cap = min_sv.get(client);
        let confirmed_end = cap.min(range.end());
        if confirmed_end > range.start {
            out.add(
                BlockId::new(*client, Clock::new(range.start)),
                confirmed_end - range.start,
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sv(entries: &[(u64, u64)]) -> StateVector {
        StateVector::from_entries(
            entries
                .iter()
                .map(|&(c, n)| (ClientId::new(c), n))
                .collect(),
        )
    }

    fn ds(entries: &[(u64, u64, u64)]) -> DeleteSet {
        let mut d = DeleteSet::new();
        for &(c, start, len) in entries {
            d.add(BlockId::new(ClientId::new(c), Clock::new(start)), len);
        }
        d
    }

    #[test]
    fn intersect_confirms_up_to_floor() {
        let confirmed = intersect(&ds(&[(1, 0, 10)]), &sv(&[(1, 6)]));
        let collected: Vec<_> = confirmed
            .iter()
            .map(|(c, r)| (c.value, r.start, r.len))
            .collect();
        assert_eq!(collected, vec![(1, 0, 6)]);
    }

    #[test]
    fn intersect_confirms_nothing_when_floor_is_zero() {
        let confirmed = intersect(&ds(&[(1, 0, 5)]), &sv(&[]));
        assert!(confirmed.is_empty());
    }

    #[test]
    fn intersect_confirms_full_range_when_floor_past_end() {
        let confirmed = intersect(&ds(&[(1, 2, 3)]), &sv(&[(1, 100)]));
        let collected: Vec<_> = confirmed
            .iter()
            .map(|(c, r)| (c.value, r.start, r.len))
            .collect();
        assert_eq!(collected, vec![(1, 2, 3)]);
    }

    #[test]
    fn min_sv_takes_per_client_minimum_treating_absent_as_zero() {
        let peer_svs = HashMap::from([
            (ClientId::new(7), sv(&[(1, 5), (2, 9)])),
            (ClientId::new(8), sv(&[(1, 8)])),
        ]);
        let own = sv(&[(1, 10), (2, 9)]);
        let min = compute_min_sv(&peer_svs, &own);
        assert_eq!(min.get(&ClientId::new(1)), 5); // min(10,5,8)
        assert_eq!(min.get(&ClientId::new(2)), 0); // peer 8 absent -> 0
    }

    #[test]
    fn stale_report_after_left_does_not_resurrect_peer() {
        let peer = ClientId::new(7);
        let mut peers = HashMap::new();
        apply_event(&mut peers, GcEvent::Joined(peer));
        apply_event(
            &mut peers,
            GcEvent::PeerSvReport {
                client: peer,
                sv: sv(&[(1, 5)]),
            },
        );
        apply_event(&mut peers, GcEvent::Left(peer));
        apply_event(
            &mut peers,
            GcEvent::PeerSvReport {
                client: peer,
                sv: sv(&[(1, 5)]),
            },
        );
        assert!(
            peers.is_empty(),
            "a report delivered after `left` must not re-add the peer"
        );
    }

    #[test]
    fn reports_merge_per_client_max_into_joined_entry() {
        let peer = ClientId::new(7);
        let mut peers = HashMap::new();
        apply_event(&mut peers, GcEvent::Joined(peer));
        apply_event(
            &mut peers,
            GcEvent::PeerSvReport {
                client: peer,
                sv: sv(&[(1, 5)]),
            },
        );
        apply_event(
            &mut peers,
            GcEvent::PeerSvReport {
                client: peer,
                sv: sv(&[(1, 3), (2, 4)]),
            },
        );
        let entry = &peers[&peer];
        assert_eq!(entry.get(&ClientId::new(1)), 5);
        assert_eq!(entry.get(&ClientId::new(2)), 4);
    }

    #[test]
    fn min_sv_equals_own_when_no_peers() {
        let own = sv(&[(1, 4), (2, 7)]);
        let min = compute_min_sv(&HashMap::new(), &own);
        assert_eq!(min.get(&ClientId::new(1)), 4);
        assert_eq!(min.get(&ClientId::new(2)), 7);
    }
}
