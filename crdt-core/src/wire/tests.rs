use super::*;
use crate::store::DeleteSet;
use crate::structs::Block;
use crate::types::{BlockId, ClientId, Clock};

fn bid(client: u64, clock: u64) -> BlockId {
    BlockId::new(ClientId::new(client), Clock::new(clock))
}

#[test]
fn wire_block_round_trips_through_block() {
    let original = Block::new(bid(1, 0), None, Some(bid(2, 5)), "hello".to_string());
    let wire = WireBlock::from(&original);
    let recovered = Block::from(wire.clone());

    assert_eq!(recovered.id, original.id);
    assert_eq!(recovered.origin_left, original.origin_left);
    assert_eq!(recovered.origin_right, original.origin_right);
    assert_eq!(recovered.content(), original.content());
    assert_eq!(recovered.len, original.len);
    assert!(!recovered.is_deleted);
}

#[test]
fn wire_block_with_both_origins_round_trips() {
    let wire = WireBlock {
        id: bid(3, 10),
        origin_left: Some(bid(1, 4)),
        origin_right: Some(bid(2, 7)),
        content: "x".to_string(),
    };
    let wire2 = wire.clone();
    let block = Block::from(wire);
    assert_eq!(WireBlock::from(&block), wire2);
}

#[test]
fn wire_block_bitcode_round_trip() {
    let wire = WireBlock {
        id: bid(7, 3),
        origin_left: Some(bid(1, 0)),
        origin_right: None,
        content: "hello".to_string(),
    };
    let bytes = bitcode::encode(&wire);
    let decoded: WireBlock = bitcode::decode(&bytes).expect("decode");
    assert_eq!(decoded, wire);
}

#[test]
fn encode_decode_insert_round_trips() {
    let msg = OpMessage::Insert(WireBlock {
        id: bid(1, 0),
        origin_left: None,
        origin_right: None,
        content: "hi".to_string(),
    });
    let frame = encode_op(&msg);
    assert_eq!(frame[0], OP_PREFIX);
    let decoded = decode_op(&frame).expect("decode");
    assert_eq!(decoded, msg);
}

#[test]
fn encode_decode_delete_round_trips() {
    let mut ds = DeleteSet::new();
    ds.add(bid(1, 0), 3);
    ds.add(bid(2, 5), 2);
    let msg = OpMessage::Delete(ds);
    let frame = encode_op(&msg);
    assert_eq!(frame[0], OP_PREFIX);
    let decoded = decode_op(&frame).expect("decode");
    assert_eq!(decoded, msg);
}

#[test]
fn decode_op_rejects_empty_frame() {
    assert!(matches!(decode_op(&[]), Err(WireError::EmptyFrame)));
}

#[test]
fn decode_op_rejects_snapshot_prefix() {
    let frame = vec![SNAPSHOT_PREFIX, 0x00];
    assert!(matches!(decode_op(&frame), Err(WireError::NotAnOp)));
}

#[test]
fn decode_op_rejects_unknown_prefix() {
    let frame = vec![0xFF, 0x00];
    assert!(matches!(
        decode_op(&frame),
        Err(WireError::UnknownPrefix(0xFF))
    ));
}

#[test]
fn decode_op_surfaces_bitcode_error_on_garbage_payload() {
    let frame = vec![OP_PREFIX, 0xFF, 0xFF, 0xFF, 0xFF];
    assert!(matches!(decode_op(&frame), Err(WireError::Decode(_))));
}

#[test]
fn wire_error_display_has_stable_text() {
    let e = WireError::EmptyFrame;
    assert!(!format!("{e}").is_empty());
}

#[test]
fn encode_decode_snapshot_round_trips() {
    use crate::snapshot::{SNAPSHOT_VERSION, Snapshot, SnapshotBlock};

    let snap = Snapshot {
        version: SNAPSHOT_VERSION,
        client_id: ClientId::new(42),
        blocks: vec![SnapshotBlock {
            id: bid(42, 0),
            origin_left: None,
            origin_right: None,
            left: None,
            right: None,
            content: "hello".to_string(),
            is_deleted: false,
        }],
        state_vector: vec![(ClientId::new(42), 1)],
        delete_set: DeleteSet::new(),
        seen_delete_set: DeleteSet::new(),
        head: Some(bid(42, 0)),
        pending_blocks: vec![],
        pending_delete_sets: vec![],
    };
    let frame = encode_snapshot(&snap);
    assert_eq!(frame[0], SNAPSHOT_PREFIX);
    let decoded = decode_snapshot(&frame).expect("decode");
    assert_eq!(decoded.version, SNAPSHOT_VERSION);
    assert_eq!(decoded.client_id, snap.client_id);
    assert_eq!(decoded.blocks.len(), 1);
    assert_eq!(decoded.blocks[0].content, "hello");
}

#[test]
fn decode_snapshot_rejects_op_prefix() {
    let frame = vec![OP_PREFIX, 0x00];
    assert!(matches!(
        decode_snapshot(&frame),
        Err(WireError::NotASnapshot)
    ));
}

#[test]
fn decode_snapshot_rejects_empty_frame() {
    assert!(matches!(decode_snapshot(&[]), Err(WireError::EmptyFrame)));
}

#[test]
fn encode_decode_gc_commit_round_trips() {
    let mut ds = DeleteSet::new();
    ds.add(bid(1, 0), 3);
    ds.add(bid(2, 5), 2);
    let frame = encode_gc_commit(&ds);
    assert_eq!(frame[0], PREFIX_GC_COMMIT);
    let decoded = decode_gc_commit(&frame).expect("decode");
    assert_eq!(decoded, ds);
}

#[test]
fn decode_gc_commit_rejects_op_prefix() {
    let frame = vec![OP_PREFIX, 0x00];
    assert!(matches!(
        decode_gc_commit(&frame),
        Err(WireError::NotAGcCommit)
    ));
}

#[test]
fn encode_decode_sv_report_round_trips() {
    let sender = ClientId::new(7);
    let entries = vec![(ClientId::new(1), 4u64), (ClientId::new(9), 0u64)];
    let frame = encode_sv_report(sender, &entries);
    assert_eq!(frame[0], PREFIX_SV_REPORT);
    let (decoded_sender, decoded) = decode_sv_report(&frame).expect("decode");
    assert_eq!(decoded_sender, sender);
    assert_eq!(decoded, entries);
}

#[test]
fn encode_decode_empty_sv_report_round_trips() {
    let sender = ClientId::new(3);
    let frame = encode_sv_report(sender, &[]);
    assert_eq!(frame[0], PREFIX_SV_REPORT);
    let (decoded_sender, decoded) = decode_sv_report(&frame).expect("decode");
    assert_eq!(decoded_sender, sender);
    assert_eq!(decoded, Vec::new());
}

#[test]
fn decode_sv_report_rejects_snapshot_prefix() {
    let frame = vec![SNAPSHOT_PREFIX, 0x00];
    assert!(matches!(
        decode_sv_report(&frame),
        Err(WireError::NotAnSvReport)
    ));
}

#[test]
fn presence_round_trips_joined_and_left() {
    for event in [PresenceEvent::Joined, PresenceEvent::Left] {
        let frame = PresenceFrame {
            client_id: ClientId::new(0x0102_0304_0506_0708),
            event,
        };
        let bytes = encode_presence(&frame);
        assert_eq!(bytes.len(), 10);
        assert_eq!(bytes[0], PREFIX_PRESENCE);
        assert_eq!(decode_presence(&bytes).expect("decode"), frame);
    }
}

#[test]
fn decode_presence_rejects_wrong_prefix() {
    let frame = vec![OP_PREFIX, PRESENCE_JOINED, 0, 0, 0, 0, 0, 0, 0, 0];
    assert!(matches!(
        decode_presence(&frame),
        Err(WireError::NotAPresence)
    ));
}

#[test]
fn decode_presence_rejects_bad_length() {
    let frame = vec![PREFIX_PRESENCE, PRESENCE_JOINED, 0, 0];
    assert!(matches!(
        decode_presence(&frame),
        Err(WireError::MalformedPresence)
    ));
}

#[test]
fn decode_presence_rejects_unknown_event() {
    let frame = vec![PREFIX_PRESENCE, 0xEE, 0, 0, 0, 0, 0, 0, 0, 1];
    assert!(matches!(
        decode_presence(&frame),
        Err(WireError::MalformedPresence)
    ));
}
