use std::fs;
use std::path::PathBuf;

use log::info;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

const IDENTITY_FILE: &str = "identity.toml";
const MAX_USERNAME_LEN: usize = 32;

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct PersistedIdentity {
    pub(super) username: Option<String>,
}

fn identity_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| format!("could not resolve app data dir: {e}"))?
        .join(IDENTITY_FILE))
}

pub(super) fn load_raw(app: &AppHandle) -> Result<PersistedIdentity, String> {
    let path = identity_path(app)?;
    Ok(fs::read_to_string(&path)
        .ok()
        .and_then(|s| toml::from_str::<PersistedIdentity>(&s).ok())
        .unwrap_or(PersistedIdentity { username: None }))
}

pub(super) fn persist(app: &AppHandle, identity: &PersistedIdentity) -> Result<(), String> {
    let path = identity_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_file_name(".identity.toml.tmp");
    let serialized = toml::to_string_pretty(identity).map_err(|e| e.to_string())?;
    fs::write(&tmp, serialized).map_err(|e| e.to_string())?;
    fs::rename(tmp, path)
        .map_err(|e| e.to_string())
        .map(|_| info!("identity persisted successfully"))
}

pub fn sanitize_username(raw: &str) -> Option<String> {
    let cleaned: String = raw.chars().filter(|c| !c.is_control()).collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.chars().take(MAX_USERNAME_LEN).collect())
}

pub fn read_username(app: &AppHandle) -> String {
    load_raw(app)
        .ok()
        .and_then(|id| id.username)
        .unwrap_or_default()
}
