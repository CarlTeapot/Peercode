package wire

import (
	"bytes"
	"testing"
)

func TestPrefixConstantsMatchRustSource(t *testing.T) {
	const (
		rustOpPrefix       byte = 0x00
		rustSnapshotPrefix byte = 0x01
		rustControlPrefix  byte = 0x02

		rustControlSessionEnded    byte = 0x01
		rustControlSnapshotRequest byte = 0x02

		rustPrefixGcCommit byte = 0x04
		rustPrefixPresence byte = 0x05
		rustPrefixSvReport byte = 0x06
		rustPresenceJoined byte = 0x01
		rustPresenceLeft   byte = 0x02
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
	if PrefixGcCommit != rustPrefixGcCommit {
		t.Fatalf("PrefixGcCommit = %#x, rust PREFIX_GC_COMMIT = %#x — protocol drift", PrefixGcCommit, rustPrefixGcCommit)
	}
	if PrefixPresence != rustPrefixPresence {
		t.Fatalf("PrefixPresence = %#x, rust PREFIX_PRESENCE = %#x — protocol drift", PrefixPresence, rustPrefixPresence)
	}
	if PrefixSvReport != rustPrefixSvReport {
		t.Fatalf("PrefixSvReport = %#x, rust PREFIX_SV_REPORT = %#x — protocol drift", PrefixSvReport, rustPrefixSvReport)
	}
	if PresenceJoined != rustPresenceJoined {
		t.Fatalf("PresenceJoined = %#x, rust PRESENCE_JOINED = %#x — protocol drift", PresenceJoined, rustPresenceJoined)
	}
	if PresenceLeft != rustPresenceLeft {
		t.Fatalf("PresenceLeft = %#x, rust PRESENCE_LEFT = %#x — protocol drift", PresenceLeft, rustPresenceLeft)
	}
}

func TestPresenceLayoutMatchesRustSource(t *testing.T) {
	// Must equal crdt-core/src/wire encode_presence for the same inputs; the Rust
	// drift test pins the identical expected bytes.
	got := EncodePresenceFrame(0x0102030405060708, PresenceJoined)
	want := []byte{PrefixPresence, PresenceJoined, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08}
	if !bytes.Equal(got, want) {
		t.Fatalf("EncodePresenceFrame layout = %x, want %x — protocol drift", got, want)
	}
}
