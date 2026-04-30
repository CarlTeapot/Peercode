package room

import (
	"log"
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
		return false
	}
	r.clients[c] = struct{}{}
	return true
}

func (r *Room) Leave(c *client.Client, onEmpty func()) {
	r.mu.Lock()
	if _, ok := r.clients[c]; !ok {
		r.mu.Unlock()
		return
	}
	delete(r.clients, c)
	c.CloseSend()
	empty := len(r.clients) == 0
	if empty {
		r.closed = true
		close(r.done)
	}
	r.mu.Unlock()

	if empty && onEmpty != nil {
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
	for {
		select {
		case <-r.done:
			r.drain()
			return
		case msg := <-r.ops:
			r.broadcast(msg)
		}
	}
}

func (r *Room) broadcast(msg BroadcastMsg) {
	r.mu.Lock()
	targets := make([]*client.Client, 0, len(r.clients))
	for c := range r.clients {
		if c != msg.Sender {
			targets = append(targets, c)
		}
	}
	r.mu.Unlock()

	for _, c := range targets {
		if !c.Send(msg.Data) {
			log.Printf("[room] %s: disconnecting slow client %s", r.ID, c.ID)
			c.ForceClose()
		}
	}
}

func (r *Room) drain() {
	for {
		select {
		case msg := <-r.ops:
			r.broadcast(msg)
		default:
			return
		}
	}
}
