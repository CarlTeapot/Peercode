use crate::session::session_types::JoinInfo;
use crate::state::appstate::{AppRole, AppState};
use crate::state::document::{request, DocOp};
use crate::state::ws_state::WsState;
use log::{debug, info, warn};
use tauri::{AppHandle, State};
use url::Url;

#[tauri::command]
pub async fn join_session(
    app: AppHandle,
    url: String,
    state: State<'_, AppState>,
    ws: State<'_, WsState>,
) -> Result<(), String> {
    info!("join_session requested: {url}");
    let join_info = parse_join_url(url)?;
    debug!(
        "join_session parsed url: server_url={} room_id={}",
        join_info.server_url, join_info.room_id
    );

    {
        let mut role = state.role.lock().unwrap();
        if !role.can_initiate_session() {
            warn!("join_session rejected: active session already exists");
            return Err("A session is already active".into());
        }
        *role = AppRole::Starting;
        debug!("join_session role set to Starting");
    }

    let guest_client_id = request(&state.doc_tx, |reply| DocOp::GetClientId { reply })
        .await
        .map_err(|e| {
            *state.role.lock().unwrap() = AppRole::Undecided;
            warn!("join_session: failed to read client_id from doc actor: {e}");
            e
        })?
        .value;

    let ws_url = format!(
        "{}/ws?room={}&client_id={}",
        join_info.server_url, join_info.room_id, guest_client_id
    );
    debug!(
        "join_session attempting websocket connect: room_id={} client_id={}",
        join_info.room_id, guest_client_id
    );

    ws.connect(&ws_url, join_info.room_id.clone(), app.clone())
        .await
        .map_err(|e| {
            *state.role.lock().unwrap() = AppRole::Undecided;
            warn!("join_session websocket connect failed; role reset to idle: {e}");
            e.to_string()
        })?;
    info!(
        "join_session websocket connected: room_id={}",
        join_info.room_id
    );

    let should_disconnect = {
        let mut role = state.role.lock().unwrap();
        if matches!(*role, AppRole::Starting) {
            *role = AppRole::Guest {
                room_id: join_info.room_id.clone(),
                server_url: join_info.server_url.clone(),
            };
            info!(
                "join_session role transitioned to Guest: room_id={}",
                join_info.room_id
            );
            false
        } else {
            warn!("join_session cancelled during role transition; disconnecting websocket");
            true
        }
    };
    if !should_disconnect {
        return Ok(());
    }

    let _ = ws.disconnect().await;
    info!("join_session websocket disconnected after cancellation");

    Err("Join session was cancelled".into())
}

#[tauri::command]
pub fn parse_join_url(url: String) -> Result<JoinInfo, String> {
    debug!("parse_join_url requested");
    let parsed = Url::parse(&url).map_err(|e| format!("Invalid URL: {e}"))?;

    let scheme = parsed.scheme();
    if scheme != "ws" && scheme != "wss" {
        warn!("parse_join_url rejected invalid scheme: {}", scheme);
        return Err("Invalid URL: must begin with ws:// or wss://".to_string());
    }

    if parsed
        .host_str()
        .map(|h| h.trim().is_empty())
        .unwrap_or(true)
    {
        warn!("parse_join_url rejected missing host");
        return Err("Invalid URL: missing host".to_string());
    }

    let room_id = parsed
        .query_pairs()
        .find(|(k, _)| k == "room")
        .map(|(_, v)| v.into_owned())
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            warn!("parse_join_url rejected missing room query parameter");
            "URL is missing the ?room= parameter".to_string()
        })?;

    let mut base_path = parsed.path().trim_end_matches('/').to_string();
    if base_path.ends_with("/ws") {
        base_path.truncate(base_path.len() - 3);
    }
    if base_path.is_empty() {
        base_path.push('/');
    }

    let mut server_url = format!("{}://{}", scheme, parsed.host_str().unwrap());
    if let Some(port) = parsed.port() {
        server_url.push(':');
        server_url.push_str(&port.to_string());
    }
    if base_path != "/" {
        server_url.push_str(&base_path);
    }

    let info = JoinInfo {
        server_url,
        room_id,
    };
    debug!(
        "parse_join_url succeeded: server_url={} room_id={}",
        info.server_url, info.room_id
    );
    Ok(info)
}
