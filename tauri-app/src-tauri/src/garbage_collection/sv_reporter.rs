//! Guest-only state-vector reporter. Polls the doc actor and reports the SV to
//! the host (debounced + heartbeat) so its GC floor advances even for observers.
//! See docs/garbage-collection-plan.md.

use std::time::Duration;

use crdt_core::encode_sv_report;
use crdt_core::store::StateVector;
use crdt_core::types::ClientId;
use log::{debug, info};
use tauri::{AppHandle, Manager};
use tokio::sync::watch;

use crate::state::appstate::{AppRole, AppState};
use crate::state::document::{request, DocOp};
use crate::state::ws_state::WsState;

const POLL_INTERVAL: Duration = Duration::from_millis(750);
/// Report after this many idle polls (~10.5s) even when the SV is unchanged.
const HEARTBEAT_TICKS: u32 = 14;

pub async fn run(app: AppHandle, mut snapshot_ready: watch::Receiver<bool>) -> Result<(), String> {
    info!("sv reporter started");
    wait_for_snapshot(&app, &mut snapshot_ready).await?;

    let sender = match request(&app.state::<AppState>().doc_tx.clone(), |reply| {
        DocOp::GetClientId { reply }
    })
    .await
    {
        Ok(id) => id,
        Err(e) => {
            info!("sv reporter: could not read client id, not starting: {e}");
            return Err(e);
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
            Err(e) => return Err(e),
        };

        ticks_since_send += 1;
        let changed = last_sent.as_ref() != Some(&sv);
        let heartbeat = ticks_since_send >= HEARTBEAT_TICKS;
        if changed || heartbeat {
            send_report(&app, sender, &sv).await?;
            last_sent = Some(sv);
            ticks_since_send = 0;
        }
    }

    info!("sv reporter stopped");
    Ok(())
}

async fn wait_for_snapshot(
    app: &AppHandle,
    snapshot_ready: &mut watch::Receiver<bool>,
) -> Result<(), String> {
    while !*snapshot_ready.borrow() {
        if !matches!(
            app.state::<AppState>().current_role(),
            AppRole::Guest { .. }
        ) {
            return Ok(());
        }
        snapshot_ready
            .changed()
            .await
            .map_err(|_| "snapshot readiness channel closed".to_string())?;
    }
    Ok(())
}

async fn send_report(app: &AppHandle, sender: ClientId, sv: &StateVector) -> Result<(), String> {
    let entries: Vec<(ClientId, u64)> = sv.iter().map(|(c, n)| (*c, *n)).collect();
    let frame = encode_sv_report(sender, &entries);
    app.state::<WsState>().send_raw(frame).await?;
    debug!("sv reporter: sent report with {} entries", entries.len());
    Ok(())
}
