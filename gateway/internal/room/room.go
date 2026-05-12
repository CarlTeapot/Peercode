package room

import (
	"log/slog"
	"sync"

	"gateway/internal/client"
)

const opsBufferSize = 256

type BroadcastMsg struct {
	Sender *client.Client
	Data   []byte
}

type Room struct {
	ID string

	mu      sync.Mutex
	clients map[*client.Client]struct{}
	closed  bool

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

func (r *Room) Join(c *client.Client) bool {
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.closed {
		slog.Warn("join rejected: room already closed", "room_id", r.ID, "client_id", c.ID)
		return false
	}
	r.clients[c] = struct{}{}
	slog.Info("join accepted", "room_id", r.ID, "client_id", c.ID, "size", len(r.clients))
	return true
}

func (r *Room) Leave(c *client.Client, onEmpty func()) {
	r.mu.Lock()
	if _, ok := r.clients[c]; !ok {
		slog.Debug("leave ignored: client not present in room", "room_id", r.ID, "client_id", c.ID)
		r.mu.Unlock()
		return
	}
	delete(r.clients, c)
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
			r.broadcast(msg)
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

func (r *Room) BroadcastAll(data []byte) {
	targets := r.getPeers(nil)
	r.sendToPeers(targets, data, "slow client during BroadcastAll; force-closing")
}

func (r *Room) getPeers(exclude *client.Client) []*client.Client {
	r.mu.Lock()
	targets := make([]*client.Client, 0, len(r.clients))
	for c := range r.clients {
		if exclude == nil || c != exclude {
			targets = append(targets, c)
		}
	}
	r.mu.Unlock()
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
			r.broadcast(msg)
		default:
			slog.Debug("room drain finished", "room_id", r.ID, "drained_messages", drained)
			return
		}
	}
}
