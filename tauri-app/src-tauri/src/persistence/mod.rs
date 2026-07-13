mod atomic;
pub mod commands;
mod paths;
mod pcdoc;
mod read;
#[cfg(test)]
mod read_tests;
mod recents;
#[cfg(test)]
mod recents_tests;
mod write;
#[cfg(test)]
mod write_tests;

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
    NotUtf8,
    TooLarge(u64),
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
            PersistError::NotUtf8 => write!(f, "file is not readable UTF-8 text"),
            PersistError::TooLarge(max) => {
                write!(f, "file is larger than the {} MB limit", max >> 20)
            }
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

/// A recent file, as shown in the Open list.
#[derive(Debug, Clone, Serialize)]
pub struct DocumentMeta {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub modified: Option<u64>,
}

pub use paths::documents_dir;
pub use pcdoc::save_snapshot;
pub use read::{read_file, FileContent, OPEN_CHUNK_CHARS};
pub use recents::{list_recent_meta, record_recent, remove_recent};
pub use write::write_text_file;
