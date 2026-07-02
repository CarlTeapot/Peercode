use crate::document::Document;
use crate::document::from_text::char_chunks;
use crate::types::ClientId;

fn block_lens(doc: &Document) -> Vec<u64> {
    let mut lens = Vec::new();
    let mut curr = doc.head;
    while let Some(id) = curr {
        let block = doc.store.get(&id).expect("linked block must exist");
        lens.push(block.len);
        curr = block.right();
    }
    lens
}

#[test]
fn empty_text_builds_empty_document() {
    let doc = Document::from_text_chunked(ClientId::new(1), "", 10).unwrap();
    assert_eq!(doc.get_text(), "");
    assert!(block_lens(&doc).is_empty());
}

#[test]
fn text_round_trips_and_no_block_exceeds_max_chars() {
    let text = "fn main() {\n    println!(\"hello world\");\n}\n";
    let doc = Document::from_text_chunked(ClientId::new(1), text, 10).unwrap();
    assert_eq!(doc.get_text(), text);
    assert!(block_lens(&doc).iter().all(|&len| len <= 10));
}

#[test]
fn exact_multiple_of_chunk_size_splits_evenly() {
    let text = "a".repeat(30);
    let doc = Document::from_text_chunked(ClientId::new(1), &text, 10).unwrap();
    assert_eq!(block_lens(&doc), vec![10, 10, 10]);
}

#[test]
fn multibyte_text_chunks_on_char_boundaries() {
    let text = "héllo wörld 🦀🦀🦀 ważne teksty";
    let doc = Document::from_text_chunked(ClientId::new(1), text, 10).unwrap();
    assert_eq!(doc.get_text(), text);
    assert!(block_lens(&doc).iter().all(|&len| len <= 10));
}

#[test]
fn imported_document_stays_editable() {
    let doc = &mut Document::from_text_chunked(ClientId::new(1), "0123456789ABCDEF", 10).unwrap();
    doc.local_insert(10, "-").unwrap();
    assert_eq!(doc.get_text(), "0123456789-ABCDEF");
}

#[test]
fn char_chunks_counts_chars_not_bytes() {
    let chunks: Vec<&str> = char_chunks("ééééé", 2).collect();
    assert_eq!(chunks, vec!["éé", "éé", "é"]);
}

#[test]
fn char_chunks_handles_empty_and_short_input() {
    assert_eq!(char_chunks("", 10).count(), 0);
    let chunks: Vec<&str> = char_chunks("hi", 10).collect();
    assert_eq!(chunks, vec!["hi"]);
}
