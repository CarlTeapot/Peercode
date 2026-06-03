use super::{
    CONTROL_SESSION_ENDED, CONTROL_SNAPSHOT_REQUEST, OP_PREFIX, PREFIX_CONTROL, PREFIX_GC_COMMIT,
    PREFIX_PRESENCE, PREFIX_SV_REPORT, PRESENCE_JOINED, PRESENCE_LEFT, PresenceEvent,
    PresenceFrame, SNAPSHOT_PREFIX, encode_presence,
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
    const GO_PREFIX_PRESENCE: u8 = 0x05;
    const GO_PREFIX_SV_REPORT: u8 = 0x06;
    const GO_PRESENCE_JOINED: u8 = 0x01;
    const GO_PRESENCE_LEFT: u8 = 0x02;

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
        PREFIX_PRESENCE, GO_PREFIX_PRESENCE,
        "PREFIX_PRESENCE drifted from gateway/internal/wire::PrefixPresence"
    );
    assert_eq!(
        PREFIX_SV_REPORT, GO_PREFIX_SV_REPORT,
        "PREFIX_SV_REPORT drifted from gateway/internal/wire::PrefixSvReport"
    );
    assert_eq!(
        PRESENCE_JOINED, GO_PRESENCE_JOINED,
        "PRESENCE_JOINED drifted from gateway/internal/wire::PresenceJoined"
    );
    assert_eq!(
        PRESENCE_LEFT, GO_PRESENCE_LEFT,
        "PRESENCE_LEFT drifted from gateway/internal/wire::PresenceLeft"
    );
}

#[test]
fn presence_layout_matches_go_mirror() {
    // The gateway hand-assembles this exact byte layout in Go. If this expected
    // vector changes, gateway/internal/wire::EncodePresenceFrame must change too
    // (and its mirror test asserts the same bytes).
    let frame = PresenceFrame {
        client_id: ClientId::new(0x0102_0304_0506_0708),
        event: PresenceEvent::Joined,
    };
    let expected = vec![
        PREFIX_PRESENCE,
        PRESENCE_JOINED,
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
        encode_presence(&frame),
        expected,
        "presence frame layout drifted from gateway/internal/wire::EncodePresenceFrame"
    );
}
