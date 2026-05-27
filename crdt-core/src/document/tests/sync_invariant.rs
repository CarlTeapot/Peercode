use crate::document::Document;
use crate::store::DeleteSet;
use crate::structs::Block;
use crate::types::ClientId;
use crate::wire::WireBlock;

struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Rng { state: seed }
    }
    fn next(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.state
    }
    fn range(&mut self, max: u64) -> u64 {
        if max == 0 { 0 } else { self.next() % max }
    }
}

const ALPHABET: &[char] = &['a', 'b', 'c', 'd', 'e', ' ', '\n'];

fn random_string(rng: &mut Rng) -> String {
    let len = (rng.range(4) + 1) as usize;
    (0..len)
        .map(|_| ALPHABET[(rng.next() % ALPHABET.len() as u64) as usize])
        .collect()
}

fn local_insert(doc: &mut Document, rng: &mut Rng, outbox: &mut Vec<WireBlock>) {
    let visible = doc.position_index.visible_len();
    let pos = rng.range(visible + 1);
    let content = random_string(rng);
    if let Some(wire) = doc.local_insert(pos, &content).unwrap() {
        outbox.push(wire);
    }
}

fn local_delete(doc: &mut Document, rng: &mut Rng, outbox: &mut Vec<DeleteSet>) {
    let visible = doc.position_index.visible_len();
    if visible == 0 {
        return;
    }
    let pos = rng.range(visible);
    let max_len = (visible - pos).min(4);
    let len = rng.range(max_len) + 1;
    let ds = doc.delete(pos, len).unwrap();
    if !ds.is_empty() {
        outbox.push(ds);
    }
}

fn sync(src_blocks: &mut Vec<WireBlock>, src_ds: &mut Vec<DeleteSet>, dst: &mut Document) {
    for wire in src_blocks.drain(..) {
        dst.remote_insert(Block::from(wire)).unwrap();
    }
    for ds in src_ds.drain(..) {
        dst.apply_delete_set(&ds).unwrap();
    }
}

#[test]
fn insert_at_end_of_first_line_goes_on_line_one() {
    let mut doc = Document::new(ClientId::new(1));
    doc.local_insert(0, "line1\nline2").unwrap();
    assert_eq!(doc.get_text(), "line1\nline2");
    doc.local_insert(5, "X").unwrap();
    assert_eq!(
        doc.get_text(),
        "line1X\nline2",
        "typing at end-of-line-1 must stay on line 1"
    );
}

#[test]
fn insert_at_end_of_first_line_then_propagate_to_guest() {
    let mut host = Document::new(ClientId::new(1));
    let mut guest = Document::new(ClientId::new(2));

    let w0 = host.local_insert(0, "line1\nline2").unwrap().expect("wire");
    guest.remote_insert(Block::from(w0)).unwrap();
    assert_eq!(guest.get_text(), "line1\nline2");

    let w1 = guest.local_insert(5, "X").unwrap().expect("wire");
    assert_eq!(guest.get_text(), "line1X\nline2");

    host.remote_insert(Block::from(w1)).unwrap();
    assert_eq!(host.get_text(), "line1X\nline2");
}

#[test]
fn type_char_by_char_then_insert_at_end_of_first_line() {
    let mut doc = Document::new(ClientId::new(1));
    let text = "line1\nline2";
    for (i, c) in text.chars().enumerate() {
        doc.local_insert(i as u64, &c.to_string()).unwrap();
    }
    assert_eq!(doc.get_text(), "line1\nline2");

    doc.local_insert(5, "X").unwrap();
    assert_eq!(doc.get_text(), "line1X\nline2");
}

#[test]
fn type_char_by_char_then_insert_multiple_on_first_line() {
    let mut doc = Document::new(ClientId::new(1));
    let text = "line1\nline2";
    for (i, c) in text.chars().enumerate() {
        doc.local_insert(i as u64, &c.to_string()).unwrap();
    }
    let extra = " more";
    for (i, c) in extra.chars().enumerate() {
        doc.local_insert(5 + i as u64, &c.to_string()).unwrap();
    }
    assert_eq!(doc.get_text(), "line1 more\nline2");
}

#[test]
fn guest_types_on_first_line_after_remote_two_lines() {
    let mut host = Document::new(ClientId::new(1));
    let mut guest = Document::new(ClientId::new(2));

    let text = "line1\nline2";
    let mut wires: Vec<WireBlock> = Vec::new();
    for (i, c) in text.chars().enumerate() {
        let w = host
            .local_insert(i as u64, &c.to_string())
            .unwrap()
            .expect("wire");
        wires.push(w);
    }
    for w in wires {
        guest.remote_insert(Block::from(w)).unwrap();
    }
    assert_eq!(guest.get_text(), "line1\nline2");

    let extra = " more";
    let mut guest_wires: Vec<WireBlock> = Vec::new();
    for (i, c) in extra.chars().enumerate() {
        let w = guest
            .local_insert(5 + i as u64, &c.to_string())
            .unwrap()
            .expect("wire");
        guest_wires.push(w);
    }
    assert_eq!(guest.get_text(), "line1 more\nline2");

    for w in guest_wires {
        host.remote_insert(Block::from(w)).unwrap();
    }
    assert_eq!(host.get_text(), "line1 more\nline2");
}

#[test]
fn type_on_first_line_after_snapshot_load() {
    let mut host = Document::new(ClientId::new(1));
    let text = "line1\nline2";
    for (i, c) in text.chars().enumerate() {
        host.local_insert(i as u64, &c.to_string()).unwrap();
    }
    assert_eq!(host.get_text(), text);

    let mut joiner = host.fork(ClientId::new(2));
    assert_eq!(joiner.get_text(), text);

    for (i, c) in " more".chars().enumerate() {
        joiner.local_insert(5 + i as u64, &c.to_string()).unwrap();
    }
    assert_eq!(joiner.get_text(), "line1 more\nline2");
}

#[test]
fn type_in_middle_of_first_line_after_snapshot_load() {
    let mut host = Document::new(ClientId::new(1));
    let text = "line1\nline2";
    for (i, c) in text.chars().enumerate() {
        host.local_insert(i as u64, &c.to_string()).unwrap();
    }
    let mut joiner = host.fork(ClientId::new(2));
    joiner.local_insert(2, "X").unwrap();
    assert_eq!(joiner.get_text(), "liXne1\nline2");
    joiner.local_insert(3, "Y").unwrap();
    assert_eq!(joiner.get_text(), "liXYne1\nline2");
}

#[test]
fn host_typing_chars_one_by_one_guest_text_matches() {
    let mut host = Document::new(ClientId::new(1));
    let mut guest = Document::new(ClientId::new(2));

    let text = "I don't think position rendering is working lol";
    let mut wires = Vec::new();
    for (i, c) in text.chars().enumerate() {
        let wire = host
            .local_insert(i as u64, &c.to_string())
            .unwrap()
            .expect("insert produces a wire");
        wires.push(wire);
    }

    let mut received_positions = Vec::new();
    for wire in wires {
        let changes = guest.remote_insert(Block::from(wire)).unwrap();
        for c in &changes {
            if let crate::document::RemoteChange::Insert { position, content } = c {
                received_positions.push((*position, content.clone()));
            }
        }
    }

    println!("Host text: {:?}", host.get_text());
    println!("Guest text: {:?}", guest.get_text());
    if guest.get_text() != text {
        println!("Positions reported to frontend: {:?}", received_positions);
    }
    assert_eq!(host.get_text(), text, "host should have the text");
    assert_eq!(guest.get_text(), text, "guest must match host");
}

#[test]
fn sync_invariant_holds_under_random_ops() {
    let mut doc_a = Document::new(ClientId::new(1));
    let mut doc_b = Document::new(ClientId::new(2));

    let mut a_to_b_blocks: Vec<WireBlock> = Vec::new();
    let mut b_to_a_blocks: Vec<WireBlock> = Vec::new();
    let mut a_to_b_ds: Vec<DeleteSet> = Vec::new();
    let mut b_to_a_ds: Vec<DeleteSet> = Vec::new();

    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_BABE);

    for _ in 0..200 {
        match rng.range(10) {
            0..=2 => local_insert(&mut doc_a, &mut rng, &mut a_to_b_blocks),
            3..=5 => local_insert(&mut doc_b, &mut rng, &mut b_to_a_blocks),
            6 => local_delete(&mut doc_a, &mut rng, &mut a_to_b_ds),
            7 => local_delete(&mut doc_b, &mut rng, &mut b_to_a_ds),
            8 => sync(&mut a_to_b_blocks, &mut a_to_b_ds, &mut doc_b),
            9 => sync(&mut b_to_a_blocks, &mut b_to_a_ds, &mut doc_a),
            _ => unreachable!(),
        }

        doc_a.assert_index_matches_linked_list();
        doc_b.assert_index_matches_linked_list();
    }

    sync(&mut a_to_b_blocks, &mut a_to_b_ds, &mut doc_b);
    sync(&mut b_to_a_blocks, &mut b_to_a_ds, &mut doc_a);

    doc_a.assert_index_matches_linked_list();
    doc_b.assert_index_matches_linked_list();
}
