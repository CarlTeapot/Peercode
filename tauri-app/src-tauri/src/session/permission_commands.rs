use crate::state::appstate::AppState;
use crate::state::ws_state::WsState;
use crdt_core::types::ClientId;
use crdt_core::wire::{encode_permission, PermissionFrame};
use log::{info, warn};
use tauri::State;

/// Host-only: asks the gateway to grant/revoke a guest's write access. The
/// gateway validates the sender and echoes the authoritative permission frame
/// to every client (this one included), which is what updates local state.
#[tauri::command]
pub async fn set_peer_permission(
    target_client_id: String,
    can_write: bool,
    state: State<'_, AppState>,
    ws: State<'_, WsState>,
) -> Result<(), String> {
    info!("set_peer_permission requested: target={target_client_id}, can_write={can_write}");

    if !state.is_host() {
        warn!("set_peer_permission rejected: caller is not host");
        return Err("Only the host can change permissions".into());
    }

    let target = target_client_id
        .parse::<u64>()
        .map_err(|_| format!("Invalid client id: {target_client_id}"))?;

    let frame = encode_permission(&PermissionFrame {
        client_id: ClientId::new(target),
        can_write,
    });
    ws.send_raw(frame).await?;
    info!("set_peer_permission: permission frame sent to gateway");
    Ok(())
}
