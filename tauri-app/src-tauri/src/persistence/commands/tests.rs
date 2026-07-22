use std::path::Path;

use crate::state::app_role::{AppRole, WriteAccess};
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

fn guest_role() -> AppRole {
    AppRole::Guest {
        room_id: "r".into(),
        server_url: "ws://x".into(),
        write_access: WriteAccess::Editable,
    }
}

fn host_role() -> AppRole {
    AppRole::Host {
        room_id: "r".into(),
        lan_url: None,
        public_url: None,
        local_room_url: "ws://l".into(),
        public_room_url: None,
    }
}

#[test]
fn doc_swap_denied_for_guest_regardless_of_allow_host() {
    let err = super::deny_doc_swap(&guest_role(), "open a file", true).unwrap_err();
    assert_eq!(err, "leave the session before you open a file");
    assert!(super::deny_doc_swap(&guest_role(), "fork the document", false).is_err());
}

#[test]
fn doc_swap_host_allowed_only_when_flagged() {
    assert!(super::deny_doc_swap(&host_role(), "open a file", true).is_ok());
    let err = super::deny_doc_swap(&host_role(), "reset the document", false).unwrap_err();
    assert_eq!(err, "end the session before you reset the document");
}

#[test]
fn doc_swap_allowed_out_of_session() {
    for role in [AppRole::Undecided, AppRole::Starting] {
        assert!(super::deny_doc_swap(&role, "open a file", false).is_ok());
    }
}
