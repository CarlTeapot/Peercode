use super::*;
use crate::types::{BlockId, ClientId, Clock};

fn bid(client: u64, clock: u64) -> BlockId {
    BlockId::new(ClientId { value: client }, Clock::new(clock))
}

#[test]
fn new_index_is_empty() {
    let idx = PositionIndex::new();
    assert_eq!(idx.visible_len(), 0);
}

#[test]
fn insert_single_block_into_empty() {
    let mut idx = PositionIndex::new();
    idx.insert_after(None, bid(1, 0), 5);
    assert_eq!(idx.visible_len(), 5);
    idx.debug_validate().unwrap();
}

#[test]
fn insert_at_head_of_single_leaf() {
    let mut idx = PositionIndex::new();
    idx.insert_after(None, bid(1, 0), 3);
    idx.insert_after(None, bid(2, 0), 2);
    assert_eq!(idx.visible_len(), 5);
    idx.debug_validate().unwrap();
}

#[test]
fn insert_after_existing_entry() {
    let mut idx = PositionIndex::new();
    idx.insert_after(None, bid(1, 0), 3);
    idx.insert_after(Some(bid(1, 0)), bid(2, 0), 4);
    assert_eq!(idx.visible_len(), 7);
    idx.debug_validate().unwrap();
}

#[test]
fn position_of_within_single_leaf() {
    let mut idx = PositionIndex::new();
    idx.insert_after(None, bid(1, 0), 3); // "abc"
    idx.insert_after(Some(bid(1, 0)), bid(2, 0), 2); // "de"
    idx.insert_after(Some(bid(2, 0)), bid(3, 0), 4); // "fghi"

    assert_eq!(idx.position_of(bid(1, 0)), Some(0));
    assert_eq!(idx.position_of(bid(2, 0)), Some(3));
    assert_eq!(idx.position_of(bid(3, 0)), Some(5));
    assert_eq!(idx.position_of(bid(99, 0)), None);
    idx.debug_validate().unwrap();
}

#[test]
fn leaf_overflow_creates_internal_node() {
    let mut idx = PositionIndex::new();
    let mut prev = None;
    for i in 0..5u64 {
        idx.insert_after(prev, bid(1, i), 1);
        prev = Some(bid(1, i));
    }
    assert_eq!(idx.visible_len(), 5);
    for i in 0..5u64 {
        assert_eq!(idx.position_of(bid(1, i)), Some(i));
    }
    idx.debug_validate().unwrap();
}

#[test]
fn many_inserts_keep_positions_correct() {
    let mut idx = PositionIndex::new();
    let mut prev = None;
    for i in 0..12u64 {
        idx.insert_after(prev, bid(1, i), 1);
        prev = Some(bid(1, i));
    }
    assert_eq!(idx.visible_len(), 12);
    for i in 0..12u64 {
        assert_eq!(idx.position_of(bid(1, i)), Some(i));
    }
    idx.debug_validate().unwrap();
}

#[test]
fn insert_at_head_after_root_grew() {
    let mut idx = PositionIndex::new();
    let mut prev = None;
    for i in 0..6u64 {
        idx.insert_after(prev, bid(1, i), 1);
        prev = Some(bid(1, i));
    }
    idx.insert_after(None, bid(2, 0), 7);
    assert_eq!(idx.position_of(bid(2, 0)), Some(0));
    assert_eq!(idx.position_of(bid(1, 0)), Some(7));
    assert_eq!(idx.visible_len(), 13);
    idx.debug_validate().unwrap();
}

#[test]
fn find_at_position_basic() {
    let mut idx = PositionIndex::new();
    idx.insert_after(None, bid(1, 0), 3); // "abc" → 0..3
    idx.insert_after(Some(bid(1, 0)), bid(2, 0), 2); // "de" → 3..5
    idx.insert_after(Some(bid(2, 0)), bid(3, 0), 4); // "fghi" → 5..9

    let r = idx.find_at_position(0);
    assert_eq!(r.id, Some(bid(1, 0)));
    assert_eq!(r.offset, 0);

    let r = idx.find_at_position(2);
    assert_eq!(r.id, Some(bid(1, 0)));
    assert_eq!(r.offset, 2);

    let r = idx.find_at_position(3);
    assert_eq!(r.id, Some(bid(2, 0)));
    assert_eq!(r.offset, 0);

    let r = idx.find_at_position(8);
    assert_eq!(r.id, Some(bid(3, 0)));
    assert_eq!(r.offset, 3);

    let r = idx.find_at_position(9);
    assert_eq!(r.id, None);
    assert_eq!(r.offset, 0);
    assert_eq!(r.tail_id, Some(bid(3, 0)));

    let r = idx.find_at_position(15);
    assert_eq!(r.id, None);
    assert_eq!(r.offset, 6);
    assert_eq!(r.tail_id, Some(bid(3, 0)));
    idx.debug_validate().unwrap();
}

#[test]
fn find_at_position_empty_tree() {
    let idx = PositionIndex::new();
    let r = idx.find_at_position(0);
    assert_eq!(r.id, None);
    assert_eq!(r.offset, 0);
    assert_eq!(r.tail_id, None);
}

#[test]
fn set_deleted_shrinks_visible_len() {
    let mut idx = PositionIndex::new();
    idx.insert_after(None, bid(1, 0), 3);
    idx.insert_after(Some(bid(1, 0)), bid(2, 0), 2);
    idx.insert_after(Some(bid(2, 0)), bid(3, 0), 4);
    assert_eq!(idx.visible_len(), 9);

    idx.set_deleted(bid(2, 0));
    assert_eq!(idx.visible_len(), 7);
    assert_eq!(idx.position_of(bid(1, 0)), Some(0));
    assert_eq!(idx.position_of(bid(3, 0)), Some(3));

    let r = idx.find_at_position(3);
    assert_eq!(r.id, Some(bid(3, 0)));
    assert_eq!(r.offset, 0);
    idx.debug_validate().unwrap();
}

#[test]
fn split_entry_creates_new_entry_with_correct_positions() {
    let mut idx = PositionIndex::new();
    idx.insert_after(None, bid(1, 0), 5);
    idx.split_entry(bid(1, 0), 2, bid(1, 2));
    assert_eq!(idx.visible_len(), 5);
    assert_eq!(idx.position_of(bid(1, 0)), Some(0));
    assert_eq!(idx.position_of(bid(1, 2)), Some(2));

    let r = idx.find_at_position(3);
    assert_eq!(r.id, Some(bid(1, 2)));
    assert_eq!(r.offset, 1);
    idx.debug_validate().unwrap();
}

#[test]
fn split_entry_preserves_is_deleted() {
    let mut idx = PositionIndex::new();
    idx.insert_after(None, bid(1, 0), 5);
    idx.set_deleted(bid(1, 0));
    idx.split_entry(bid(1, 0), 2, bid(1, 2));
    assert_eq!(idx.visible_len(), 0);
    idx.debug_validate().unwrap();
}

#[test]
fn rebuild_matches_incremental_inserts() {
    let mut a = PositionIndex::new();
    let mut prev = None;
    for i in 0..10u64 {
        a.insert_after(prev, bid(1, i), 1);
        prev = Some(bid(1, i));
    }
    a.set_deleted(bid(1, 3));
    a.set_deleted(bid(1, 7));

    let mut b = PositionIndex::new();
    let entries = (0..10u64).map(|i| {
        let deleted = i == 3 || i == 7;
        (bid(1, i), 1u64, deleted)
    });
    b.rebuild_from_order(entries);

    assert_eq!(a.visible_len(), b.visible_len());
    for i in 0..10u64 {
        assert_eq!(a.position_of(bid(1, i)), b.position_of(bid(1, i)));
    }
    a.debug_validate().unwrap();
    b.debug_validate().unwrap();
}

#[test]
fn find_at_position_past_end_with_multi_leaf_tree() {
    let mut idx = PositionIndex::new();
    let mut prev = None;

    for i in 0..8u64 {
        idx.insert_after(prev, bid(1, i), 1);
        prev = Some(bid(1, i));
    }
    assert_eq!(idx.visible_len(), 8);

    let r = idx.find_at_position(8);
    assert_eq!(r.id, None, "pos == visible_len must be past-end");
    assert_eq!(r.offset, 0);
    assert_eq!(r.tail_id, Some(bid(1, 7)));

    let r = idx.find_at_position(12);
    assert_eq!(r.id, None);
    assert_eq!(r.offset, 4);
    assert_eq!(r.tail_id, Some(bid(1, 7)));

    let r = idx.find_at_position(7);
    assert_eq!(r.id, Some(bid(1, 7)));
    assert_eq!(r.offset, 0);
}

#[test]
fn debug_validate_ok_for_typical_tree() {
    let mut idx = PositionIndex::new();
    let mut prev = None;
    for i in 0..10u64 {
        idx.insert_after(prev, bid(1, i), 2);
        prev = Some(bid(1, i));
    }
    idx.set_deleted(bid(1, 4));
    idx.split_entry(bid(1, 6), 1, bid(2, 0));
    idx.debug_validate().expect("tree invariants hold");
}
