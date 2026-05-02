package room

import (
	"log/slog"
	"os"
	"sync"
	"testing"
	"time"

	"gateway/internal/client"
)

func init() {
	slog.SetDefault(slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: slog.LevelError})))
}

func TestRoom_JoinLeaveTriggersOnEmpty(t *testing.T) {
	r := New("room-1")
	runDone := make(chan struct{})
	go func() { r.Run(); close(runDone) }()

	a := client.New("a", "room-1", nil)
	b := client.New("b", "room-1", nil)
	if !r.Join(a) || !r.Join(b) {
		t.Fatal("join rejected by fresh room")
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
	r.Join(a)

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
	r.Join(a)

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
	r.Join(a)
	r.Leave(a, nil)

	b := client.New("b", "room-4", nil)
	if r.Join(b) {
		t.Fatal("Join succeeded on closed room")
	}

	<-runDone
}
