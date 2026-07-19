package client

import (
	"context"
	"log/slog"
	"sync/atomic"

	"github.com/coder/websocket"
)

const sendBufferSize = 256

type Role int32

const (
	RoleGuest Role = iota
	RoleHost
)

type Client struct {
	ID       string
	RoomID   string
	Username string
	HostDefaultCanWrite bool
	conn                *websocket.Conn
	send                chan []byte
	closed              atomic.Bool
	role                atomic.Int32
	canWrite            atomic.Bool
}

func New(id, roomID, username string, conn *websocket.Conn) *Client {
	c := &Client{
		ID:       id,
		RoomID:   roomID,
		Username: username,
		conn:     conn,
		send:     make(chan []byte, sendBufferSize),
	}
	slog.Info("client created", "room_id", roomID, "client_id", id, "username", username, "send_buffer_size", sendBufferSize)
	return c
}

func (c *Client) CloseSend() {
	slog.Info("closing client send channel", "room_id", c.RoomID, "client_id", c.ID)
	close(c.send)
}

func (c *Client) SetRole(r Role) {
	c.role.Store(int32(r))
}

func (c *Client) Role() Role {
	return Role(c.role.Load())
}

func (c *Client) IsHost() bool {
	return c != nil && c.Role() == RoleHost
}

func (c *Client) SetCanWrite(v bool) {
	c.canWrite.Store(v)
}

func (c *Client) CanWrite() bool {
	return c != nil && c.canWrite.Load()
}

// returns false without blocking if the send buffer is full
func (c *Client) Send(data []byte) (ok bool) {
	defer func() {
		if recover() != nil {
			ok = false
		}
	}()
	select {
	case c.send <- data:
		return true
	default:
		return false
	}
}

// for testing only
func (c *Client) SendChan() <-chan []byte {
	return c.send
}

// ForceClose closes the websocket connection. It is idempotent: only the first
// call performs the close; subsequent calls are no-ops. Returns true on the
// first call so callers can count the real disconnect exactly once.
func (c *Client) ForceClose() (first bool) {
	if !c.closed.CompareAndSwap(false, true) {
		return false
	}
	if c.conn == nil {
		slog.Debug("force-close skipped: client connection is nil", "room_id", c.RoomID, "client_id", c.ID)
		return true
	}
	slog.Warn("force-closing client websocket due to slow consumer", "room_id", c.RoomID, "client_id", c.ID)
	go c.conn.Close(websocket.StatusPolicyViolation, "slow consumer")
	return true
}

// reads frames from the websocket and pushes each payload onto
func (c *Client) ReadPump(ctx context.Context, ops chan<- []byte, leave chan<- *Client) {
	slog.Info("read pump started", "room_id", c.RoomID, "client_id", c.ID)
	defer func() {
		slog.Debug("read pump exiting; signaling leave", "room_id", c.RoomID, "client_id", c.ID)
		select {
		case leave <- c:
			slog.Debug("read pump leave signal delivered", "room_id", c.RoomID, "client_id", c.ID)
		case <-ctx.Done():
			slog.Debug("read pump leave signal skipped due to cancelled context", "room_id", c.RoomID, "client_id", c.ID)
		}
	}()

	for {
		_, data, err := c.conn.Read(ctx)
		if err != nil {
			slog.Warn("read pump: connection read error; closing",
				"room_id", c.RoomID,
				"client_id", c.ID,
				"error", err,
			)
			return
		}
		select {
		case ops <- data:
			slog.Debug("read pump forwarded frame to hub", "room_id", c.RoomID, "client_id", c.ID, "bytes", len(data))
		case <-ctx.Done():
			slog.Debug("read pump context cancelled before forwarding frame", "room_id", c.RoomID, "client_id", c.ID)
			return
		}
	}
}

// drains the send channel to the websocket until it is closed
func (c *Client) WritePump(ctx context.Context) {
	slog.Info("write pump started", "room_id", c.RoomID, "client_id", c.ID)
	defer func() {
		_ = c.conn.Close(websocket.StatusNormalClosure, "")
		slog.Debug("write pump closed websocket connection", "room_id", c.RoomID, "client_id", c.ID)
	}()

	for {
		select {
		case msg, ok := <-c.send:
			if !ok {
				slog.Debug("write pump send channel closed", "room_id", c.RoomID, "client_id", c.ID)
				return
			}
			if err := c.conn.Write(ctx, websocket.MessageBinary, msg); err != nil {
				slog.Warn("write pump: connection write error; closing",
					"room_id", c.RoomID,
					"client_id", c.ID,
					"frame_bytes", len(msg),
					"error", err,
				)
				return
			}
			slog.Debug("write pump sent frame", "room_id", c.RoomID, "client_id", c.ID, "bytes", len(msg))
		case <-ctx.Done():
			slog.Debug("write pump context cancelled", "room_id", c.RoomID, "client_id", c.ID)
			return
		}
	}
}
