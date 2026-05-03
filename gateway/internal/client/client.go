package client

import (
	"context"
	"log/slog"

	"github.com/coder/websocket"
)

const sendBufferSize = 256

type Client struct {
	ID     string
	RoomID string
	conn   *websocket.Conn
	send   chan []byte
}

func New(id, roomID string, conn *websocket.Conn) *Client {
	c := &Client{
		ID:     id,
		RoomID: roomID,
		conn:   conn,
		send:   make(chan []byte, sendBufferSize),
	}
	slog.Info("client created", "room_id", roomID, "client_id", id, "send_buffer_size", sendBufferSize)
	return c
}

func (c *Client) CloseSend() {
	slog.Info("closing client send channel", "room_id", c.RoomID, "client_id", c.ID)
	close(c.send)
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

func (c *Client) ForceClose() {
	if c.conn == nil {
		slog.Debug("force-close skipped: client connection is nil", "room_id", c.RoomID, "client_id", c.ID)
		return
	}
	slog.Warn("force-closing client websocket due to slow consumer", "room_id", c.RoomID, "client_id", c.ID)
	go c.conn.Close(websocket.StatusPolicyViolation, "slow consumer")
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
			slog.Debug("read pump stopped", "room_id", c.RoomID, "client_id", c.ID, "error", err)
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
				slog.Debug("write pump stopped", "room_id", c.RoomID, "client_id", c.ID, "error", err)
				return
			}
			slog.Debug("write pump sent frame", "room_id", c.RoomID, "client_id", c.ID, "bytes", len(msg))
		case <-ctx.Done():
			slog.Debug("write pump context cancelled", "room_id", c.RoomID, "client_id", c.ID)
			return
		}
	}
}
