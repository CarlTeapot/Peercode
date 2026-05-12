use super::{CONTROL_SESSION_ENDED, OP_PREFIX, PREFIX_CONTROL, SNAPSHOT_PREFIX};

#[test]
fn prefix_constants_match_go_mirror() {
    const GO_PREFIX_OP: u8 = 0x00;
    const GO_PREFIX_SNAPSHOT: u8 = 0x01;

    const GO_PREFIX_CONTROL: u8 = 0x02;
    const GO_CONTROL_SESSION_ENDED: u8 = 0x01;

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
}
