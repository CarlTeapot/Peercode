package wire

import "testing"

func TestPrefixConstantsMatchRustSource(t *testing.T) {
	const (
		rustOpPrefix       byte = 0x00
		rustSnapshotPrefix byte = 0x01
		rustControlPrefix  byte = 0x02

		rustControlSessionEnded    byte = 0x01
		rustControlSnapshotRequest byte = 0x02
	)

	if PrefixOp != rustOpPrefix {
		t.Fatalf("PrefixOp = %#x, rust OP_PREFIX = %#x — protocol drift", PrefixOp, rustOpPrefix)
	}
	if PrefixSnapshot != rustSnapshotPrefix {
		t.Fatalf("PrefixSnapshot = %#x, rust SNAPSHOT_PREFIX = %#x — protocol drift", PrefixSnapshot, rustSnapshotPrefix)
	}
	if PrefixControl != rustControlPrefix {
		t.Fatalf("PrefixControl = %#x, rust PREFIX_CONTROL = %#x — protocol drift", PrefixControl, rustControlPrefix)
	}
	if ControlSessionEnded != rustControlSessionEnded {
		t.Fatalf("ControlSessionEnded = %#x, rust CONTROL_SESSION_ENDED = %#x — protocol drift", ControlSessionEnded, rustControlSessionEnded)
	}
	if ControlSnapshotRequest != rustControlSnapshotRequest {
		t.Fatalf("ControlSnapshotRequest = %#x, rust CONTROL_SNAPSHOT_REQUEST = %#x — protocol drift", ControlSnapshotRequest, rustControlSnapshotRequest)
	}
}
