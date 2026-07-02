use super::Document;
use crate::error::DocumentError;
use crate::types::ClientId;

impl Document {
    pub fn from_text_chunked(
        client_id: ClientId,
        text: &str,
        max_chars: usize,
    ) -> Result<Self, DocumentError> {
        assert!(max_chars > 0, "max_chars must be at least 1");
        let mut doc = Document::new(client_id);
        let mut position: u64 = 0;
        for chunk in char_chunks(text, max_chars) {
            doc.local_insert(position, chunk)?;
            position += chunk.chars().count() as u64;
        }
        Ok(doc)
    }
}


pub(super) fn char_chunks(text: &str, max_chars: usize) -> impl Iterator<Item = &str> {
    let mut rest = text;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let split = rest
            .char_indices()
            .nth(max_chars)
            .map(|(i, _)| i)
            .unwrap_or(rest.len());
        let (chunk, tail) = rest.split_at(split);
        rest = tail;
        Some(chunk)
    })
}
