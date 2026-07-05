use std::fs;

use super::read::{read_file, FileContent};
use super::read_tests::TempFile;
use super::write::write_text_file;

#[test]
fn writes_lf_text_verbatim() {
    let file = TempFile::new(b"");
    write_text_file(&file.0, "a\nb\n", false).unwrap();
    assert_eq!(fs::read(&file.0).unwrap(), b"a\nb\n");
}

#[test]
fn restores_crlf_when_the_file_had_it() {
    let file = TempFile::new(b"");
    write_text_file(&file.0, "a\nb\n", true).unwrap();
    assert_eq!(fs::read(&file.0).unwrap(), b"a\r\nb\r\n");
}

#[test]
fn crlf_file_round_trips_byte_identical() {
    let file = TempFile::new(b"one\r\ntwo\r\n");
    let (text, had_crlf) = match read_file(&file.0).unwrap() {
        FileContent::Text { text, had_crlf } => (text, had_crlf),
        FileContent::Pcdoc(_) => panic!("expected text content"),
    };
    write_text_file(&file.0, &text, had_crlf).unwrap();
    assert_eq!(fs::read(&file.0).unwrap(), b"one\r\ntwo\r\n");
}
