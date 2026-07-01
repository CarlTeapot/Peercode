use super::{
    CONTROL_SESSION_ENDED, CONTROL_SNAPSHOT_REQUEST, MembershipEvent, MembershipFrame, OP_PREFIX,
    PEER_JOINED, PEER_LEFT, PREFIX_CONTROL, PREFIX_GC_COMMIT, PREFIX_MEMBERSHIP, PREFIX_SV_REPORT,
    SNAPSHOT_PREFIX, encode_membership,
};
use crate::types::ClientId;

#[test]
fn prefix_constants_match_go_mirror() {
    const GO_PREFIX_OP: u8 = 0x00;
    const GO_PREFIX_SNAPSHOT: u8 = 0x01;

    const GO_PREFIX_CONTROL: u8 = 0x02;
    const GO_CONTROL_SESSION_ENDED: u8 = 0x01;
    const GO_CONTROL_SNAPSHOT_REQUEST: u8 = 0x02;

    const GO_PREFIX_GC_COMMIT: u8 = 0x04;
    const GO_PREFIX_MEMBERSHIP: u8 = 0x05;
    const GO_PREFIX_SV_REPORT: u8 = 0x06;
    const GO_MEMBERSHIP_JOINED: u8 = 0x01;
    const GO_MEMBERSHIP_LEFT: u8 = 0x02;

    assert_eq!(
        OP_PREFIX, GO_PREFIX_OP,
        "OP_PREFIX drifted from gateway/internal/wire::PrefixOp"
    );
    assert_eq!(
        SNAPSHOT_PREFIX, GO_PREFIX_SNAPSHOT,
        "SNAPSHOT_PREFIX drifted from gateway/internal/wire::PrefixSnapshot"
    );
    assert_eq!(
        PREFIX_CONTROL, GO_PREFIX_CONTROL,
        "PREFIX_CONTROL drifted from gateway/internal/wire::PREFIX_CONTROL"
    );
    assert_eq!(
        CONTROL_SESSION_ENDED, GO_CONTROL_SESSION_ENDED,
        "CONTROL_SESSION_ENDED drifted from gateway/internal/wire::CONTROL_SESSION_ENDED"
    );
    assert_eq!(
        CONTROL_SNAPSHOT_REQUEST, GO_CONTROL_SNAPSHOT_REQUEST,
        "CONTROL_SNAPSHOT_REQUEST drifted from gateway/internal/wire::CONTROL_SNAPSHOT_REQUEST"
    );
    assert_eq!(
        PREFIX_GC_COMMIT, GO_PREFIX_GC_COMMIT,
        "PREFIX_GC_COMMIT drifted from gateway/internal/wire::PrefixGcCommit"
    );
    assert_eq!(
        PREFIX_MEMBERSHIP, GO_PREFIX_MEMBERSHIP,
        "PREFIX_PRESENCE drifted from gateway/internal/wire::PrefixPresence"
    );
    assert_eq!(
        PREFIX_SV_REPORT, GO_PREFIX_SV_REPORT,
        "PREFIX_SV_REPORT drifted from gateway/internal/wire::PrefixSvReport"
    );
    assert_eq!(
        PEER_JOINED, GO_MEMBERSHIP_JOINED,
        "MEMBERSHIP_JOINED drifted from gateway/internal/wire::MembershipJoined"
    );
    assert_eq!(
        PEER_LEFT, GO_MEMBERSHIP_LEFT,
        "MEMBERSHIP_LEFT drifted from gateway/internal/wire::MembershipLeft"
    );
}

#[test]
fn presence_layout_matches_go_mirror() {
    let frame = MembershipFrame {
        client_id: ClientId::new(0x0102_0304_0506_0708),
        event: MembershipEvent::Joined,
    };
    let expected = vec![
        PREFIX_MEMBERSHIP,
        PEER_JOINED,
        0x01,
        0x02,
        0x03,
        0x04,
        0x05,
        0x06,
        0x07,
        0x08,
    ];
    assert_eq!(
        encode_membership(&frame),
        expected,
        "Membership frame layout drifted from gateway/internal/wire::EncodeMembershipFrame"
    );
}
