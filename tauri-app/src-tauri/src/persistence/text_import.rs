use std::fs;
use std::path::Path;

use super::{PersistError, MAGIC};

pub const MAX_IMPORT_BYTES: u64 = 10 * 1024 * 1024;

pub const IMPORT_CHUNK_CHARS: usize = 64;

pub fn read_text_for_import(path: &Path) -> Result<String, PersistError> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_IMPORT_BYTES {
        return Err(PersistError::TooLarge(MAX_IMPORT_BYTES));
    }
    let bytes = fs::read(path)?;
    if bytes.starts_with(MAGIC) {
        return Err(PersistError::IsPcdoc);
    }
    let text = String::from_utf8(bytes).map_err(|_| PersistError::NotUtf8)?;
    Ok(normalize_line_endings(&text))
}

fn normalize_line_endings(text: &str) -> String {
    if !text.contains('\r') {
        return text.to_string();
    }
    text.replace("\r\n", "\n").replace('\r', "\n")
}
