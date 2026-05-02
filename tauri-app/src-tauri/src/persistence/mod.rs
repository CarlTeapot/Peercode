pub mod commands;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crdt_core::{Document, Snapshot, SnapshotError};
use serde::Serialize;
use tauri::{AppHandle, Manager};

const MAGIC: &[u8; 4] = b"PCDC";
const FORMAT_VERSION: u8 = 1;
const FILE_EXTENSION: &str = "pcdoc";

#[derive(Debug)]
pub enum PersistError {
    Io(std::io::Error),
    InvalidMagic,
    UnsupportedFormat(u8),
    Snapshot(SnapshotError),
}

impl std::fmt::Display for PersistError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistError::Io(e) => write!(f, "I/O error: {e}"),
            PersistError::InvalidMagic => write!(f, "not a valid .pcdoc file"),
            PersistError::UnsupportedFormat(v) => {
                write!(f, "unsupported file format version: {v}")
            }
            PersistError::Snapshot(e) => write!(f, "snapshot error: {e}"),
        }
    }
}

impl From<std::io::Error> for PersistError {
    fn from(e: std::io::Error) -> Self {
        PersistError::Io(e)
    }
}

impl From<SnapshotError> for PersistError {
    fn from(e: SnapshotError) -> Self {
        PersistError::Snapshot(e)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DocumentMeta {
    pub name: String,
    pub size_bytes: u64,
    pub modified: Option<u64>,
}

pub fn documents_dir(app: &AppHandle) -> Result<PathBuf, PersistError> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| PersistError::Io(std::io::Error::other(e.to_string())))?;
    Ok(base.join("documents"))
}

fn doc_path(app: &AppHandle, name: &str) -> Result<PathBuf, PersistError> {
    let dir = documents_dir(app)?;
    Ok(dir.join(format!("{name}.{FILE_EXTENSION}")))
}

pub fn save_document(path: &Path, doc: &Document) -> Result<(), PersistError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let snap = doc.to_snapshot();
    let payload = snap.encode();

    let mut data = Vec::with_capacity(MAGIC.len() + 1 + payload.len());
    data.extend_from_slice(MAGIC);
    data.push(FORMAT_VERSION);
    data.extend_from_slice(&payload);

    let tmp = path.with_extension("pcdoc.tmp");
    fs::write(&tmp, &data)?;
    fs::rename(tmp, path)?;
    Ok(())
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
    let doc = Document::from_snapshot(snap)?;
    Ok(doc)
}

pub fn list_documents(app: &AppHandle) -> Result<Vec<DocumentMeta>, PersistError> {
    let dir = documents_dir(app)?;
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut docs = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some(FILE_EXTENSION) {
            continue;
        }

        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let metadata = entry.metadata()?;
        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());

        docs.push(DocumentMeta {
            name,
            size_bytes: metadata.len(),
            modified,
        });
    }

    docs.sort_by_key(|d| std::cmp::Reverse(d.modified));
    Ok(docs)
}

pub fn save_named(app: &AppHandle, name: &str, doc: &Document) -> Result<(), PersistError> {
    let path = doc_path(app, name)?;
    save_document(&path, doc)
}

pub fn load_named(app: &AppHandle, name: &str) -> Result<Document, PersistError> {
    let path = doc_path(app, name)?;
    load_document(&path)
}

pub fn save_snapshot(path: &Path, snap: &Snapshot) -> Result<(), PersistError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let payload = snap.encode();
    let mut data = Vec::with_capacity(MAGIC.len() + 1 + payload.len());
    data.extend_from_slice(MAGIC);
    data.push(FORMAT_VERSION);
    data.extend_from_slice(&payload);

    let tmp = path.with_extension("pcdoc.tmp");
    fs::write(&tmp, &data)?;
    fs::rename(tmp, path)?;
    Ok(())
}

pub fn save_snapshot_named(
    app: &AppHandle,
    name: &str,
    snap: &Snapshot,
) -> Result<(), PersistError> {
    let path = doc_path(app, name)?;
    save_snapshot(&path, snap)
}
