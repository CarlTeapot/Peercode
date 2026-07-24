package room

import (
	"errors"
	"log/slog"
	"strconv"
	"sync"
	"time"

	"gateway/internal/client"
	gatewaymetrics "gateway/internal/metrics"
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

	mu              sync.Mutex
	clients         map[string]*client.Client
	closed          bool
	host            *client.Client
	defaultCanWrite bool

	snapshotRequests []chan []byte

	ops  chan BroadcastMsg
	done chan struct{}

	metrics *gatewaymetrics.Registry
}

func New(id string, registry *gatewaymetrics.Registry) *Room {
	return &Room{
		ID:      id,
		clients: make(map[string]*client.Client),
		ops:     make(chan BroadcastMsg, opsBufferSize),
		done:    make(chan struct{}),
		metrics: registry,
	}
}

func (r *Room) Join(c *client.Client) error {
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.closed {
		slog.Warn("join rejected: room already closed", "room_id", r.ID, "client_id", c.ID)
		return ErrRoomClosed
	}
	if _, exists := r.clients[c.ID]; exists {
		slog.Warn("join rejected: duplicate client_id", "room_id", r.ID, "client_id", c.ID)
		return ErrDuplicateClientID
	}
	r.clients[c.ID] = c
	isHost := r.host == nil
	if isHost {
		r.host = c
		c.SetRole(client.RoleHost)
		c.SetCanWrite(true)
		r.defaultCanWrite = c.HostDefaultCanWrite
		slog.Info("host client registered", "room_id", r.ID, "client_id", c.ID, "default_can_write", r.defaultCanWrite)
	} else {
		c.SetRole(client.RoleGuest)
		c.SetCanWrite(r.defaultCanWrite)
	}
	r.metrics.ClientJoined(isHost)
	// Notify existing members (incl. the host) that c joined.
	r.broadcastMembershipLocked(c.ID, wire.MembershipJoined, c)
	r.sendRosterLocked(c)
	r.announcePeerLocked(c)
	slog.Info("join accepted", "room_id", r.ID, "client_id", c.ID, "can_write", c.CanWrite(), "size", len(r.clients))
	return nil
}

func (r *Room) Leave(c *client.Client, onEmpty func()) {
	r.mu.Lock()
	if current, ok := r.clients[c.ID]; !ok || current != c {
		slog.Debug("leave ignored: client not present in room", "room_id", r.ID, "client_id", c.ID)
		r.mu.Unlock()
		return
	}
	delete(r.clients, c.ID)
	wasHost := r.host == c
	if wasHost {
		r.host = nil
		c.SetRole(client.RoleGuest)
	}
	r.metrics.ClientLeft(wasHost)
	c.CloseSend()
	// Notify remaining members that c left (c is already removed).
	r.broadcastMembershipLocked(c.ID, wire.MembershipLeft, nil)
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
		r.metrics.ReplayFailed()
		r.removeSnapshotRequest(waiter)
		slog.Warn("replay: failed to request snapshot from host", "room_id", r.ID, "client_id", c.ID, "host_id", host.ID)
		return false
	}
	slog.Info("replay: snapshot requested from host", "room_id", r.ID, "client_id", c.ID, "host_id", host.ID)

	var snap []byte
	select {
	case snap = <-waiter:
	case <-time.After(snapshotResponseTimeout):
		r.metrics.ReplayFailed()
		r.removeSnapshotRequest(waiter)
		slog.Warn("replay: timed out waiting for host snapshot", "room_id", r.ID, "client_id", c.ID, "host_id", host.ID)
		return false
	}

	if !c.Send(snap) {
		r.metrics.ReplayFailed()
		r.removeSnapshotRequest(waiter)
		slog.Warn("replay: failed to send snapshot to joiner", "room_id", r.ID, "client_id", c.ID)
		return false
	}
	r.metrics.ReplaySucceeded()
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
			r.dispatch(msg)
		}
	}
}

func (r *Room) dispatch(msg BroadcastMsg) {
	switch {
	case wire.IsSnapshotFrame(msg.Data):
		if r.deliverSnapshotResponse(msg.Sender, msg.Data) {
			return
		}
		if msg.Sender.IsHost() {
			slog.Info("host snapshot broadcast to room", "room_id", r.ID, "bytes", len(msg.Data))
			r.broadcast(msg)
			return
		}
		slog.Warn("snapshot frame dropped: not from host", "room_id", r.ID, "client_id", msg.Sender.ID, "bytes", len(msg.Data))
	case wire.IsPermissionFrame(msg.Data):
		r.handlePermissionChange(msg)
	default:
		r.broadcast(msg)
	}
}

func (r *Room) handlePermissionChange(msg BroadcastMsg) {
	if !msg.Sender.IsHost() {
		slog.Warn("permission change dropped from non-host sender", "room_id", r.ID, "client_id", msg.Sender.ID)
		return
	}
	targetID, canWrite, err := wire.DecodePermissionFrame(msg.Data)
	if err != nil {
		slog.Warn("permission change dropped: decode failed", "room_id", r.ID, "error", err)
		return
	}
	target := r.clientByID(strconv.FormatUint(targetID, 10))
	if target == nil {
		slog.Warn("permission change dropped: unknown target", "room_id", r.ID, "target_id", targetID)
		return
	}
	if target.IsHost() {
		slog.Warn("permission change dropped: host permission is immutable", "room_id", r.ID, "target_id", targetID)
		return
	}
	target.SetCanWrite(canWrite)
	slog.Info("permission updated", "room_id", r.ID, "target_id", targetID, "can_write", canWrite)
	r.BroadcastAll(msg.Data)
}

func (r *Room) clientByID(id string) *client.Client {
	r.mu.Lock()
	defer r.mu.Unlock()
	return r.clients[id]
}

func (r *Room) broadcast(msg BroadcastMsg) {
	if wire.IsGcCommitFrame(msg.Data) && !msg.Sender.IsHost() {
		slog.Warn(
			"gc-commit dropped from non-host sender",
			"room_id", r.ID,
			"client_id", msg.Sender.ID,
			"bytes", len(msg.Data),
		)
		return
	}
	if wire.IsOpFrame(msg.Data) && !msg.Sender.CanWrite() {
		slog.Warn(
			"op dropped from read-only sender",
			"room_id", r.ID,
			"client_id", msg.Sender.ID,
			"bytes", len(msg.Data),
		)
		return
	}

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

// broadcastMembershipLocked sends a membership frame for subjectID to all clients
// except `exclude`. Must hold r.mu (Send is a non-blocking push, no re-lock); a
// non-numeric subjectID is skipped rather than failing the join/leave.
func (r *Room) broadcastMembershipLocked(subjectID string, event byte, exclude *client.Client) {
	id, err := strconv.ParseUint(subjectID, 10, 64)
	if err != nil {
		slog.Warn("membership: non-numeric client_id; skipping membership", "room_id", r.ID, "client_id", subjectID, "error", err)
		return
	}
	frame := wire.EncodeMembershipFrame(id, event)
	for _, cl := range r.clients {
		if cl == exclude {
			continue
		}
		if !cl.Send(frame) {
			slog.Warn("membership: send failed (slow client); force-closing", "room_id", r.ID, "client_id", cl.ID)
			cl.ForceClose()
		}
	}
}

func (r *Room) sendRosterLocked(joiner *client.Client) {
	for _, member := range r.clients {
		frame, ok := encodePeerInfo(member)
		if !ok {
			continue
		}
		if !joiner.Send(frame) {
			slog.Warn("roster: send failed (slow client); force-closing", "room_id", r.ID, "client_id", joiner.ID)
			joiner.ForceClose()
			return
		}
	}
}

func (r *Room) announcePeerLocked(joiner *client.Client) {
	frame, ok := encodePeerInfo(joiner)
	if !ok {
		return
	}
	for _, member := range r.clients {
		if member == joiner {
			continue
		}
		if !member.Send(frame) {
			slog.Warn("peer-info: send failed (slow client); force-closing", "room_id", r.ID, "client_id", member.ID)
			member.ForceClose()
		}
	}
}

func encodePeerInfo(c *client.Client) (frame []byte, ok bool) {
	id, err := strconv.ParseUint(c.ID, 10, 64)
	if err != nil {
		slog.Warn("peer-info: non-numeric client_id; skipping", "client_id", c.ID, "error", err)
		return nil, false
	}
	frame, err = wire.EncodePeerInfoFrame(id, c.IsHost(), c.CanWrite(), c.Username)
	if err != nil {
		slog.Warn("peer-info: encode failed; skipping", "client_id", c.ID, "error", err)
		return nil, false
	}
	return frame, true
}

func (r *Room) getPeers(exclude *client.Client) []*client.Client {
	r.mu.Lock()
	defer r.mu.Unlock()
	targets := make([]*client.Client, 0, len(r.clients))
	for _, c := range r.clients {
		if exclude == nil || c != exclude {
			targets = append(targets, c)
		}
	}
	return targets
}

func (r *Room) sendToPeers(targets []*client.Client, data []byte, slowClientLogMsg string) {
	for _, c := range targets {
		if !c.Send(data) {
			if c.ForceClose() {
				r.metrics.SlowClientDisconnected()
				slog.Warn(slowClientLogMsg, "room_id", r.ID, "client_id", c.ID)
			}
		} else {
			r.metrics.MessageRelayed(len(data))
		}
	}
}

func (r *Room) drain() {
	drained := 0
	for {
		select {
		case msg := <-r.ops:
			drained++
			r.dispatch(msg)
		default:
			slog.Debug("room drain finished", "room_id", r.ID, "drained_messages", drained)
			return
		}
	}
}
