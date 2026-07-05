use std::path::Path;

use crdt_core::{Document, Snapshot};

use super::atomic::atomic_write;
use super::{PersistError, FORMAT_VERSION, MAGIC};

pub fn save_snapshot(path: &Path, snap: &Snapshot) -> Result<(), PersistError> {
    let payload = snap.encode();
    let mut data = Vec::with_capacity(MAGIC.len() + 1 + payload.len());
    data.extend_from_slice(MAGIC);
    data.push(FORMAT_VERSION);
    data.extend_from_slice(&payload);
    atomic_write(path, &data)
}

/// Decodes the raw bytes of a .pcdoc file (magic + version + snapshot).
pub fn decode(data: &[u8]) -> Result<Document, PersistError> {
    if data.len() < MAGIC.len() + 1 || &data[..MAGIC.len()] != MAGIC {
        return Err(PersistError::InvalidMagic);
    }
    let file_version = data[MAGIC.len()];
    if file_version != FORMAT_VERSION {
        return Err(PersistError::UnsupportedFormat(file_version));
    }
    let snap = Snapshot::decode(&data[MAGIC.len() + 1..])?;
    Ok(Document::from_snapshot(snap))
}
