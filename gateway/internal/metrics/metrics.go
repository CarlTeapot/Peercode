package metrics

import (
	"sync/atomic"
	"time"
)

type Registry struct {
	startedAt time.Time

	activeRooms           atomic.Int64
	connectedClients      atomic.Int64
	activeHosts           atomic.Int64
	relayedMessages       atomic.Uint64
	relayedBytes          atomic.Uint64
	replaySuccesses       atomic.Uint64
	replayFailures        atomic.Uint64
	droppedFrames         atomic.Uint64
	slowClientDisconnects atomic.Uint64
}

type Response struct {
	Healthy               bool   `json:"healthy"`
	UptimeSeconds         uint64 `json:"uptime_seconds"`
	ActiveRooms           int64  `json:"active_rooms"`
	ConnectedClients      int64  `json:"connected_clients"`
	ActiveHosts           int64  `json:"active_hosts"`
	RelayedMessages       uint64 `json:"relayed_messages"`
	RelayedBytes          uint64 `json:"relayed_bytes"`
	ReplaySuccesses       uint64 `json:"replay_successes"`
	ReplayFailures        uint64 `json:"replay_failures"`
	DroppedFrames         uint64 `json:"dropped_frames"`
	SlowClientDisconnects uint64 `json:"slow_client_disconnects"`
}

func New() *Registry {
	return &Registry{startedAt: time.Now()}
}

func (r *Registry) RoomOpened() { r.activeRooms.Add(1) }
func (r *Registry) RoomClosed() { r.activeRooms.Add(-1) }

func (r *Registry) ClientJoined(isHost bool) {
	r.connectedClients.Add(1)
	if isHost {
		r.activeHosts.Add(1)
	}
}

func (r *Registry) ClientLeft(wasHost bool) {
	r.connectedClients.Add(-1)
	if wasHost {
		r.activeHosts.Add(-1)
	}
}

func (r *Registry) MessageRelayed(bytes int) {
	r.relayedMessages.Add(1)
	r.relayedBytes.Add(uint64(bytes))
}

func (r *Registry) ReplaySucceeded()        { r.replaySuccesses.Add(1) }
func (r *Registry) ReplayFailed()           { r.replayFailures.Add(1) }
func (r *Registry) FrameDropped()           { r.droppedFrames.Add(1) }
func (r *Registry) SlowClientDisconnected() { r.slowClientDisconnects.Add(1) }

func (r *Registry) Response() Response {
	return Response{
		Healthy:               true,
		UptimeSeconds:         uint64(time.Since(r.startedAt).Seconds()),
		ActiveRooms:           r.activeRooms.Load(),
		ConnectedClients:      r.connectedClients.Load(),
		ActiveHosts:           r.activeHosts.Load(),
		RelayedMessages:       r.relayedMessages.Load(),
		RelayedBytes:          r.relayedBytes.Load(),
		ReplaySuccesses:       r.replaySuccesses.Load(),
		ReplayFailures:        r.replayFailures.Load(),
		DroppedFrames:         r.droppedFrames.Load(),
		SlowClientDisconnects: r.slowClientDisconnects.Load(),
	}
}
