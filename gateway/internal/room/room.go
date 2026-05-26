package room

import (
	"errors"
	"log/slog"
	"sync"
	"time"

	"gateway/internal/client"
	"gateway/internal/wire"
)

var (
	ErrRoomClosed        = errors.New("room closed")
	ErrDuplicateClientID = errors.New("duplicate client_id")
)

const (
	opsBufferSize           = 256
	snapshotResponseTimeout = 5 * time.Second
)

type BroadcastMsg struct {
	Sender *client.Client
	Data   []byte
}

type Room struct {
	ID string

	mu      sync.Mutex
	clients map[*client.Client]struct{}
	closed  bool
	host    *client.Client

	snapshotRequests []chan []byte

	ops  chan BroadcastMsg
	done chan struct{}
}

func New(id string) *Room {
	return &Room{
		ID:      id,
		clients: make(map[*client.Client]struct{}),
		ops:     make(chan BroadcastMsg, opsBufferSize),
		done:    make(chan struct{}),
	}
}

func (r *Room) Join(c *client.Client) error {
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.closed {
		slog.Warn("join rejected: room already closed", "room_id", r.ID, "client_id", c.ID)
		return ErrRoomClosed
	}
	for existing := range r.clients {
		if existing.ID == c.ID {
			slog.Warn("join rejected: duplicate client_id", "room_id", r.ID, "client_id", c.ID)
			return ErrDuplicateClientID
		}
	}
	r.clients[c] = struct{}{}
	if r.host == nil {
		r.host = c
		slog.Info("host client registered", "room_id", r.ID, "client_id", c.ID)
	}
	slog.Info("join accepted", "room_id", r.ID, "client_id", c.ID, "size", len(r.clients))
	return nil
}

func (r *Room) Leave(c *client.Client, onEmpty func()) {
	r.mu.Lock()
	if _, ok := r.clients[c]; !ok {
		slog.Debug("leave ignored: client not present in room", "room_id", r.ID, "client_id", c.ID)
		r.mu.Unlock()
		return
	}
	delete(r.clients, c)
	if r.host == c {
		r.host = nil
	}
	c.CloseSend()
	empty := len(r.clients) == 0
	if empty {
		r.closed = true
		close(r.done)
		slog.Info("room marked closed after last client left", "room_id", r.ID)
	}
	r.mu.Unlock()

	if empty && onEmpty != nil {
		slog.Debug("invoking room onEmpty callback", "room_id", r.ID)
		onEmpty()
	}
}

func (r *Room) Ops() chan<- BroadcastMsg { return r.ops }

func (r *Room) Size() int {
	r.mu.Lock()
	defer r.mu.Unlock()
	return len(r.clients)
}

func (r *Room) ReplayTo(c *client.Client) bool {
	r.mu.Lock()
	host := r.host
	if host == nil || host == c {
		r.mu.Unlock()
		slog.Debug("replay skipped: no host available", "room_id", r.ID, "client_id", c.ID)
		return false
	}
	waiter := make(chan []byte, 1)
	r.snapshotRequests = append(r.snapshotRequests, waiter)
	r.mu.Unlock()

	if !host.Send(wire.EncodeControlFrame(wire.ControlSnapshotRequest)) {
		r.removeSnapshotRequest(waiter)
		slog.Warn("replay: failed to request snapshot from host", "room_id", r.ID, "client_id", c.ID, "host_id", host.ID)
		return false
	}
	slog.Info("replay: snapshot requested from host", "room_id", r.ID, "client_id", c.ID, "host_id", host.ID)

	var snap []byte
	select {
	case snap = <-waiter:
	case <-time.After(snapshotResponseTimeout):
		r.removeSnapshotRequest(waiter)
		slog.Warn("replay: timed out waiting for host snapshot", "room_id", r.ID, "client_id", c.ID, "host_id", host.ID)
		return false
	}

	if !c.Send(snap) {
		r.removeSnapshotRequest(waiter)
		slog.Warn("replay: failed to send snapshot to joiner", "room_id", r.ID, "client_id", c.ID)
		return false
	}
	slog.Info("replay: snapshot sent to joiner", "room_id", r.ID, "client_id", c.ID, "snapshot_bytes", len(snap))
	return true
}

func (r *Room) Run() {
	slog.Info("room loop started", "room_id", r.ID)
	for {
		select {
		case <-r.done:
			slog.Info("room loop stopping; draining queued ops", "room_id", r.ID)
			r.drain()
			slog.Info("room loop stopped", "room_id", r.ID)
			return
		case msg := <-r.ops:
			if wire.IsSnapshotFrame(msg.Data) {
				if !r.deliverSnapshotResponse(msg.Sender, msg.Data) {
					slog.Debug("snapshot frame dropped: no pending snapshot request", "room_id", r.ID, "client_id", msg.Sender.ID, "bytes", len(msg.Data))
				}
			} else {
				r.broadcast(msg)
			}
		}
	}
}

func (r *Room) broadcast(msg BroadcastMsg) {
	targets := r.getPeers(msg.Sender)

	slog.Debug(
		"broadcast dispatch prepared",
		"room_id", r.ID,
		"sender_id", msg.Sender.ID,
		"targets", len(targets),
		"bytes", len(msg.Data),
	)
	r.sendToPeers(targets, msg.Data, "disconnecting slow client")
}

func (r *Room) deliverSnapshotResponse(sender *client.Client, data []byte) bool {
	r.mu.Lock()
	if sender != r.host || len(r.snapshotRequests) == 0 {
		r.mu.Unlock()
		return false
	}
	waiter := r.snapshotRequests[0]
	r.snapshotRequests = r.snapshotRequests[1:]
	cp := make([]byte, len(data))
	copy(cp, data)
	r.mu.Unlock()

	waiter <- cp
	return true
}

func (r *Room) removeSnapshotRequest(waiter chan []byte) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.removeSnapshotRequestLocked(waiter)
}

func (r *Room) removeSnapshotRequestLocked(waiter chan []byte) {
	for i, pending := range r.snapshotRequests {
		if pending == waiter {
			r.snapshotRequests = append(r.snapshotRequests[:i], r.snapshotRequests[i+1:]...)
			return
		}
	}
}

func (r *Room) BroadcastAll(data []byte) {
	targets := r.getPeers(nil)
	r.sendToPeers(targets, data, "slow client during BroadcastAll; force-closing")
}

func (r *Room) getPeers(exclude *client.Client) []*client.Client {
	r.mu.Lock()
	defer r.mu.Unlock()
	targets := make([]*client.Client, 0, len(r.clients))
	for c := range r.clients {
		if exclude == nil || c != exclude {
			targets = append(targets, c)
		}
	}
	return targets
}

func (r *Room) sendToPeers(targets []*client.Client, data []byte, slowClientLogMsg string) {
	for _, c := range targets {
		if !c.Send(data) {
			slog.Warn(slowClientLogMsg, "room_id", r.ID, "client_id", c.ID)
			c.ForceClose()
		}
	}
}

func (r *Room) drain() {
	drained := 0
	for {
		select {
		case msg := <-r.ops:
			drained++
			if wire.IsSnapshotFrame(msg.Data) {
				_ = r.deliverSnapshotResponse(msg.Sender, msg.Data)
			} else {
				r.broadcast(msg)
			}
		default:
			slog.Debug("room drain finished", "room_id", r.ID, "drained_messages", drained)
			return
		}
	}
}
