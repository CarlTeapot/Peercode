use std::fs;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use super::PersistError;

const RECENTS_FILE: &str = "recent.json";
const MAX_RECENTS: usize = 50;

fn recents_path(app: &AppHandle) -> Result<PathBuf, PersistError> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| PersistError::Io(std::io::Error::other(e.to_string())))?;
    Ok(base.join(RECENTS_FILE))
}

/// Recently used paths, most recent first.
pub fn read_recents(app: &AppHandle) -> Vec<PathBuf> {
    let Ok(path) = recents_path(app) else {
        return Vec::new();
    };
    let Ok(data) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let paths: Vec<PathBuf> = serde_json::from_str(&data).unwrap_or_default();
    paths.into_iter().filter(|p| p.exists()).collect()
}

pub fn record_recent(app: &AppHandle, path: &Path) -> Result<(), PersistError> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let recents = push_front_dedup(read_recents(app), canonical);
    write_recents(app, &recents)
}

fn write_recents(app: &AppHandle, recents: &[PathBuf]) -> Result<(), PersistError> {
    let path = recents_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(recents)
        .map_err(|e| PersistError::Io(std::io::Error::other(e)))?;
    fs::write(&path, data)?;
    Ok(())
}

fn push_front_dedup(mut recents: Vec<PathBuf>, entry: PathBuf) -> Vec<PathBuf> {
    recents.retain(|p| *p != entry);
    recents.insert(0, entry);
    recents.truncate(MAX_RECENTS);
    recents
}
