package wire

import (
	"bytes"
	"errors"
	"testing"
)

func TestDecodeOpFrame_Valid(t *testing.T) {
	frame := []byte{PrefixOp, 0xDE, 0xAD, 0xBE, 0xEF}
	payload, err := DecodeOpFrame(frame)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !bytes.Equal(payload, []byte{0xDE, 0xAD, 0xBE, 0xEF}) {
		t.Fatalf("payload = %x, want DEADBEEF", payload)
	}
}

func TestDecodeOpFrame_EmptyPayloadIsValid(t *testing.T) {
	payload, err := DecodeOpFrame([]byte{PrefixOp})
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if len(payload) != 0 {
		t.Fatalf("payload = %x, want empty", payload)
	}
}

func TestDecodeOpFrame_EmptyFrame(t *testing.T) {
	for _, frame := range [][]byte{nil, {}} {
		if _, err := DecodeOpFrame(frame); !errors.Is(err, ErrEmptyFrame) {
			t.Fatalf("err = %v, want ErrEmptyFrame", err)
		}
	}
}

func TestDecodeOpFrame_UnknownPrefix(t *testing.T) {
	_, err := DecodeOpFrame([]byte{0xFF, 0x00})
	if !errors.Is(err, ErrUnknownPrefix) {
		t.Fatalf("err = %v, want ErrUnknownPrefix", err)
	}
	if msg := err.Error(); !bytes.Contains([]byte(msg), []byte("0xFF")) {
		t.Fatalf("err message = %q, want to contain 0xFF", msg)
	}
}

func TestDecodeOpFrame_SnapshotPrefixRejectsAsOp(t *testing.T) {
	_, err := DecodeOpFrame([]byte{PrefixSnapshot, 0x00})
	if err == nil {
		t.Fatal("expected error for snapshot prefix, got nil")
	}
}

func TestEncodeOpFrame_RoundTrip(t *testing.T) {
	payload := []byte{0x01, 0x02, 0x03}
	frame := EncodeOpFrame(payload)
	if frame[0] != PrefixOp {
		t.Fatalf("frame[0] = %#x, want PrefixOp", frame[0])
	}
	got, err := DecodeOpFrame(frame)
	if err != nil {
		t.Fatalf("unexpected error: %v", err)
	}
	if !bytes.Equal(got, payload) {
		t.Fatalf("payload = %x, want %x", got, payload)
	}
}

func TestValidateFrame_AcceptsOpAndSnapshot(t *testing.T) {
	if err := ValidateFrame([]byte{PrefixOp, 0x00}); err != nil {
		t.Fatalf("ValidateFrame(op) = %v, want nil", err)
	}
	if err := ValidateFrame([]byte{PrefixSnapshot, 0x00}); err != nil {
		t.Fatalf("ValidateFrame(snapshot) = %v, want nil", err)
	}
}

func TestValidateFrame_AcceptsGcCommitAndSvReport(t *testing.T) {
	if err := ValidateFrame([]byte{PrefixGcCommit, 0x00}); err != nil {
		t.Fatalf("ValidateFrame(gc_commit) = %v, want nil", err)
	}
	if err := ValidateFrame([]byte{PrefixSvReport, 0x00}); err != nil {
		t.Fatalf("ValidateFrame(sv_report) = %v, want nil", err)
	}
}

func TestValidateFrame_RejectsInboundMembership(t *testing.T) {
	// Membership is gateway-authored only; a client must not be able to inject one.
	if err := ValidateFrame([]byte{PrefixMembership, MembershipJoined}); !errors.Is(err, ErrUnknownPrefix) {
		t.Fatalf("ValidateFrame(membership) = %v, want ErrUnknownPrefix", err)
	}
}

func TestValidateFrame_AcceptsPermission(t *testing.T) {
	if err := ValidateFrame(EncodePermissionFrame(1, true)); err != nil {
		t.Fatalf("ValidateFrame(permission) = %v, want nil", err)
	}
}

func TestValidateFrame_RejectsInboundPeerInfo(t *testing.T) {
	frame, err := EncodePeerInfoFrame(1, false, true, "alice")
	if err != nil {
		t.Fatalf("EncodePeerInfoFrame: %v", err)
	}
	if err := ValidateFrame(frame); !errors.Is(err, ErrUnknownPrefix) {
		t.Fatalf("ValidateFrame(peer-info) = %v, want ErrUnknownPrefix", err)
	}
}

func TestPermissionFrame_RoundTrip(t *testing.T) {
	for _, canWrite := range []bool{true, false} {
		frame := EncodePermissionFrame(0x0102030405060708, canWrite)
		id, got, err := DecodePermissionFrame(frame)
		if err != nil {
			t.Fatalf("DecodePermissionFrame: %v", err)
		}
		if id != 0x0102030405060708 || got != canWrite {
			t.Fatalf("round trip = (%#x, %v), want (0x0102030405060708, %v)", id, got, canWrite)
		}
	}
}

func TestDecodePermissionFrame_Rejects(t *testing.T) {
	if _, _, err := DecodePermissionFrame(nil); !errors.Is(err, ErrEmptyFrame) {
		t.Fatalf("nil frame err = %v, want ErrEmptyFrame", err)
	}
	if _, _, err := DecodePermissionFrame([]byte{PrefixOp, 1}); !errors.Is(err, ErrNotAPermission) {
		t.Fatalf("op frame err = %v, want ErrNotAPermission", err)
	}
	if _, _, err := DecodePermissionFrame([]byte{PrefixPermission, 1, 0}); !errors.Is(err, ErrMalformedFrame) {
		t.Fatalf("short frame err = %v, want ErrMalformedFrame", err)
	}
	bad := EncodePermissionFrame(1, false)
	bad[1] = 0x02
	if _, _, err := DecodePermissionFrame(bad); !errors.Is(err, ErrMalformedFrame) {
		t.Fatalf("bad flag err = %v, want ErrMalformedFrame", err)
	}
}

func TestEncodePeerInfoFrame_Layout(t *testing.T) {
	frame, err := EncodePeerInfoFrame(0x0102030405060708, true, true, "ab")
	if err != nil {
		t.Fatalf("EncodePeerInfoFrame: %v", err)
	}
	want := []byte{
		PrefixPeerInfo, PeerFlagHost | PeerFlagCanWrite,
		0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
		0x02, 'a', 'b',
	}
	if !bytes.Equal(frame, want) {
		t.Fatalf("frame = %x, want %x", frame, want)
	}
}

func TestEncodePeerInfoFrame_RejectsOversizedUsername(t *testing.T) {
	long := bytes.Repeat([]byte{'x'}, MaxUsernameBytes+1)
	if _, err := EncodePeerInfoFrame(1, false, false, string(long)); !errors.Is(err, ErrUsernameTooLong) {
		t.Fatalf("err = %v, want ErrUsernameTooLong", err)
	}
}

func TestIsOpAndPermissionFrameHelpers(t *testing.T) {
	if !IsOpFrame([]byte{PrefixOp, 0x01}) || IsOpFrame([]byte{PrefixSnapshot}) || IsOpFrame(nil) {
		t.Fatal("IsOpFrame misclassified a frame")
	}
	if !IsPermissionFrame(EncodePermissionFrame(1, true)) || IsPermissionFrame([]byte{PrefixOp}) || IsPermissionFrame(nil) {
		t.Fatal("IsPermissionFrame misclassified a frame")
	}
}

func TestValidateFrame_RejectsUnknown(t *testing.T) {
	if err := ValidateFrame([]byte{0xFF}); !errors.Is(err, ErrUnknownPrefix) {
		t.Fatalf("ValidateFrame(0xFF) = %v, want ErrUnknownPrefix", err)
	}
}

func TestValidateFrame_RejectsEmpty(t *testing.T) {
	if err := ValidateFrame(nil); !errors.Is(err, ErrEmptyFrame) {
		t.Fatalf("ValidateFrame(nil) = %v, want ErrEmptyFrame", err)
	}
}

func TestIsSnapshotFrame(t *testing.T) {
	if !IsSnapshotFrame([]byte{PrefixSnapshot, 0x01}) {
		t.Fatal("IsSnapshotFrame should return true for snapshot prefix")
	}
	if IsSnapshotFrame([]byte{PrefixOp, 0x01}) {
		t.Fatal("IsSnapshotFrame should return false for op prefix")
	}
	if IsSnapshotFrame(nil) {
		t.Fatal("IsSnapshotFrame should return false for nil")
	}
}
