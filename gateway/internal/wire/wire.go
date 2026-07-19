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
	PrefixPermission byte = 0x07
	PrefixPeerInfo   byte = 0x08
)

const (
	ControlSessionEnded    byte = 0x01
	ControlSnapshotRequest byte = 0x02
)

const (
	MembershipJoined byte = 0x01
	MembershipLeft   byte = 0x02
)

const (
	PeerFlagHost     byte = 0x01
	PeerFlagCanWrite byte = 0x02
)

const MaxUsernameBytes = 255

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
	ErrEmptyFrame      = errors.New("wire: empty frame")
	ErrUnknownPrefix   = errors.New("wire: unknown prefix")
	ErrMalformedFrame  = errors.New("wire: malformed frame")
	ErrUsernameTooLong = errors.New("wire: username exceeds max bytes")
	ErrNotAPermission  = errors.New("wire: not a permission frame")
)

func ValidateFrame(frame []byte) error {
	if len(frame) == 0 {
		return ErrEmptyFrame
	}
	switch frame[0] {
	case PrefixOp, PrefixSnapshot, PrefixGcCommit, PrefixSvReport, PrefixPermission:
		return nil
	default:
		return fmt.Errorf("%w: 0x%02X", ErrUnknownPrefix, frame[0])
	}
}

func IsOpFrame(frame []byte) bool {
	return len(frame) > 0 && frame[0] == PrefixOp
}

func IsPermissionFrame(frame []byte) bool {
	return len(frame) > 0 && frame[0] == PrefixPermission
}

func EncodePermissionFrame(clientID uint64, canWrite bool) []byte {
	out := make([]byte, 10)
	out[0] = PrefixPermission
	if canWrite {
		out[1] = 1
	}
	binary.BigEndian.PutUint64(out[2:], clientID)
	return out
}

func DecodePermissionFrame(frame []byte) (clientID uint64, canWrite bool, err error) {
	if len(frame) == 0 {
		return 0, false, ErrEmptyFrame
	}
	if frame[0] != PrefixPermission {
		return 0, false, ErrNotAPermission
	}
	if len(frame) != 10 || frame[1] > 1 {
		return 0, false, fmt.Errorf("%w: permission frame", ErrMalformedFrame)
	}
	return binary.BigEndian.Uint64(frame[2:]), frame[1] == 1, nil
}

func EncodePeerInfoFrame(clientID uint64, isHost, canWrite bool, username string) ([]byte, error) {
	if len(username) > MaxUsernameBytes {
		return nil, ErrUsernameTooLong
	}
	var flags byte
	if isHost {
		flags |= PeerFlagHost
	}
	if canWrite {
		flags |= PeerFlagCanWrite
	}
	out := make([]byte, 0, 11+len(username))
	out = append(out, PrefixPeerInfo, flags)
	out = binary.BigEndian.AppendUint64(out, clientID)
	out = append(out, byte(len(username)))
	out = append(out, username...)
	return out, nil
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
