use std::time::Duration;

pub const GATEWAY_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(serde::Deserialize)]
pub struct RoomResponse {
    pub(crate) room_id: String,
}
