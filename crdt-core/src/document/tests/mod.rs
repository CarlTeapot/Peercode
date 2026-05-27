use super::{Document, RemoteChange};
use crate::error::DocumentError;
use crate::structs::Block;
use crate::types::{BlockId, ClientId, Clock};
use crate::wire::WireBlock;

mod sync_invariant;

fn block_id(client: u64, clock: u64) -> BlockId {
    BlockId::new(ClientId::new(client), Clock::new(clock))
}

fn doc_with_single_block(content: &str) -> (Document, BlockId) {
    let client_id = ClientId::new(1);
    let id = BlockId::new(client_id, Clock::new(0));
    let mut doc = Document::new(client_id);
    let block = Block::new(id, None, None, content.to_string());
    let len = block.len;
    doc.head = Some(id);
    doc.store.insert(block);
    doc.position_index.insert_after(None, id, len);
    (doc, id)
}

fn doc_with_two_blocks(left: &str, right: &str) -> (Document, BlockId, BlockId) {
    let client_id = ClientId::new(1);
    let left_id = BlockId::new(client_id, Clock::new(0));
    let right_id = left_id.at_offset(left.chars().count() as u64);
    let mut doc = Document::new(client_id);

    let mut left_block = Block::new(left_id, None, Some(right_id), left.to_string());
    left_block.set_right(Some(right_id));
    let mut right_block = Block::new(right_id, Some(left_id), None, right.to_string());
    right_block.set_left(Some(left_id));

    let left_len = left_block.len;
    let right_len = right_block.len;

    doc.head = Some(left_id);
    doc.store.insert(left_block);
    doc.store.insert(right_block);
    doc.position_index.insert_after(None, left_id, left_len);
    doc.position_index
        .insert_after(Some(left_id), right_id, right_len);

    (doc, left_id, right_id)
}

#[test]
fn insert_into_empty_document() {
    let mut doc = Document::new(ClientId::new(1));
    doc.local_insert(0, "Test").unwrap();

    assert_eq!(doc.get_text(), "Test");
    assert_eq!(doc.state_vector.get(&ClientId::new(1)), 4);
}

#[test]
fn insert_append_prepend_and_middle() {
    let mut doc = Document::new(ClientId::new(1));

    doc.local_insert(0, "Vaime").unwrap();
    assert_eq!(doc.get_text(), "Vaime");

    doc.local_insert(0, "Vuime ").unwrap();
    assert_eq!(doc.get_text(), "Vuime Vaime");

    doc.local_insert(11, "!").unwrap();
    assert_eq!(doc.get_text(), "Vuime Vaime!");

    doc.local_insert(5, ", :O").unwrap();
    assert_eq!(doc.get_text(), "Vuime, :O Vaime!");
}

#[test]
fn insert_middle_maintains_correct_origins() {
    let mut doc = Document::new(ClientId::new(1));

    doc.local_insert(0, "AC").unwrap();
    doc.local_insert(1, "B").unwrap();

    assert_eq!(doc.get_text(), "ABC");
    assert_eq!(doc.state_vector.get(&ClientId::new(1)), 3);

    let a_id = doc.head.unwrap();
    let a_block = doc.store.get(&a_id).unwrap();
    assert_eq!(a_block.content(), "A");

    let b_id = a_block.right().unwrap();
    let b_block = doc.store.get(&b_id).unwrap();
    assert_eq!(b_block.content(), "B");

    let c_id = b_block.right().unwrap();
    let c_block = doc.store.get(&c_id).unwrap();
    assert_eq!(c_block.content(), "C");
    assert_eq!(b_block.origin_left, Some(a_id));
    assert_eq!(b_block.origin_right, Some(c_id));
}

#[test]
fn split_block_in_middle_updates_links_and_content() {
    let (mut doc, id) = doc_with_single_block("hello");

    doc.split_block(id, 2).unwrap();

    let left = doc.store.get(&id).unwrap();
    let right_id = left.right().unwrap();
    let right = doc.store.get(&right_id).unwrap();

    assert_eq!(left.content(), "he");
    assert_eq!(right.content(), "llo");
    assert_eq!(right_id, id.at_offset(2));
    assert_eq!(left.right(), Some(right_id));
    assert_eq!(right.left(), Some(id));
    assert_eq!(right.origin_left, Some(id));
}

#[test]
fn split_block_at_zero_is_noop() {
    let (mut doc, id) = doc_with_single_block("hello");

    doc.split_block(id, 0).unwrap();

    let block = doc.store.get(&id).unwrap();
    assert_eq!(block.content(), "hello");
    assert_eq!(block.right(), None);
}

#[test]
fn split_block_at_len_is_noop() {
    let (mut doc, id) = doc_with_single_block("hello");

    doc.split_block(id, 5).unwrap();

    let block = doc.store.get(&id).unwrap();
    assert_eq!(block.content(), "hello");
    assert_eq!(block.right(), None);
}

#[test]
fn split_block_past_len_is_noop() {
    let (mut doc, id) = doc_with_single_block("hello");

    doc.split_block(id, 99).unwrap();

    let block = doc.store.get(&id).unwrap();
    assert_eq!(block.content(), "hello");
    assert_eq!(block.right(), None);
}

#[test]
fn split_block_updates_existing_right_neighbor() {
    let (mut doc, left_id, right_id) = doc_with_two_blocks("abc", "def");

    doc.split_block(left_id, 1).unwrap();

    let left = doc.store.get(&left_id).unwrap();
    let middle_id = left.right().unwrap();
    let middle = doc.store.get(&middle_id).unwrap();
    let right = doc.store.get(&right_id).unwrap();

    assert_eq!(left.content(), "a");
    assert_eq!(middle.content(), "bc");
    assert_eq!(right.content(), "def");
    assert_eq!(middle.left(), Some(left_id));
    assert_eq!(middle.right(), Some(right_id));
    assert_eq!(right.left(), Some(middle_id));
}

#[test]
fn split_deleted_block_keeps_both_halves_deleted() {
    let (mut doc, id) = doc_with_single_block("hello");
    doc.mark_block_deleted(&id).unwrap();

    doc.split_block(id, 2).unwrap();

    let left = doc.store.get(&id).unwrap();
    let right_id = left.right().unwrap();
    let right = doc.store.get(&right_id).unwrap();

    assert!(left.is_deleted);
    assert!(right.is_deleted);
}

#[test]
fn get_block_and_offset_by_position_finds_first_block() {
    let (doc, left_id, _) = doc_with_two_blocks("abc", "def");

    let (found, offset, _tail) = doc.get_block_and_offset_by_position(2);

    assert_eq!(found, Some(left_id));
    assert_eq!(offset, 2);
}

#[test]
fn get_block_and_offset_by_position_finds_second_block() {
    let (doc, _, right_id) = doc_with_two_blocks("abc", "def");

    let (found, offset, _tail) = doc.get_block_and_offset_by_position(4);

    assert_eq!(found, Some(right_id));
    assert_eq!(offset, 1);
}

#[test]
fn get_block_and_offset_by_position_returns_none_past_end() {
    let (doc, _, _) = doc_with_two_blocks("abc", "def");

    let (found, offset, _tail) = doc.get_block_and_offset_by_position(7);

    assert_eq!(found, None);
    assert_eq!(offset, 1);
}

#[test]
fn get_block_and_offset_by_position_uses_character_offsets_for_unicode() {
    let client_id = ClientId::new(1);
    let left_id = block_id(1, 0);
    let right_id = block_id(1, 2);
    let mut doc = Document::new(client_id);

    let mut left = Block::new(left_id, None, Some(right_id), "a😀".to_string());
    left.set_right(Some(right_id));
    let mut right = Block::new(right_id, Some(left_id), None, "b".to_string());
    right.set_left(Some(left_id));

    let left_len = left.len;
    let right_len = right.len;

    doc.head = Some(left_id);
    doc.store.insert(left);
    doc.store.insert(right);
    doc.position_index.insert_after(None, left_id, left_len);
    doc.position_index
        .insert_after(Some(left_id), right_id, right_len);

    let (found, offset, _tail) = doc.get_block_and_offset_by_position(2);

    assert_eq!(found, Some(right_id));
    assert_eq!(offset, 0);
}

#[test]
fn test_remote_insert_conflict_resolution() {
    let mut doc_a = Document::new(ClientId::new(1));
    let mut doc_b = Document::new(ClientId::new(2));

    doc_a.local_insert(0, "A").unwrap();

    let id_a = doc_a.head.unwrap();
    let block_a = doc_a.store.get(&id_a).unwrap().clone();
    doc_b.remote_insert(block_a).unwrap();

    assert_eq!(doc_a.get_text(), "A");
    assert_eq!(doc_b.get_text(), "A");

    doc_a.local_insert(1, "X").unwrap();
    doc_b.local_insert(1, "Y").unwrap();

    assert_eq!(doc_a.get_text(), "AX");
    assert_eq!(doc_b.get_text(), "AY");

    let id_x = BlockId::new(ClientId::new(1), Clock::new(1));
    let block_x = doc_a.store.get(&id_x).unwrap().clone();

    let id_y = BlockId::new(ClientId::new(2), Clock::new(0));
    let block_y = doc_b.store.get(&id_y).unwrap().clone();

    doc_a.remote_insert(block_y).unwrap();
    doc_b.remote_insert(block_x).unwrap();

    let final_text_a = doc_a.get_text();
    let final_text_b = doc_b.get_text();

    assert_eq!(final_text_a, final_text_b, "Documents failed to converge");
    assert_eq!(final_text_a, "AXY");
}

#[test]
fn insert_out_of_bounds_on_empty_document_returns_error() {
    let mut doc = Document::new(ClientId::new(1));
    let result = doc.local_insert(5, "X");
    assert_eq!(result, Err(DocumentError::OutOfBounds(5)));
}

#[test]
fn split_block_right_half_inherits_origin_right_not_current_right() {
    let client1 = ClientId::new(1);
    let client3 = ClientId::new(3);
    let a_id = BlockId::new(client1, Clock::new(0));
    let c_id = BlockId::new(client3, Clock::new(0));

    let mut doc = Document::new(client1);

    let mut a = Block::new(a_id, None, None, "ab".to_string());
    let mut c = Block::new(c_id, Some(a_id), None, "cd".to_string());
    c.set_left(Some(a_id));
    a.set_right(Some(c_id));

    let a_len = a.len;
    let c_len = c.len;

    doc.head = Some(a_id);
    doc.store.insert(a);
    doc.store.insert(c);
    doc.position_index.insert_after(None, a_id, a_len);
    doc.position_index.insert_after(Some(a_id), c_id, c_len);

    doc.split_block(a_id, 1).unwrap();

    let middle_id = doc.store.get(&a_id).unwrap().right().unwrap();
    let middle = doc.store.get(&middle_id).unwrap();

    assert_eq!(middle.origin_right, None);
}

#[test]
fn yata_descendant_block_not_split_by_concurrent_insert() {
    let a_id = BlockId::new(ClientId::new(1), Clock::new(0));
    let b_id = BlockId::new(ClientId::new(2), Clock::new(0));
    let c_id = BlockId::new(ClientId::new(2), Clock::new(1));
    let x_id = BlockId::new(ClientId::new(3), Clock::new(0));

    let mut doc = Document::new(ClientId::new(99));

    doc.remote_insert(Block::new(a_id, None, None, "A".to_string()))
        .unwrap();
    doc.remote_insert(Block::new(b_id, Some(a_id), None, "B".to_string()))
        .unwrap();
    doc.remote_insert(Block::new(c_id, Some(b_id), None, "C".to_string()))
        .unwrap();

    assert_eq!(doc.get_text(), "ABC");

    doc.remote_insert(Block::new(x_id, Some(a_id), None, "X".to_string()))
        .unwrap();

    let text = doc.get_text();
    assert_ne!(
        text, "ABXC",
        "X must not split B's sequence (YATA interleaving bug)"
    );
    assert_eq!(text, "ABCX");
}

#[test]
fn concurrent_inserts_at_end_converge_regardless_of_arrival_order() {
    let mut doc_a = Document::new(ClientId::new(1));
    let mut doc_b = Document::new(ClientId::new(2));

    doc_a.local_insert(0, "A").unwrap();
    doc_b.local_insert(0, "B").unwrap();

    let id_a = BlockId::new(ClientId::new(1), Clock::new(0));
    let id_b = BlockId::new(ClientId::new(2), Clock::new(0));

    let block_a = doc_a.store.get(&id_a).unwrap().clone();
    let block_b = doc_b.store.get(&id_b).unwrap().clone();

    doc_a.remote_insert(block_b.clone()).unwrap();
    doc_b.remote_insert(block_a.clone()).unwrap();

    assert_eq!(
        doc_a.get_text(),
        doc_b.get_text(),
        "documents must converge regardless of arrival order"
    );
    assert_eq!(doc_a.get_text(), "AB");
}

#[test]
fn local_insert_mid_block_produces_correct_origin_left_clock() {
    let mut doc = Document::new(ClientId::new(1));
    doc.local_insert(0, "Hello").unwrap();

    doc.local_insert(2, "X").unwrap();

    let x_id = BlockId::new(ClientId::new(1), Clock::new(5));
    let x_block = doc.store.get(&x_id).unwrap();

    assert_eq!(
        x_block.origin_left,
        Some(BlockId::new(ClientId::new(1), Clock::new(1))),
        "origin_left must point to 'e' (clock 1), not the block-start clock"
    );
    assert_eq!(
        x_block.origin_right,
        Some(BlockId::new(ClientId::new(1), Clock::new(2))),
        "origin_right must point to 'l' (clock 2)"
    );

    assert_eq!(doc.get_text(), "HeXllo");
}

#[test]
fn struct_store_get_finds_block_inserted_out_of_clock_order() {
    use crate::store::StructStore;

    let client = ClientId::new(1);
    let id0 = BlockId::new(client, Clock::new(0));
    let id2 = BlockId::new(client, Clock::new(2));

    let mut store = StructStore::new();
    store.insert(Block::new(id2, None, None, "B".to_string()));
    store.insert(Block::new(id0, None, None, "A".to_string()));

    assert!(
        store.get(&id0).is_some(),
        "block at clock=0 must be findable"
    );
    assert!(
        store.get(&id2).is_some(),
        "block at clock=2 must be findable"
    );
}

#[test]
fn out_of_order_remote_blocks_are_buffered_then_applied() {
    let mut doc = Document::new(ClientId::new(1));
    let client2 = ClientId::new(2);

    let block_1 = Block::new(
        BlockId::new(client2, Clock::new(1)),
        Some(BlockId::new(client2, Clock::new(0))),
        None,
        "B".to_string(),
    );
    doc.remote_insert(block_1).unwrap();
    assert_eq!(
        doc.get_text(),
        "",
        "out-of-order block must be buffered, not applied"
    );

    let block_0 = Block::new(
        BlockId::new(client2, Clock::new(0)),
        None,
        None,
        "A".to_string(),
    );
    doc.remote_insert(block_0).unwrap();

    assert_eq!(
        doc.get_text(),
        "AB",
        "after gap is filled both blocks must appear"
    );
}

#[test]
fn delete_set_for_unreceived_block_is_applied_after_block_arrives() {
    use crate::store::DeleteSet;

    let mut doc = Document::new(ClientId::new(1));
    let client2 = ClientId::new(2);
    let target_id = BlockId::new(client2, Clock::new(0));

    let mut ds = DeleteSet::new();
    ds.add(target_id, 1);
    doc.apply_delete_set(&ds).unwrap();

    let block = Block::new(target_id, None, None, "A".to_string());
    doc.remote_insert(block).unwrap();

    assert_eq!(doc.get_text(), "");
    assert!(
        doc.store.get(&target_id).unwrap().is_deleted,
        "block must be marked deleted after pending delete set is drained"
    );
}

#[test]
fn remote_mid_block_insert_placed_at_correct_position() {
    let mut doc_a = Document::new(ClientId::new(1));
    let mut doc_b = Document::new(ClientId::new(2));

    doc_a.local_insert(0, "Hello").unwrap();
    let hello_id = doc_a.head.unwrap();
    let hello_block = doc_a.store.get(&hello_id).unwrap().clone();
    doc_b.remote_insert(hello_block).unwrap();

    assert_eq!(doc_b.get_text(), "Hello");

    doc_a.local_insert(2, "X").unwrap();
    assert_eq!(doc_a.get_text(), "HeXllo");

    let x_id = BlockId::new(ClientId::new(1), Clock::new(5));
    let x_block = doc_a.store.get(&x_id).unwrap().clone();

    doc_b.remote_insert(x_block).unwrap();

    assert_eq!(doc_b.get_text(), "HeXllo");
}

#[test]
fn two_docs_converge_after_concurrent_mid_block_inserts() {
    let mut doc_a = Document::new(ClientId::new(1));
    let mut doc_b = Document::new(ClientId::new(2));

    doc_a.local_insert(0, "Hello").unwrap();
    let hello_block = doc_a.store.get(&doc_a.head.unwrap()).unwrap().clone();
    doc_b.remote_insert(hello_block).unwrap();

    doc_a.local_insert(2, "X").unwrap();
    doc_b.local_insert(3, "Y").unwrap();

    let x_id = BlockId::new(ClientId::new(1), Clock::new(5));
    let x_block = doc_a.store.get(&x_id).unwrap().clone();

    let y_id = BlockId::new(ClientId::new(2), Clock::new(0));
    let y_block = doc_b.store.get(&y_id).unwrap().clone();

    doc_a.remote_insert(y_block).unwrap();
    doc_b.remote_insert(x_block).unwrap();

    assert_eq!(
        doc_a.get_text(),
        doc_b.get_text(),
        "documents must converge"
    );
}

#[test]
fn remote_block_referencing_unreceived_cross_client_origin_is_buffered() {
    let mut doc = Document::new(ClientId::new(99));
    let c1 = ClientId::new(1);
    let c2 = ClientId::new(2);

    let c1_block_id = BlockId::new(c1, Clock::new(0));
    let c2_block_id = BlockId::new(c2, Clock::new(0));

    let c2_block = Block::new(c2_block_id, Some(c1_block_id), None, "B".to_string());
    doc.remote_insert(c2_block).unwrap();
    assert_eq!(
        doc.get_text(),
        "",
        "block with missing cross-client origin must be buffered"
    );

    let c1_block = Block::new(c1_block_id, None, None, "A".to_string());
    doc.remote_insert(c1_block).unwrap();

    assert_eq!(doc.get_text(), "AB");
}

#[test]
fn remote_insert_dedupes_resent_block() {
    let mut doc = Document::new(ClientId::new(99));
    let c1 = ClientId::new(1);
    let a_id = BlockId::new(c1, Clock::new(0));
    let block_a = Block::new(a_id, None, None, "A".to_string());

    doc.remote_insert(block_a.clone()).unwrap();
    assert_eq!(doc.get_text(), "A");

    doc.remote_insert(block_a.clone()).unwrap();
    doc.remote_insert(block_a).unwrap();

    assert_eq!(
        doc.get_text(),
        "A",
        "duplicate retransmits must not double-insert"
    );
    assert_eq!(doc.state_vector.get(&c1), 1);
}

#[test]
fn drain_drops_pending_duplicate_when_gap_fills() {
    let mut doc = Document::new(ClientId::new(99));
    let c1 = ClientId::new(1);
    let id0 = BlockId::new(c1, Clock::new(0));
    let id1 = BlockId::new(c1, Clock::new(1));

    let block_b = Block::new(id1, Some(id0), None, "B".to_string());
    doc.remote_insert(block_b.clone()).unwrap();
    doc.remote_insert(block_b).unwrap();
    assert_eq!(doc.get_text(), "");

    let block_a = Block::new(id0, None, None, "A".to_string());
    doc.remote_insert(block_a).unwrap();

    assert_eq!(
        doc.get_text(),
        "AB",
        "pending duplicate must be dropped during drain instead of re-integrated"
    );
    assert_eq!(doc.state_vector.get(&c1), 2);
}

#[test]
fn delete_past_end_of_empty_document_returns_out_of_bounds() {
    let mut doc = Document::new(ClientId::new(1));
    let result = doc.delete(100, 5);
    assert_eq!(result, Err(DocumentError::OutOfBounds(100)));
}

#[test]
fn delete_past_end_of_nonempty_document_returns_out_of_bounds() {
    let mut doc = Document::new(ClientId::new(1));
    doc.local_insert(0, "abc").unwrap();
    let result = doc.delete(10, 1);
    assert_eq!(result, Err(DocumentError::OutOfBounds(10)));
}

#[test]
fn delete_over_length_returns_out_of_bounds_at_first_unreachable_position() {
    let mut doc = Document::new(ClientId::new(1));
    doc.local_insert(0, "abcde").unwrap();

    let result = doc.delete(0, 999);

    assert_eq!(
        result,
        Err(DocumentError::OutOfBounds(5)),
        "over-delete must error at the position where the doc ran out"
    );
}

#[test]
#[should_panic(expected = "cycle detected")]
fn get_text_debug_asserts_on_cycle() {
    let (mut doc, id) = doc_with_single_block("hello");
    doc.store.get_mut(&id).unwrap().set_right(Some(id));

    let _ = doc.get_text();
}

#[test]
fn cross_client_dependency_chain_drains_in_order() {
    let mut doc = Document::new(ClientId::new(99));
    let c1 = ClientId::new(1);
    let c2 = ClientId::new(2);
    let c3 = ClientId::new(3);

    let c1_id = BlockId::new(c1, Clock::new(0));
    let c2_id = BlockId::new(c2, Clock::new(0));
    let c3_id = BlockId::new(c3, Clock::new(0));

    doc.remote_insert(Block::new(c3_id, Some(c2_id), None, "C".to_string()))
        .unwrap();
    doc.remote_insert(Block::new(c2_id, Some(c1_id), None, "B".to_string()))
        .unwrap();
    assert_eq!(doc.get_text(), "", "chain still missing the root");

    doc.remote_insert(Block::new(c1_id, None, None, "A".to_string()))
        .unwrap();

    assert_eq!(doc.get_text(), "ABC");
}

#[test]
fn remote_insert_returns_change_with_visible_position() {
    let mut doc = Document::new(ClientId::new(99));
    doc.local_insert(0, "Hello").unwrap();

    let remote_id = BlockId::new(ClientId::new(7), Clock::new(0));
    let origin_left = Some(block_id(99, 4));
    let block = Block::new(remote_id, origin_left, None, "!".to_string());

    let changes = doc.remote_insert(block).unwrap();

    assert_eq!(
        changes,
        vec![RemoteChange::Insert {
            position: 5,
            content: "!".to_string(),
        }]
    );
}

#[test]
fn remote_insert_returns_empty_for_pending_and_duplicate() {
    let mut doc = Document::new(ClientId::new(99));

    let missing_origin = BlockId::new(ClientId::new(1), Clock::new(0));
    let pending_block = Block::new(
        BlockId::new(ClientId::new(2), Clock::new(0)),
        Some(missing_origin),
        None,
        "X".to_string(),
    );
    let pending_changes = doc.remote_insert(pending_block).unwrap();
    assert!(pending_changes.is_empty(), "pending yields no changes");

    doc.remote_insert(Block::new(missing_origin, None, None, "A".to_string()))
        .unwrap();
    let duplicate = Block::new(missing_origin, None, None, "A".to_string());
    let dup_changes = doc.remote_insert(duplicate).unwrap();
    assert!(dup_changes.is_empty(), "duplicate yields no changes");
}

#[test]
fn remote_insert_drained_pending_returns_all_changes_in_order() {
    let mut doc = Document::new(ClientId::new(99));
    let c1 = ClientId::new(1);
    let c2 = ClientId::new(2);

    let c1_id = BlockId::new(c1, Clock::new(0));
    let c2_id = BlockId::new(c2, Clock::new(0));

    let buffered = doc
        .remote_insert(Block::new(c2_id, Some(c1_id), None, "B".to_string()))
        .unwrap();
    assert!(buffered.is_empty());

    let changes = doc
        .remote_insert(Block::new(c1_id, None, None, "A".to_string()))
        .unwrap();

    assert_eq!(
        changes,
        vec![
            RemoteChange::Insert {
                position: 0,
                content: "A".to_string()
            },
            RemoteChange::Insert {
                position: 1,
                content: "B".to_string()
            },
        ]
    );
    assert_eq!(doc.get_text(), "AB");
}

#[test]
fn apply_delete_set_returns_delete_events_with_visible_positions() {
    let client = ClientId::new(1);
    let mut doc_a = Document::new(client);
    doc_a.local_insert(0, "Hello").unwrap();

    let mut doc_b = Document::new(ClientId::new(2));
    doc_b
        .remote_insert(Block::new(
            BlockId::new(client, Clock::new(0)),
            None,
            None,
            "Hello".to_string(),
        ))
        .unwrap();

    doc_a.delete(1, 3).unwrap();

    let changes = doc_b.apply_delete_set(&doc_a.delete_set).unwrap();

    assert_eq!(
        changes,
        vec![RemoteChange::Delete {
            position: 1,
            length: 3,
        }]
    );
    assert_eq!(doc_b.get_text(), "Ho");
}

#[test]
fn apply_delete_set_is_idempotent_and_emits_nothing_on_resend() {
    let client = ClientId::new(1);
    let mut doc_a = Document::new(client);
    doc_a.local_insert(0, "Hello").unwrap();
    doc_a.delete(0, 2).unwrap();

    let mut doc_b = Document::new(ClientId::new(2));
    doc_b
        .remote_insert(Block::new(
            BlockId::new(client, Clock::new(0)),
            None,
            None,
            "Hello".to_string(),
        ))
        .unwrap();

    let first = doc_b.apply_delete_set(&doc_a.delete_set).unwrap();
    assert_eq!(first.len(), 1);

    let second = doc_b.apply_delete_set(&doc_a.delete_set).unwrap();
    assert!(
        second.is_empty(),
        "already-tombstoned runs must not emit repeat Delete events"
    );
}

#[test]
fn apply_delete_set_buffers_until_block_arrives_then_drains_on_remote_insert() {
    let author = ClientId::new(1);
    let mut doc = Document::new(ClientId::new(2));

    let mut ds = crate::store::DeleteSet::new();
    ds.add(BlockId::new(author, Clock::new(0)), 3);

    let buffered = doc.apply_delete_set(&ds).unwrap();
    assert!(buffered.is_empty(), "no block yet, nothing to delete");

    let changes = doc
        .remote_insert(Block::new(
            BlockId::new(author, Clock::new(0)),
            None,
            None,
            "ABC".to_string(),
        ))
        .unwrap();

    assert_eq!(
        changes,
        vec![
            RemoteChange::Insert {
                position: 0,
                content: "ABC".to_string()
            },
            RemoteChange::Delete {
                position: 0,
                length: 3,
            },
        ]
    );
    assert_eq!(doc.get_text(), "");
}

#[test]
fn local_insert_returns_wire_block() {
    let mut doc = Document::new(ClientId::new(1));
    let wire = doc.local_insert(0, "hi").unwrap().expect("expected Some");
    assert_eq!(wire.content, "hi");
    assert_eq!(wire.id, BlockId::new(ClientId::new(1), Clock::new(0)));
    assert_eq!(wire.origin_left, None);
    assert_eq!(wire.origin_right, None);
}

#[test]
fn local_insert_empty_returns_none() {
    let mut doc = Document::new(ClientId::new(1));
    assert!(doc.local_insert(0, "").unwrap().is_none());
}

#[test]
fn local_insert_mid_document_returns_correct_origins() {
    let mut doc = Document::new(ClientId::new(1));
    doc.local_insert(0, "ac").unwrap();
    let wire = doc.local_insert(1, "b").unwrap().expect("expected Some");
    assert_eq!(wire.content, "b");
    assert_eq!(
        wire.origin_left,
        Some(BlockId::new(ClientId::new(1), Clock::new(0)))
    );
    assert_eq!(doc.get_text(), "abc");
}

#[test]
fn delete_returns_delete_set_diff_for_tombstoned_range() {
    let mut doc = Document::new(ClientId::new(1));
    doc.local_insert(0, "hello").unwrap();
    let diff = doc.delete(1, 3).unwrap();
    assert!(
        !diff.is_empty(),
        "diff must describe the 3 tombstoned chars"
    );
    assert_eq!(doc.get_text(), "ho");
}

#[test]
fn delete_zero_length_returns_empty_diff() {
    let mut doc = Document::new(ClientId::new(1));
    doc.local_insert(0, "hi").unwrap();
    let diff = doc.delete(0, 0).unwrap();
    assert!(diff.is_empty());
    assert_eq!(doc.get_text(), "hi");
}

#[test]
fn wire_block_round_trip_reconstructs_identical_document() {
    let mut doc_a = Document::new(ClientId::new(1));
    let wire = doc_a.local_insert(0, "hi").unwrap().expect("expected Some");
    let bytes = bitcode::encode(&wire);
    let decoded: WireBlock = bitcode::decode(&bytes).expect("decode");

    let mut doc_b = Document::new(ClientId::new(2));
    doc_b.remote_insert(Block::from(decoded)).unwrap();
    assert_eq!(doc_b.get_text(), "hi");
}

#[test]
fn snapshot_empty_document_round_trips() {
    use crate::snapshot::Snapshot;

    let doc = Document::new(ClientId::new(1));
    let snap = doc.to_snapshot();
    let bytes = snap.encode();
    let restored = Document::from_snapshot(Snapshot::decode(&bytes).unwrap());

    assert_eq!(restored.get_text(), "");
    assert_eq!(restored.client_id, ClientId::new(1));
}

#[test]
fn snapshot_with_content_round_trips() {
    use crate::snapshot::Snapshot;

    let mut doc = Document::new(ClientId::new(1));
    doc.local_insert(0, "hello ").unwrap();
    doc.local_insert(6, "world").unwrap();
    assert_eq!(doc.get_text(), "hello world");

    let snap = doc.to_snapshot();
    let bytes = snap.encode();
    let restored = Document::from_snapshot(Snapshot::decode(&bytes).unwrap());

    assert_eq!(restored.get_text(), "hello world");
    assert_eq!(restored.client_id, ClientId::new(1));
}

#[test]
fn snapshot_preserves_deletions() {
    use crate::snapshot::Snapshot;

    let mut doc = Document::new(ClientId::new(1));
    doc.local_insert(0, "abcde").unwrap();
    doc.delete(1, 3).unwrap();
    assert_eq!(doc.get_text(), "ae");

    let snap = doc.to_snapshot();
    let bytes = snap.encode();
    let restored = Document::from_snapshot(Snapshot::decode(&bytes).unwrap());

    assert_eq!(restored.get_text(), "ae");
}

#[test]
fn snapshot_preserves_state_vector() {
    use crate::snapshot::Snapshot;

    let mut doc = Document::new(ClientId::new(42));
    doc.local_insert(0, "abc").unwrap();

    let snap = doc.to_snapshot();
    let bytes = snap.encode();
    let restored = Document::from_snapshot(Snapshot::decode(&bytes).unwrap());

    assert_eq!(restored.state_vector.get(&ClientId::new(42)), 3);
}

#[test]
fn snapshot_can_accept_new_inserts_after_load() {
    use crate::snapshot::Snapshot;

    let mut doc = Document::new(ClientId::new(1));
    doc.local_insert(0, "hello").unwrap();

    let snap = doc.to_snapshot();
    let bytes = snap.encode();
    let mut restored = Document::from_snapshot(Snapshot::decode(&bytes).unwrap());

    restored.local_insert(5, " world").unwrap();
    assert_eq!(restored.get_text(), "hello world");
}

#[test]
fn fork_creates_independent_copy_with_new_client_id() {
    let mut doc = Document::new(ClientId::new(1));
    doc.local_insert(0, "original").unwrap();

    let fork = doc.fork(ClientId::new(99));

    assert_eq!(fork.client_id, ClientId::new(99));
    assert_eq!(fork.get_text(), "original");
    assert_eq!(doc.get_text(), "original");
    assert_eq!(doc.client_id, ClientId::new(1));
}

#[test]
fn fork_diverges_independently() {
    let mut doc = Document::new(ClientId::new(1));
    doc.local_insert(0, "base").unwrap();

    let mut fork = doc.fork(ClientId::new(2));
    fork.local_insert(4, " fork").unwrap();
    doc.local_insert(4, " original").unwrap();

    assert_eq!(doc.get_text(), "base original");
    assert_eq!(fork.get_text(), "base fork");
}

#[test]
fn fork_clocks_do_not_collide() {
    let mut doc = Document::new(ClientId::new(1));
    doc.local_insert(0, "abc").unwrap();

    let mut fork = doc.fork(ClientId::new(2));
    let wire = fork.local_insert(3, "d").unwrap().unwrap();
    assert_eq!(wire.id.client, ClientId::new(2));
    assert_eq!(wire.id.clock.value, 0);
}

#[test]
fn snapshot_version_mismatch_returns_error() {
    use crate::snapshot::Snapshot;

    let doc = Document::new(ClientId::new(1));
    let mut snap = doc.to_snapshot();
    snap.version = 255;
    let bytes = bitcode::encode(&snap);
    let result = Snapshot::decode(&bytes);
    assert!(result.is_err());
}
