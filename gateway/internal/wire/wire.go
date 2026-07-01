package wire

import (
	"encoding/binary"
	"errors"
	"fmt"
)

const (
	PrefixOp         byte = 0x00
	PrefixSnapshot   byte = 0x01
	PrefixControl    byte = 0x02
	PrefixGcCommit   byte = 0x04
	PrefixMembership byte = 0x05
	PrefixSvReport   byte = 0x06
)

const (
	ControlSessionEnded    byte = 0x01
	ControlSnapshotRequest byte = 0x02
)

const (
	MembershipJoined byte = 0x01
	MembershipLeft   byte = 0x02
)

func EncodeControlFrame(controlType byte) []byte {
	return []byte{PrefixControl, controlType}
}

func EncodeMembershipFrame(clientID uint64, event byte) []byte {
	out := make([]byte, 10)
	out[0] = PrefixMembership
	out[1] = event
	binary.BigEndian.PutUint64(out[2:], clientID)
	return out
}

var (
	ErrEmptyFrame    = errors.New("wire: empty frame")
	ErrUnknownPrefix = errors.New("wire: unknown prefix")
)

func ValidateFrame(frame []byte) error {
	if len(frame) == 0 {
		return ErrEmptyFrame
	}
	switch frame[0] {
	// PrefixMembership is gateway-authored only; an inbound one is rejected.
	case PrefixOp, PrefixSnapshot, PrefixGcCommit, PrefixSvReport:
		return nil
	default:
		return fmt.Errorf("%w: 0x%02X", ErrUnknownPrefix, frame[0])
	}
}

func DecodeOpFrame(frame []byte) ([]byte, error) {
	if len(frame) == 0 {
		return nil, ErrEmptyFrame
	}
	switch frame[0] {
	case PrefixOp:
		return frame[1:], nil
	case PrefixSnapshot:
		return nil, fmt.Errorf("wire: expected op frame, got snapshot")
	default:
		return nil, fmt.Errorf("%w: 0x%02X", ErrUnknownPrefix, frame[0])
	}
}

func IsSnapshotFrame(frame []byte) bool {
	return len(frame) > 0 && frame[0] == PrefixSnapshot
}

func IsGcCommitFrame(frame []byte) bool {
	return len(frame) > 0 && frame[0] == PrefixGcCommit
}

func EncodeOpFrame(payload []byte) []byte {
	out := make([]byte, 1+len(payload))
	out[0] = PrefixOp
	copy(out[1:], payload)
	return out
}
