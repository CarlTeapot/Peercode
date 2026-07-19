use std::collections::HashMap;
use std::sync::Mutex;

use crdt_core::types::ClientId;
use crdt_core::wire::{PeerInfoFrame, PermissionFrame};
use log::{debug, warn};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::session::session_types::{CanWritePayload, CAN_WRITE_CHANGED, ROOM_STATE_CHANGED};
use crate::state::app_role::WriteAccess;
use crate::state::appstate::AppState;
use crate::state::document::{request, DocOp};

#[derive(Clone)]
struct PeerEntry {
    username: String,
    is_host: bool,
    can_write: bool,
}

#[derive(Clone, Serialize)]
pub struct PeerDto {
    pub client_id: String,
    pub username: String,
    pub is_host: bool,
    pub can_write: bool,
}

#[derive(Clone, Serialize)]
pub struct RoomStatePayload {
    pub peers: Vec<PeerDto>,
}

/// The session peer list, fed by gateway-authored peer-info (`0x08`),
/// permission (`0x07`) and membership (`0x05`) frames. Owned by `AppState` so
/// permission decisions live in app state; the ws receiver only routes frames
/// to the `apply_*` functions below.
#[derive(Default)]
pub struct RosterState {
    peers: Mutex<HashMap<u64, PeerEntry>>,
}

impl RosterState {
    pub fn clear(&self) {
        self.peers.lock().unwrap().clear();
    }

    fn upsert(&self, client_id: u64, entry: PeerEntry) {
        self.peers.lock().unwrap().insert(client_id, entry);
    }

    fn set_can_write(&self, client_id: u64, can_write: bool) -> bool {
        match self.peers.lock().unwrap().get_mut(&client_id) {
            Some(entry) => {
                entry.can_write = can_write;
                true
            }
            None => false,
        }
    }

    fn remove(&self, client_id: u64) {
        self.peers.lock().unwrap().remove(&client_id);
    }

    fn can_write_of(&self, client_id: u64) -> Option<bool> {
        self.peers
            .lock()
            .unwrap()
            .get(&client_id)
            .map(|entry| entry.can_write)
    }

    fn room_state_payload(&self) -> RoomStatePayload {
        let peers = self.peers.lock().unwrap();
        let mut dtos: Vec<PeerDto> = peers
            .iter()
            .map(|(id, entry)| PeerDto {
                client_id: id.to_string(),
                username: entry.username.clone(),
                is_host: entry.is_host,
                can_write: entry.can_write,
            })
            .collect();
        dtos.sort_by(|a, b| {
            b.is_host
                .cmp(&a.is_host)
                .then_with(|| a.client_id.cmp(&b.client_id))
        });
        RoomStatePayload { peers: dtos }
    }
}

/// A gateway roster entry arrived: seed/update the peer list, and adopt the
/// permission it carries if it describes this client.
pub async fn apply_peer_info(app: &AppHandle, frame: PeerInfoFrame) {
    let state = app.state::<AppState>();
    state.roster.upsert(
        frame.client_id.value,
        PeerEntry {
            username: frame.username.clone(),
            is_host: frame.is_host,
            can_write: frame.can_write,
        },
    );
    apply_own_permission_if_local(app, frame.client_id, frame.can_write).await;
    emit_room_state(app);
}

pub async fn apply_permission(app: &AppHandle, frame: PermissionFrame) {
    let state = app.state::<AppState>();
    if !state
        .roster
        .set_can_write(frame.client_id.value, frame.can_write)
    {
        warn!(
            "roster: permission change for unknown peer {}",
            frame.client_id.value
        );
    }
    apply_own_permission_if_local(app, frame.client_id, frame.can_write).await;
    emit_room_state(app);
}

pub fn apply_peer_left(app: &AppHandle, client_id: ClientId) {
    app.state::<AppState>().roster.remove(client_id.value);
    emit_room_state(app);
}

/// Re-applies this client's roster permission to the role. The websocket
/// connects before `complete_guest` runs, so a peer-info frame can arrive
/// while the role is still `Starting` and its permission cannot land in the
/// role yet; `join_session` calls this once the Guest role exists.
pub fn resync_own_permission(app: &AppHandle, client_id: u64) {
    let state = app.state::<AppState>();
    let Some(can_write) = state.roster.can_write_of(client_id) else {
        return;
    };
    let access = if can_write {
        WriteAccess::Editable
    } else {
        WriteAccess::ReadOnly
    };
    if state.set_write_access(access) {
        debug!("roster: own permission resynced after join: can_write={can_write}");
        let _ = app.emit(CAN_WRITE_CHANGED, CanWritePayload { can_write });
    }
}

async fn apply_own_permission_if_local(app: &AppHandle, subject: ClientId, can_write: bool) {
    let state = app.state::<AppState>();
    let local_id = match request(&state.doc_tx, |reply| DocOp::GetClientId { reply }).await {
        Ok(id) => id,
        Err(e) => {
            warn!("roster: failed to read local client id: {e}");
            return;
        }
    };
    if local_id != subject {
        return;
    }
    let access = if can_write {
        WriteAccess::Editable
    } else {
        WriteAccess::ReadOnly
    };
    if state.set_write_access(access) {
        debug!("roster: local write access set to {access:?}");
    }
    let _ = app.emit(CAN_WRITE_CHANGED, CanWritePayload { can_write });
}

fn emit_room_state(app: &AppHandle) {
    let payload = app.state::<AppState>().roster.room_state_payload();
    let _ = app.emit(ROOM_STATE_CHANGED, payload);
}
