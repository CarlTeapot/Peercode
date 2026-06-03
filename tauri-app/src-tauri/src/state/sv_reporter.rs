//! Guest-only state-vector reporter. Polls the doc actor and reports the SV to
//! the host (debounced + heartbeat) so its GC floor advances even for observers.
//! See docs/garbage-collection-plan.md.

use std::time::Duration;

use crdt_core::encode_sv_report;
use crdt_core::store::StateVector;
use crdt_core::types::ClientId;
use log::{debug, info};
use tauri::{AppHandle, Manager};

use crate::state::appstate::{AppRole, AppState};
use crate::state::document::{request, DocOp};
use crate::state::ws_state::WsState;

const POLL_INTERVAL: Duration = Duration::from_millis(750);
/// Report after this many idle polls (~10.5s) even when the SV is unchanged.
const HEARTBEAT_TICKS: u32 = 14;

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(run(app));
}

async fn run(app: AppHandle) {
    info!("sv reporter started");

    let sender = match request(&app.state::<AppState>().doc_tx.clone(), |reply| {
        DocOp::GetClientId { reply }
    })
    .await
    {
        Ok(id) => id,
        Err(e) => {
            info!("sv reporter: could not read client id, not starting: {e}");
            return;
        }
    };

    let mut interval = tokio::time::interval(POLL_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut last_sent: Option<StateVector> = None;
    let mut ticks_since_send: u32 = 0;

    loop {
        interval.tick().await;

        if !matches!(
            app.state::<AppState>().current_role(),
            AppRole::Guest { .. }
        ) {
            break;
        }

        let doc_tx = app.state::<AppState>().doc_tx.clone();
        let sv = match request(&doc_tx, |reply| DocOp::GetStateVector { reply }).await {
            Ok(sv) => sv,
            Err(_) => break,
        };

        ticks_since_send += 1;
        let changed = last_sent.as_ref() != Some(&sv);
        let heartbeat = ticks_since_send >= HEARTBEAT_TICKS;
        if changed || heartbeat {
            send_report(&app, sender, &sv).await;
            last_sent = Some(sv);
            ticks_since_send = 0;
        }
    }

    info!("sv reporter stopped");
}

async fn send_report(app: &AppHandle, sender: ClientId, sv: &StateVector) {
    let entries: Vec<(ClientId, u64)> = sv.iter().map(|(c, n)| (*c, *n)).collect();
    let frame = encode_sv_report(sender, &entries);
    app.state::<WsState>().send_raw(frame).await;
    debug!("sv reporter: sent report with {} entries", entries.len());
}
