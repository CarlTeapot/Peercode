pub mod commands;
mod io;
mod recents;

use crdt_core::SnapshotError;
use serde::Serialize;

pub const MAGIC: &[u8; 4] = b"PCDC";
pub const FORMAT_VERSION: u8 = 1;
pub const FILE_EXTENSION: &str = "pcdoc";

#[derive(Debug)]
pub enum PersistError {
    Io(std::io::Error),
    InvalidMagic,
    UnsupportedFormat(u8),
    Snapshot(SnapshotError),
    InvalidName(String),
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
            PersistError::InvalidName(msg) => write!(f, "invalid document name: {msg}"),
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
    pub path: String,
    pub size_bytes: u64,
    pub modified: Option<u64>,
    pub external: bool,
}

pub use io::{
    doc_path, documents_dir, list_documents, load_document, load_named, migrate_legacy_documents,
    save_named, save_snapshot, save_snapshot_named,
};
pub use recents::record_recent;
