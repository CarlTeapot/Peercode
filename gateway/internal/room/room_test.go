package room

import (
	"errors"
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

	snapFrame := []byte{0x01, 0xAA, 0xBB}
	r.Ops() <- BroadcastMsg{Sender: host, Data: snapFrame}
	time.Sleep(50 * time.Millisecond)

	op1 := []byte{0x00, 0x01}
	op2 := []byte{0x00, 0x02}
	r.Ops() <- BroadcastMsg{Sender: host, Data: op1}
	r.Ops() <- BroadcastMsg{Sender: host, Data: op2}
	time.Sleep(50 * time.Millisecond)

	joiner := client.New("joiner", "room-snap", nil)
	if err := r.Join(joiner); err != nil {
		t.Fatalf("join joiner: %v", err)
	}

	got := r.ReplayTo(joiner)
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
		t.Fatalf("joiner received %d messages, want 3 (snapshot + 2 ops)", len(received))
	}
	if received[0][0] != 0x01 {
		t.Fatalf("first message prefix = 0x%02X, want 0x01 (snapshot)", received[0][0])
	}
	if received[1][0] != 0x00 || received[2][0] != 0x00 {
		t.Fatalf("op messages have wrong prefix")
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
