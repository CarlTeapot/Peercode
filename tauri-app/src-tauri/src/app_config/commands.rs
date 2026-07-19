use log::{debug, info};
use tauri::AppHandle;

use crate::app_config::identity::{self, PersistedIdentity};

#[derive(serde::Serialize)]
pub struct IdentityDto {
    pub username: Option<String>,
}

#[tauri::command]
pub fn get_identity(app: AppHandle) -> Result<IdentityDto, String> {
    let id = identity::load_raw(&app)?;
    debug!(
        "get_identity completed: has_username={}",
        id.username.is_some()
    );
    Ok(IdentityDto {
        username: id.username,
    })
}

#[tauri::command]
pub fn set_username(app: AppHandle, username: String) -> Result<(), String> {
    info!(
        "set_username requested: input_len={}",
        username.chars().count()
    );
    let clean = identity::sanitize_username(&username).ok_or("username must not be empty")?;
    identity::persist(
        &app,
        &PersistedIdentity {
            username: Some(clean),
        },
    )
}
