use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crdt_core::{Document, Snapshot};
use tauri::{AppHandle, Manager};

use super::{recents, DocumentMeta, PersistError, FILE_EXTENSION, FORMAT_VERSION, MAGIC};

const FORBIDDEN_CHARS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

const RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

fn validate_name(name: &str) -> Result<(), PersistError> {
    if name.is_empty() {
        return Err(PersistError::InvalidName("name is empty".into()));
    }
    if name.contains(FORBIDDEN_CHARS) {
        return Err(PersistError::InvalidName(format!(
            "name contains forbidden characters: {name}"
        )));
    }
    if name.starts_with('.') || name.ends_with('.') || name.ends_with(' ') {
        return Err(PersistError::InvalidName(format!(
            "name has invalid leading/trailing characters: {name}"
        )));
    }
    let upper = name.to_uppercase();
    if RESERVED_NAMES.iter().any(|r| *r == upper) {
        return Err(PersistError::InvalidName(format!(
            "name is a reserved system name: {name}"
        )));
    }
    Ok(())
}

fn is_valid_name(name: &str) -> bool {
    validate_name(name).is_ok()
}

pub fn documents_dir(app: &AppHandle) -> Result<PathBuf, PersistError> {
    match app.path().document_dir() {
        Ok(docs) => Ok(docs.join("PeerCode")),
        Err(_) => fallback_documents_dir(app),
    }
}

fn fallback_documents_dir(app: &AppHandle) -> Result<PathBuf, PersistError> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| PersistError::Io(std::io::Error::other(e.to_string())))?;
    Ok(base.join("documents"))
}

pub fn doc_path(app: &AppHandle, name: &str) -> Result<PathBuf, PersistError> {
    validate_name(name)?;
    let dir = documents_dir(app)?;
    Ok(dir.join(format!("{name}.{FILE_EXTENSION}")))
}

fn build_pcdoc_bytes(payload: &[u8]) -> Vec<u8> {
    let mut data = Vec::with_capacity(MAGIC.len() + 1 + payload.len());
    data.extend_from_slice(MAGIC);
    data.push(FORMAT_VERSION);
    data.extend_from_slice(payload);
    data
}

fn atomic_write(path: &Path, data: &[u8]) -> Result<(), PersistError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tmp = path.with_extension("pcdoc.tmp");
    fs::write(&tmp, data)?;

    if let Err(e) = fs::rename(&tmp, path) {
        if cfg!(windows) && path.exists() {
            if let Err(remove_err) = fs::remove_file(path) {
                let _ = fs::remove_file(&tmp);
                return Err(PersistError::Io(remove_err));
            }
            if let Err(rename_err) = fs::rename(&tmp, path) {
                let _ = fs::remove_file(&tmp);
                return Err(PersistError::Io(rename_err));
            }
        } else {
            let _ = fs::remove_file(&tmp);
            return Err(PersistError::Io(e));
        }
    }

    Ok(())
}

pub fn save_document(path: &Path, doc: &Document) -> Result<(), PersistError> {
    let snap = doc.to_snapshot();
    let payload = snap.encode();
    let data = build_pcdoc_bytes(&payload);
    atomic_write(path, &data)
}

pub fn save_snapshot(path: &Path, snap: &Snapshot) -> Result<(), PersistError> {
    let payload = snap.encode();
    let data = build_pcdoc_bytes(&payload);
    atomic_write(path, &data)
}

pub fn load_document(path: &Path) -> Result<Document, PersistError> {
    let data = fs::read(path)?;

    if data.len() < MAGIC.len() + 1 {
        return Err(PersistError::InvalidMagic);
    }

    if &data[..4] != MAGIC {
        return Err(PersistError::InvalidMagic);
    }

    let file_version = data[4];
    if file_version != FORMAT_VERSION {
        return Err(PersistError::UnsupportedFormat(file_version));
    }

    let snap = Snapshot::decode(&data[5..])?;
    let doc = Document::from_snapshot(snap);
    Ok(doc)
}

/// Library documents plus recently used external files, deduplicated by
/// canonical path and sorted newest first.
pub fn list_documents(app: &AppHandle) -> Result<Vec<DocumentMeta>, PersistError> {
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut docs = list_library_documents(app, &mut seen)?;

    for path in recents::read_recents(app) {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !seen.insert(canonical) {
            continue;
        }
        if let Some(meta) = meta_for_path(&path, true) {
            docs.push(meta);
        }
    }

    docs.sort_by_key(|d| std::cmp::Reverse(d.modified));
    Ok(docs)
}

fn list_library_documents(
    app: &AppHandle,
    seen: &mut HashSet<PathBuf>,
) -> Result<Vec<DocumentMeta>, PersistError> {
    let dir = documents_dir(app)?;

    let iter = match fs::read_dir(&dir) {
        Ok(it) => it,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };

    let mut docs = Vec::new();
    for entry in iter {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some(FILE_EXTENSION) {
            continue;
        }
        if !path
            .file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(is_valid_name)
        {
            continue;
        }
        seen.insert(path.canonicalize().unwrap_or_else(|_| path.clone()));
        if let Some(meta) = meta_for_path(&path, false) {
            docs.push(meta);
        }
    }
    Ok(docs)
}

fn meta_for_path(path: &Path, external: bool) -> Option<DocumentMeta> {
    let is_pcdoc = path.extension().and_then(|e| e.to_str()) == Some(FILE_EXTENSION);

    let name = if is_pcdoc {
        path.file_stem()?.to_string_lossy().into_owned()
    } else {
        path.file_name()?.to_string_lossy().into_owned()
    };
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
        external,
    })
}

pub fn save_named(app: &AppHandle, name: &str, doc: &Document) -> Result<(), PersistError> {
    let path = doc_path(app, name)?;
    save_document(&path, doc)
}

pub fn load_named(app: &AppHandle, name: &str) -> Result<Document, PersistError> {
    let path = doc_path(app, name)?;
    load_document(&path)
}

pub fn save_snapshot_named(
    app: &AppHandle,
    name: &str,
    snap: &Snapshot,
) -> Result<(), PersistError> {
    let path = doc_path(app, name)?;
    save_snapshot(&path, snap)
}
