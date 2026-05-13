use crate::gateway::types::{RoomResponse, GATEWAY_TIMEOUT};
use log::{debug, warn};
use url::Url;

pub async fn create_room(port: u16, auth_token: &str) -> Result<String, String> {
    debug!("fetching gateway room id via /rooms: port={port}");
    tokio::time::timeout(GATEWAY_TIMEOUT, async {
        reqwest::Client::new()
            .post(format!("http://127.0.0.1:{port}/rooms"))
            .bearer_auth(auth_token)
            .send()
            .await
            .map_err(|e| format!("gateway /rooms: {e}"))?
            .error_for_status()
            .map_err(|e| format!("gateway /rooms: {e}"))?
            .json::<RoomResponse>()
            .await
            .map(|r| r.room_id)
            .map_err(|e| format!("gateway /rooms: {e}"))
    })
    .await
    .map_err(|_| {
        format!(
            "gateway /rooms: timed out after {}s",
            GATEWAY_TIMEOUT.as_secs()
        )
    })?
}

pub async fn destroy_room(local_room_url: String, auth_token: &str) -> Result<(), String> {
    let end_session_url = end_session_url_from_room_url(&local_room_url)?;
    tokio::time::timeout(GATEWAY_TIMEOUT, async {
        reqwest::Client::new()
            .post(&end_session_url)
            .bearer_auth(auth_token)
            .send()
            .await
            .map_err(|e| {
                warn!("end-session HTTP request failed: {e}");
                format!("Failed to notify gateway: {e}")
            })?
            .error_for_status()
            .map_err(|e| {
                warn!("end-session HTTP non-OK response: {e}");
                format!("Gateway rejected end-session: {e}")
            })?;
        Ok::<(), String>(())
    })
    .await
    .map_err(|_| {
        format!(
            "gateway /end-session: timed out after {}s",
            GATEWAY_TIMEOUT.as_secs()
        )
    })??;
    Ok(())
}

fn end_session_url_from_room_url(room_url: &str) -> Result<String, String> {
    let parsed = Url::parse(room_url).map_err(|e| format!("invalid room URL: {e}"))?;
    let room_id = parsed
        .query_pairs()
        .find(|(k, _)| k == "room")
        .map(|(_, v)| v.into_owned())
        .ok_or_else(|| "room URL is missing room query parameter".to_string())?;
    let port = parsed
        .port_or_known_default()
        .ok_or_else(|| "room URL is missing a valid port".to_string())?;
    Ok(format!(
        "http://127.0.0.1:{port}/end-session?room={room_id}"
    ))
}
