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

		rustPrefixGcCommit   byte = 0x04
		rustPrefixMembership byte = 0x05
		rustPrefixSvReport   byte = 0x06
		rustPrefixPermission byte = 0x07
		rustPrefixPeerInfo   byte = 0x08
		rustMembershipJoined byte = 0x01
		rustMembershipLeft   byte = 0x02
		rustPeerFlagHost     byte = 0x01
		rustPeerFlagCanWrite byte = 0x02
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
	if PrefixMembership != rustPrefixMembership {
		t.Fatalf("PrefixMembership = %#x, rust PREFIX_MEMBERSHIP = %#x — protocol drift", PrefixMembership, rustPrefixMembership)
	}
	if PrefixSvReport != rustPrefixSvReport {
		t.Fatalf("PrefixSvReport = %#x, rust PREFIX_SV_REPORT = %#x — protocol drift", PrefixSvReport, rustPrefixSvReport)
	}
	if MembershipJoined != rustMembershipJoined {
		t.Fatalf("MembershipJoined = %#x, rust PEER_JOINED = %#x — protocol drift", MembershipJoined, rustMembershipJoined)
	}
	if MembershipLeft != rustMembershipLeft {
		t.Fatalf("MembershipLeft = %#x, rust PEER_LEFT = %#x — protocol drift", MembershipLeft, rustMembershipLeft)
	}
	if PrefixPermission != rustPrefixPermission {
		t.Fatalf("PrefixPermission = %#x, rust PREFIX_PERMISSION = %#x — protocol drift", PrefixPermission, rustPrefixPermission)
	}
	if PrefixPeerInfo != rustPrefixPeerInfo {
		t.Fatalf("PrefixPeerInfo = %#x, rust PREFIX_PEER_INFO = %#x — protocol drift", PrefixPeerInfo, rustPrefixPeerInfo)
	}
	if PeerFlagHost != rustPeerFlagHost {
		t.Fatalf("PeerFlagHost = %#x, rust PEER_FLAG_HOST = %#x — protocol drift", PeerFlagHost, rustPeerFlagHost)
	}
	if PeerFlagCanWrite != rustPeerFlagCanWrite {
		t.Fatalf("PeerFlagCanWrite = %#x, rust PEER_FLAG_CAN_WRITE = %#x — protocol drift", PeerFlagCanWrite, rustPeerFlagCanWrite)
	}
}

func TestMembershipLayoutMatchesRustSource(t *testing.T) {
	// Must equal crdt-core/src/wire encode_membership for the same inputs; the Rust
	// drift test pins the identical expected bytes.
	got := EncodeMembershipFrame(0x0102030405060708, MembershipJoined)
	want := []byte{PrefixMembership, MembershipJoined, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08}
	if !bytes.Equal(got, want) {
		t.Fatalf("EncodeMembershipFrame layout = %x, want %x — protocol drift", got, want)
	}
}

func TestPermissionLayoutMatchesRustSource(t *testing.T) {
	got := EncodePermissionFrame(0x0102030405060708, true)
	want := []byte{PrefixPermission, 0x01, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08}
	if !bytes.Equal(got, want) {
		t.Fatalf("EncodePermissionFrame layout = %x, want %x — protocol drift", got, want)
	}
}

func TestPeerInfoLayoutMatchesRustSource(t *testing.T) {
	got, err := EncodePeerInfoFrame(0x0102030405060708, true, true, "ab")
	if err != nil {
		t.Fatalf("EncodePeerInfoFrame: %v", err)
	}
	want := []byte{
		PrefixPeerInfo, PeerFlagHost | PeerFlagCanWrite,
		0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
		0x02, 'a', 'b',
	}
	if !bytes.Equal(got, want) {
		t.Fatalf("EncodePeerInfoFrame layout = %x, want %x — protocol drift", got, want)
	}
}
