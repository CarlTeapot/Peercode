use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use super::text_import::read_text_for_import;
use super::{PersistError, MAGIC};

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Temp file that removes itself on drop, so failed asserts don't leak files.
struct TempFile(PathBuf);

impl TempFile {
    fn new(contents: &[u8]) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "peercode-import-test-{}-{n}.bin",
            std::process::id()
        ));
        fs::write(&path, contents).unwrap();
        TempFile(path)
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[test]
fn reads_utf8_text() {
    let file = TempFile::new("fn main() {}\n".as_bytes());
    assert_eq!(read_text_for_import(&file.0).unwrap(), "fn main() {}\n");
}

#[test]
fn rejects_pcdoc_payloads() {
    let mut bytes = MAGIC.to_vec();
    bytes.push(1);
    let file = TempFile::new(&bytes);
    assert!(matches!(
        read_text_for_import(&file.0),
        Err(PersistError::IsPcdoc)
    ));
}

#[test]
fn rejects_non_utf8_content() {
    let file = TempFile::new(&[0xff, 0xfe, 0x00, 0x41]);
    assert!(matches!(
        read_text_for_import(&file.0),
        Err(PersistError::NotUtf8)
    ));
}

#[test]
fn normalizes_crlf_and_cr_to_lf() {
    let file = TempFile::new(b"one\r\ntwo\rthree\n");
    assert_eq!(read_text_for_import(&file.0).unwrap(), "one\ntwo\nthree\n");
}

#[test]
fn missing_file_is_an_io_error() {
    let path = std::env::temp_dir().join("peercode-import-test-does-not-exist");
    assert!(matches!(
        read_text_for_import(&path),
        Err(PersistError::Io(_))
    ));
}
