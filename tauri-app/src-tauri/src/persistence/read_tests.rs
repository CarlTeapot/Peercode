use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use crdt_core::types::ClientId;
use crdt_core::Document;

use super::pcdoc;
use super::read::{read_file, FileContent};
use super::PersistError;

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Temp file that removes itself on drop, so failed asserts don't leak files.
pub(super) struct TempFile(pub(super) PathBuf);

impl TempFile {
    pub(super) fn new(contents: &[u8]) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("peercode-read-test-{}-{n}.bin", std::process::id()));
        fs::write(&path, contents).unwrap();
        TempFile(path)
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn text_of(content: FileContent) -> (String, bool) {
    match content {
        FileContent::Text { text, had_crlf } => (text, had_crlf),
        FileContent::Pcdoc(_) => panic!("expected text content"),
    }
}

#[test]
fn reads_utf8_text_without_crlf() {
    let file = TempFile::new(b"fn main() {}\n");
    let (text, had_crlf) = text_of(read_file(&file.0).unwrap());
    assert_eq!(text, "fn main() {}\n");
    assert!(!had_crlf);
}

#[test]
fn normalizes_crlf_and_cr_and_remembers_crlf() {
    let file = TempFile::new(b"one\r\ntwo\rthree\n");
    let (text, had_crlf) = text_of(read_file(&file.0).unwrap());
    assert_eq!(text, "one\ntwo\nthree\n");
    assert!(had_crlf);
}

#[test]
fn cr_only_files_do_not_count_as_crlf() {
    let file = TempFile::new(b"one\rtwo");
    let (text, had_crlf) = text_of(read_file(&file.0).unwrap());
    assert_eq!(text, "one\ntwo");
    assert!(!had_crlf);
}

#[test]
fn pcdoc_bytes_decode_to_a_document() {
    let mut doc = Document::new(ClientId::new(7));
    doc.local_insert(0, "hello").unwrap();
    let file = TempFile::new(b"");
    pcdoc::save_snapshot(&file.0, &doc.to_snapshot()).unwrap();
    match read_file(&file.0).unwrap() {
        FileContent::Pcdoc(loaded) => assert_eq!(loaded.get_text(), "hello"),
        FileContent::Text { .. } => panic!("expected pcdoc content"),
    }
}

#[test]
fn rejects_non_utf8_content() {
    let file = TempFile::new(&[0xff, 0xfe, 0x00, 0x41]);
    assert!(matches!(read_file(&file.0), Err(PersistError::NotUtf8)));
}

#[test]
fn missing_file_is_an_io_error() {
    let path = std::env::temp_dir().join("peercode-read-test-does-not-exist");
    assert!(matches!(read_file(&path), Err(PersistError::Io(_))));
}
