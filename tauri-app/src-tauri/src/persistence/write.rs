use std::path::Path;

use super::atomic::atomic_write;
use super::PersistError;

pub fn write_text_file(path: &Path, text: &str, use_crlf: bool) -> Result<(), PersistError> {
    if use_crlf {
        atomic_write(path, text.replace('\n', "\r\n").as_bytes())
    } else {
        atomic_write(path, text.as_bytes())
    }
}
