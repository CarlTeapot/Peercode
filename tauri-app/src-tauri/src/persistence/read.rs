use std::fs;
use std::path::Path;

use crdt_core::Document;

use super::{pcdoc, PersistError};

pub const MAX_OPEN_BYTES: u64 = 10 * 1024 * 1024;

pub const OPEN_CHUNK_CHARS: usize = 64;

pub enum FileContent {
    Pcdoc(Box<Document>),
    Text { text: String, had_crlf: bool },
}

pub fn read_file(path: &Path) -> Result<FileContent, PersistError> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_OPEN_BYTES {
        return Err(PersistError::TooLarge(MAX_OPEN_BYTES));
    }
    let bytes = fs::read(path)?;
    if bytes.starts_with(super::MAGIC) {
        return Ok(FileContent::Pcdoc(Box::new(pcdoc::decode(&bytes)?)));
    }
    let text = String::from_utf8(bytes).map_err(|_| PersistError::NotUtf8)?;
    let had_crlf = text.contains("\r\n");
    Ok(FileContent::Text {
        text: normalize_line_endings(&text),
        had_crlf,
    })
}

fn normalize_line_endings(text: &str) -> String {
    if !text.contains('\r') {
        return text.to_string();
    }
    text.replace("\r\n", "\n").replace('\r', "\n")
}
