pub const SESSION_READY: &str = "session://session-ready";
pub const SESSION_ERROR: &str = "session://session-error";
pub const SESSION_ENDED: &str = "session://session-ended";

#[derive(Clone, serde::Serialize)]
pub struct SessionEndedPayload {}

#[derive(Clone, serde::Serialize)]
pub struct SessionReadyPayload {
    pub lan_url: Option<String>,
    pub public_url: Option<String>,
    pub local_room_url: String,
    pub public_room_url: Option<String>,
    pub room_id: String,
    pub port: u16,
}

#[derive(Clone, serde::Serialize)]
pub struct SessionErrorPayload {
    pub message: String,
}

#[derive(serde::Serialize)]
pub struct SessionInfo {
    pub status: String,
    pub lan_url: Option<String>,
    pub public_url: Option<String>,
    pub local_room_url: Option<String>,
    pub public_room_url: Option<String>,
    pub room_id: Option<String>,
}

#[derive(serde::Serialize)]
pub struct JoinInfo {
    pub server_url: String,
    pub room_id: String,
}
