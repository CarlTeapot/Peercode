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

func TestValidateFrame_RejectsInboundPresence(t *testing.T) {
	// Presence is gateway-authored only; a client must not be able to inject one.
	if err := ValidateFrame([]byte{PrefixPresence, PresenceJoined}); !errors.Is(err, ErrUnknownPrefix) {
		t.Fatalf("ValidateFrame(presence) = %v, want ErrUnknownPrefix", err)
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
