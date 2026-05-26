package wire

import (
	"errors"
	"fmt"
)

const (
	PrefixOp       byte = 0x00
	PrefixSnapshot byte = 0x01
	PrefixControl  byte = 0x02
)

const (
	ControlSessionEnded    byte = 0x01
	ControlSnapshotRequest byte = 0x02
)

func EncodeControlFrame(controlType byte) []byte {
	return []byte{PrefixControl, controlType}
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
	case PrefixOp, PrefixSnapshot:
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

func EncodeOpFrame(payload []byte) []byte {
	out := make([]byte, 1+len(payload))
	out[0] = PrefixOp
	copy(out[1:], payload)
	return out
}
