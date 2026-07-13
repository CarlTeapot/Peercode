use std::path::Path;

use crdt_core::types::ClientId;
use crdt_core::Document;

use super::{name_from_path, rebuild_if_crlf};

#[test]
fn name_is_the_full_file_name() {
    assert_eq!(name_from_path(Path::new("/tmp/main.py")), "main.py");
    assert_eq!(name_from_path(Path::new("/tmp/notes.pcdoc")), "notes.pcdoc");
}

#[test]
fn name_falls_back_to_untitled() {
    assert_eq!(name_from_path(Path::new("/")), "untitled");
}

#[test]
fn rebuild_if_crlf_keeps_lf_document_intact() {
    let mut doc = Document::new(ClientId::new(7));
    doc.local_insert(0, "line one\nline two").unwrap();
    let original_sv = doc.state_vector.clone();

    let (doc, text) = rebuild_if_crlf(doc).unwrap();

    assert_eq!(text, "line one\nline two");
    assert_eq!(doc.get_text(), "line one\nline two");
    assert_eq!(doc.state_vector, original_sv);
}

#[test]
fn rebuild_if_crlf_normalizes_legacy_crlf_document() {
    let mut doc = Document::new(ClientId::new(7));
    doc.local_insert(0, "line one\r\nline two\rend").unwrap();

    let (doc, text) = rebuild_if_crlf(doc).unwrap();

    assert_eq!(text, "line one\nline two\nend");
    assert_eq!(doc.get_text(), "line one\nline two\nend");
    assert_eq!(doc.client_id, ClientId::new(7));
}
