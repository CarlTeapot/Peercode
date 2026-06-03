package room

import (
	"bytes"
	"errors"
	"log/slog"
	"os"
	"sync"
	"testing"
	"time"

	"gateway/internal/client"
	"gateway/internal/wire"
)

// drainFrames non-blockingly collects all frames currently queued on a client's
// (open) send channel.
func drainFrames(c *client.Client) [][]byte {
	var out [][]byte
	for {
		select {
		case f := <-c.SendChan():
			out = append(out, f)
		default:
			return out
		}
	}
}

func init() {
	slog.SetDefault(slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: slog.LevelError})))
}

func TestRoom_JoinLeaveTriggersOnEmpty(t *testing.T) {
	r := New("room-1")
	runDone := make(chan struct{})
	go func() { r.Run(); close(runDone) }()

	a := client.New("a", "room-1", nil)
	b := client.New("b", "room-1", nil)
	if err := r.Join(a); err != nil {
		t.Fatalf("join a: %v", err)
	}
	if err := r.Join(b); err != nil {
		t.Fatalf("join b: %v", err)
	}
	if got := r.Size(); got != 2 {
		t.Fatalf("Size=%d, want 2", got)
	}

	var emptied sync.WaitGroup
	emptied.Add(1)
	r.Leave(a, func() { t.Fatal("onEmpty fired with 1 member left") })
	r.Leave(b, func() { emptied.Done() })
	emptied.Wait()

	select {
	case <-runDone:
	case <-time.After(time.Second):
		t.Fatal("Run did not return after room emptied")
	}
}

func TestRoom_SendToEmptyIsNoop(t *testing.T) {
	r := New("room-2")
	runDone := make(chan struct{})
	go func() { r.Run(); close(runDone) }()

	a := client.New("a", "room-2", nil)
	if err := r.Join(a); err != nil {
		t.Fatalf("join: %v", err)
	}

	r.Ops() <- BroadcastMsg{Sender: a, Data: []byte{0x00, 0xDE, 0xAD}}
	r.Ops() <- BroadcastMsg{Sender: a, Data: []byte{}}

	r.Leave(a, nil)
	select {
	case <-runDone:
	case <-time.After(time.Second):
		t.Fatal("Run did not return after last client left")
	}
}

func TestRoom_DoubleLeaveIsSilent(t *testing.T) {
	r := New("room-3")
	runDone := make(chan struct{})
	go func() { r.Run(); close(runDone) }()

	a := client.New("a", "room-3", nil)
	if err := r.Join(a); err != nil {
		t.Fatalf("join: %v", err)
	}

	calls := 0
	r.Leave(a, func() { calls++ })
	r.Leave(a, func() { calls++ })
	if calls != 1 {
		t.Fatalf("onEmpty fired %d times, want exactly 1", calls)
	}

	select {
	case <-runDone:
	case <-time.After(time.Second):
		t.Fatal("Run did not return")
	}
}

func TestRoom_JoinAfterCloseIsRejected(t *testing.T) {
	r := New("room-4")
	runDone := make(chan struct{})
	go func() { r.Run(); close(runDone) }()

	a := client.New("a", "room-4", nil)
	if err := r.Join(a); err != nil {
		t.Fatalf("join: %v", err)
	}
	r.Leave(a, nil)

	b := client.New("b", "room-4", nil)
	if err := r.Join(b); !errors.Is(err, ErrRoomClosed) {
		t.Fatalf("Join on closed room: got %v, want ErrRoomClosed", err)
	}

	<-runDone
}

func TestRoom_SnapshotReplayToJoiner(t *testing.T) {
	r := New("room-snap")
	runDone := make(chan struct{})
	go func() { r.Run(); close(runDone) }()

	host := client.New("host", "room-snap", nil)
	if err := r.Join(host); err != nil {
		t.Fatalf("join host: %v", err)
	}

	joiner := client.New("joiner", "room-snap", nil)
	if err := r.Join(joiner); err != nil {
		t.Fatalf("join joiner: %v", err)
	}

	replayDone := make(chan bool, 1)
	go func() {
		replayDone <- r.ReplayTo(joiner)
	}()

	var request []byte
	select {
	case request = <-host.SendChan():
	case <-time.After(time.Second):
		t.Fatal("host did not receive snapshot request")
	}
	if len(request) != 2 || request[0] != wire.PrefixControl || request[1] != wire.ControlSnapshotRequest {
		t.Fatalf("snapshot request = %v, want control snapshot request", request)
	}

	op1 := []byte{0x00, 0x01}
	op2 := []byte{0x00, 0x02}
	r.Ops() <- BroadcastMsg{Sender: host, Data: op1}
	r.Ops() <- BroadcastMsg{Sender: host, Data: op2}
	time.Sleep(50 * time.Millisecond)

	snapFrame := []byte{0x01, 0xAA, 0xBB}
	r.Ops() <- BroadcastMsg{Sender: host, Data: snapFrame}

	var got bool
	select {
	case got = <-replayDone:
	case <-time.After(time.Second):
		t.Fatal("ReplayTo did not return")
	}
	if !got {
		t.Fatal("ReplayTo returned false; expected snapshot replay")
	}

	var received [][]byte
	for {
		select {
		case msg := <-joiner.SendChan():
			received = append(received, msg)
		default:
			goto done
		}
	}
done:
	if len(received) != 3 {
		t.Fatalf("joiner received %d messages, want 3 (2 ops + snapshot)", len(received))
	}
	if received[0][0] != 0x00 || received[1][0] != 0x00 {
		t.Fatalf("op messages have wrong prefix")
	}
	if received[2][0] != 0x01 {
		t.Fatalf("third message prefix = 0x%02X, want 0x01 (snapshot)", received[2][0])
	}

	r.Leave(host, nil)
	r.Leave(joiner, nil)
	<-runDone
}

func TestRoom_DuplicateClientIDRejected(t *testing.T) {
	r := New("room-dup")
	runDone := make(chan struct{})
	go func() { r.Run(); close(runDone) }()

	a := client.New("same", "room-dup", nil)
	b := client.New("same", "room-dup", nil)
	if err := r.Join(a); err != nil {
		t.Fatalf("first join: %v", err)
	}
	if err := r.Join(b); !errors.Is(err, ErrDuplicateClientID) {
		t.Fatalf("second join: got %v, want ErrDuplicateClientID", err)
	}

	r.Leave(a, nil)
	<-runDone
}

func TestRoom_JoinBroadcastsPresenceToExistingMembers(t *testing.T) {
	r := New("room-presence-join")
	host := client.New("1", "room-presence-join", nil)
	guest := client.New("2", "room-presence-join", nil)

	if err := r.Join(host); err != nil {
		t.Fatalf("join host: %v", err)
	}
	if got := drainFrames(host); len(got) != 0 {
		t.Fatalf("first joiner received %d frames, want 0", len(got))
	}

	if err := r.Join(guest); err != nil {
		t.Fatalf("join guest: %v", err)
	}

	hostFrames := drainFrames(host)
	if len(hostFrames) != 1 {
		t.Fatalf("host received %d frames, want 1 (joined guest)", len(hostFrames))
	}
	want := wire.EncodePresenceFrame(2, wire.PresenceJoined)
	if !bytes.Equal(hostFrames[0], want) {
		t.Fatalf("host received %x, want presence-joined for client 2 %x", hostFrames[0], want)
	}

	if got := drainFrames(guest); len(got) != 0 {
		t.Fatalf("joiner received its own presence (%d frames), want 0", len(got))
	}
}

func TestRoom_LeaveBroadcastsPresenceToRemaining(t *testing.T) {
	r := New("room-presence-leave")
	host := client.New("1", "room-presence-leave", nil)
	guest := client.New("2", "room-presence-leave", nil)
	_ = r.Join(host)
	_ = r.Join(guest)
	_ = drainFrames(host) // discard joined(guest)

	r.Leave(guest, nil)

	hostFrames := drainFrames(host)
	if len(hostFrames) != 1 {
		t.Fatalf("host received %d frames after guest left, want 1 (left guest)", len(hostFrames))
	}
	want := wire.EncodePresenceFrame(2, wire.PresenceLeft)
	if !bytes.Equal(hostFrames[0], want) {
		t.Fatalf("host received %x, want presence-left for client 2 %x", hostFrames[0], want)
	}
}

func TestRoom_NonNumericClientIDSkipsPresence(t *testing.T) {
	// Existing tests rely on this: a non-numeric client_id cannot be encoded into
	// the fixed u64 presence layout, so the join/leave proceeds without presence
	// rather than failing.
	r := New("room-presence-nonnumeric")
	host := client.New("1", "room-presence-nonnumeric", nil)
	weird := client.New("not-a-number", "room-presence-nonnumeric", nil)
	_ = r.Join(host)
	_ = drainFrames(host)

	if err := r.Join(weird); err != nil {
		t.Fatalf("join weird: %v", err)
	}
	if got := drainFrames(host); len(got) != 0 {
		t.Fatalf("host received %d presence frames for non-numeric id, want 0", len(got))
	}
}

func TestRoom_GcCommitBroadcastsToPeers(t *testing.T) {
	r := New("room-gc")
	runDone := make(chan struct{})
	go func() { r.Run(); close(runDone) }()

	host := client.New("1", "room-gc", nil)
	guest := client.New("2", "room-gc", nil)
	_ = r.Join(host)
	_ = r.Join(guest)
	_ = drainFrames(host)
	_ = drainFrames(guest)

	gc := []byte{wire.PrefixGcCommit, 0xAA, 0xBB}
	r.Ops() <- BroadcastMsg{Sender: host, Data: gc}

	select {
	case f := <-guest.SendChan():
		if !bytes.Equal(f, gc) {
			t.Fatalf("guest received %x, want gc-commit %x", f, gc)
		}
	case <-time.After(time.Second):
		t.Fatal("guest did not receive gc-commit broadcast")
	}

	r.Leave(host, nil)
	r.Leave(guest, nil)
	<-runDone
}
