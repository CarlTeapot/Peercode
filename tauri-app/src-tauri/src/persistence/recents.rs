use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use tauri::{AppHandle, Manager};

use super::{DocumentMeta, PersistError};

const RECENTS_FILE: &str = "recent.json";
const MAX_RECENTS: usize = 50;

fn recents_path(app: &AppHandle) -> Result<PathBuf, PersistError> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| PersistError::Io(std::io::Error::other(e.to_string())))?;
    Ok(base.join(RECENTS_FILE))
}

/// Every stored path, including ones that no longer exist.
fn read_all(app: &AppHandle) -> Vec<PathBuf> {
    let Ok(path) = recents_path(app) else {
        return Vec::new();
    };
    let Ok(data) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn record_recent(app: &AppHandle, path: &Path) -> Result<(), PersistError> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let recents = push_front_dedup(read_all(app), canonical);
    write_recents(app, &recents)
}

/// Removes one path from the list
pub fn remove_recent(app: &AppHandle, path: &Path) -> Result<(), PersistError> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut recents = read_all(app);
    recents.retain(|p| p != path && *p != canonical);
    write_recents(app, &recents)
}

/// Recents that still exist, most recent first
pub fn list_recent_meta(app: &AppHandle) -> Vec<DocumentMeta> {
    let all = read_all(app);
    let live: Vec<PathBuf> = all.iter().filter(|p| p.exists()).cloned().collect();
    if live.len() != all.len() {
        let _ = write_recents(app, &live);
    }
    live.iter().filter_map(|p| meta_for_path(p)).collect()
}

pub(super) fn meta_for_path(path: &Path) -> Option<DocumentMeta> {
    let name = path.file_name()?.to_string_lossy().into_owned();
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    Some(DocumentMeta {
        name,
        path: path.to_string_lossy().into_owned(),
        size_bytes: metadata.len(),
        modified,
    })
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

pub(super) fn push_front_dedup(mut recents: Vec<PathBuf>, entry: PathBuf) -> Vec<PathBuf> {
    recents.retain(|p| *p != entry);
    recents.insert(0, entry);
    recents.truncate(MAX_RECENTS);
    recents
}
