package wire

import (
	"encoding/json"
	"errors"
	"fmt"
)

const (
	PrefixOp       byte = 0x00
	PrefixSnapshot byte = 0x01
	PrefixControl  byte = 0x02
)

const (
	ControlSessionEnded     byte = 0x01
	ControlRoomState        byte = 0x02
	ControlPermissionChange byte = 0x03
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
	case PrefixOp, PrefixSnapshot, PrefixControl:
		return nil
	default:
		return fmt.Errorf("%w: 0x%02X", ErrUnknownPrefix, frame[0])
	}
}

func IsControlFrame(frame []byte) bool {
	return len(frame) >= 2 && frame[0] == PrefixControl
}

func ControlSubType(frame []byte) byte {
	if len(frame) < 2 {
		return 0
	}
	return frame[1]
}

func EncodeControlJSON(subType byte, payload any) ([]byte, error) {
	jsonBytes, err := json.Marshal(payload)
	if err != nil {
		return nil, fmt.Errorf("wire: marshal control payload: %w", err)
	}
	frame := append([]byte{PrefixControl, subType}, jsonBytes...)
	return frame, nil
}

func DecodeControlJSON(frame []byte, out any) error {
	if len(frame) < 2 {
		return ErrEmptyFrame
	}
	if frame[0] != PrefixControl {
		return fmt.Errorf("wire: not a control frame")
	}
	return json.Unmarshal(frame[2:], out)
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
