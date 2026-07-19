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
	gatewaymetrics "gateway/internal/metrics"
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
	r := New("room-1", gatewaymetrics.New())
	runDone := make(chan struct{})
	go func() { r.Run(); close(runDone) }()

	a := client.New("a", "room-1", "userA", nil)
	b := client.New("b", "room-1", "userB", nil)
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
	r := New("room-2", gatewaymetrics.New())
	runDone := make(chan struct{})
	go func() { r.Run(); close(runDone) }()

	a := client.New("a", "room-2", "userA", nil)
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
	r := New("room-3", gatewaymetrics.New())
	runDone := make(chan struct{})
	go func() { r.Run(); close(runDone) }()

	a := client.New("a", "room-3", "userA", nil)
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
	r := New("room-4", gatewaymetrics.New())
	runDone := make(chan struct{})
	go func() { r.Run(); close(runDone) }()

	a := client.New("a", "room-4", "userA", nil)
	if err := r.Join(a); err != nil {
		t.Fatalf("join: %v", err)
	}
	r.Leave(a, nil)

	b := client.New("b", "room-4", "userB", nil)
	if err := r.Join(b); !errors.Is(err, ErrRoomClosed) {
		t.Fatalf("Join on closed room: got %v, want ErrRoomClosed", err)
	}

	<-runDone
}

func TestRoom_SnapshotReplayToJoiner(t *testing.T) {
	r := New("room-snap", gatewaymetrics.New())
	runDone := make(chan struct{})
	go func() { r.Run(); close(runDone) }()

	host := client.New("host", "room-snap", "hostUser", nil)
	if err := r.Join(host); err != nil {
		t.Fatalf("join host: %v", err)
	}

	joiner := client.New("joiner", "room-snap", "joinerUser", nil)
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
	r := New("room-dup", gatewaymetrics.New())
	runDone := make(chan struct{})
	go func() { r.Run(); close(runDone) }()

	a := client.New("same", "room-dup", "dupUser", nil)
	b := client.New("same", "room-dup", "dupUser", nil)
	if err := r.Join(a); err != nil {
		t.Fatalf("first join: %v", err)
	}
	if err := r.Join(b); !errors.Is(err, ErrDuplicateClientID) {
		t.Fatalf("second join: got %v, want ErrDuplicateClientID", err)
	}

	r.Leave(a, nil)
	<-runDone
}

func containsFrame(frames [][]byte, want []byte) bool {
	for _, f := range frames {
		if bytes.Equal(f, want) {
			return true
		}
	}
	return false
}

func mustEncodePeerInfo(t *testing.T, clientID uint64, isHost, canWrite bool, username string) []byte {
	t.Helper()
	frame, err := wire.EncodePeerInfoFrame(clientID, isHost, canWrite, username)
	if err != nil {
		t.Fatalf("EncodePeerInfoFrame: %v", err)
	}
	return frame
}

func TestRoom_JoinBroadcastsMembershipToExistingMembers(t *testing.T) {
	r := New("room-membership-join", gatewaymetrics.New())
	host := client.New("1", "room-membership-join", "hostname", nil)
	guest := client.New("2", "room-membership-join", "guestname", nil)

	if err := r.Join(host); err != nil {
		t.Fatalf("join host: %v", err)
	}
	wantHostInfo := mustEncodePeerInfo(t, 1, true, true, "hostname")
	hostRoster := drainFrames(host)
	if len(hostRoster) != 1 || !bytes.Equal(hostRoster[0], wantHostInfo) {
		t.Fatalf("first joiner roster = %x, want [%x]", hostRoster, wantHostInfo)
	}

	if err := r.Join(guest); err != nil {
		t.Fatalf("join guest: %v", err)
	}

	hostFrames := drainFrames(host)
	if len(hostFrames) != 2 {
		t.Fatalf("host received %d frames, want 2 (membership + peer-info for guest)", len(hostFrames))
	}
	wantJoined := wire.EncodeMembershipFrame(2, wire.MembershipJoined)
	if !bytes.Equal(hostFrames[0], wantJoined) {
		t.Fatalf("host received %x, want membership-joined for client 2 %x", hostFrames[0], wantJoined)
	}
	wantGuestInfo := mustEncodePeerInfo(t, 2, false, false, "guestname")
	if !bytes.Equal(hostFrames[1], wantGuestInfo) {
		t.Fatalf("host received %x, want peer-info for guest %x", hostFrames[1], wantGuestInfo)
	}

	guestRoster := drainFrames(guest)
	if len(guestRoster) != 2 {
		t.Fatalf("joiner received %d frames, want 2 (roster: host + self)", len(guestRoster))
	}
	if !containsFrame(guestRoster, wantHostInfo) || !containsFrame(guestRoster, wantGuestInfo) {
		t.Fatalf("joiner roster %x missing host or self peer-info", guestRoster)
	}
}

func TestRoom_LeaveBroadcastsMembershipToRemaining(t *testing.T) {
	r := New("room-membership-leave", gatewaymetrics.New())
	host := client.New("1", "room-membership-leave", "hostname", nil)
	guest := client.New("2", "room-membership-leave", "guestname", nil)
	_ = r.Join(host)
	_ = r.Join(guest)
	_ = drainFrames(host) // discard joined(guest)

	r.Leave(guest, nil)

	hostFrames := drainFrames(host)
	if len(hostFrames) != 1 {
		t.Fatalf("host received %d frames after guest left, want 1 (left guest)", len(hostFrames))
	}
	want := wire.EncodeMembershipFrame(2, wire.MembershipLeft)
	if !bytes.Equal(hostFrames[0], want) {
		t.Fatalf("host received %x, want membership-left for client 2 %x", hostFrames[0], want)
	}
}

func TestRoom_NonNumericClientIDSkipsMembership(t *testing.T) {
	// Existing tests rely on this: a non-numeric client_id cannot be encoded into
	// the fixed u64 membership layout, so the join/leave proceeds without membership
	// rather than failing.
	r := New("room-membership-nonnumeric", gatewaymetrics.New())
	host := client.New("1", "room-membership-nonnumeric", "hostname", nil)
	weird := client.New("not-a-number", "room-membership-nonnumeric", "weird", nil)
	_ = r.Join(host)
	_ = drainFrames(host)

	if err := r.Join(weird); err != nil {
		t.Fatalf("join weird: %v", err)
	}
	if got := drainFrames(host); len(got) != 0 {
		t.Fatalf("host received %d membership frames for non-numeric id, want 0", len(got))
	}
}

func TestRoom_GcCommitBroadcastsToPeers(t *testing.T) {
	r := New("room-gc", gatewaymetrics.New())
	runDone := make(chan struct{})
	go func() { r.Run(); close(runDone) }()

	host := client.New("1", "room-gc", "hostname", nil)
	guest := client.New("2", "room-gc", "guestname", nil)
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

func TestRoom_FirstJoinerBecomesHostGuestsReadOnlyByDefault(t *testing.T) {
	r := New("room-roles", gatewaymetrics.New())
	host := client.New("1", "room-roles", "hostname", nil)
	guest := client.New("2", "room-roles", "guestname", nil)
	if err := r.Join(host); err != nil {
		t.Fatalf("join host: %v", err)
	}
	if err := r.Join(guest); err != nil {
		t.Fatalf("join guest: %v", err)
	}

	if host.Role() != client.RoleHost {
		t.Fatal("first joiner was not assigned RoleHost")
	}
	if guest.Role() != client.RoleGuest {
		t.Fatal("second joiner was not assigned RoleGuest")
	}
	if !host.CanWrite() {
		t.Fatal("host must always have write access")
	}
	if guest.CanWrite() {
		t.Fatal("guest should be read-only when the host did not opt into default_can_write")
	}
}

func TestRoom_HostDefaultCanWriteGrantsGuests(t *testing.T) {
	r := New("room-default-write", gatewaymetrics.New())
	host := client.New("1", "room-default-write", "hostname", nil)
	host.HostDefaultCanWrite = true
	guest := client.New("2", "room-default-write", "guestname", nil)
	_ = r.Join(host)
	_ = r.Join(guest)

	if !guest.CanWrite() {
		t.Fatal("guest should inherit write access from the host's default_can_write")
	}
}

func TestRoom_OpFromReadOnlyGuestIsDropped(t *testing.T) {
	r := New("room-readonly-op", gatewaymetrics.New())
	runDone := make(chan struct{})
	go func() { r.Run(); close(runDone) }()

	host := client.New("1", "room-readonly-op", "hostname", nil)
	guest := client.New("2", "room-readonly-op", "guestname", nil)
	_ = r.Join(host)
	_ = r.Join(guest)
	_ = drainFrames(host)
	_ = drainFrames(guest)

	op := []byte{wire.PrefixOp, 0xAA}
	r.Ops() <- BroadcastMsg{Sender: guest, Data: op}

	select {
	case f := <-host.SendChan():
		t.Fatalf("host received op %x from read-only guest, want drop", f)
	case <-time.After(100 * time.Millisecond):
	}

	r.Leave(host, nil)
	r.Leave(guest, nil)
	<-runDone
}

func TestRoom_PermissionChangeGrantsWriteAndBroadcasts(t *testing.T) {
	r := New("room-perm-grant", gatewaymetrics.New())
	runDone := make(chan struct{})
	go func() { r.Run(); close(runDone) }()

	host := client.New("1", "room-perm-grant", "hostname", nil)
	guest := client.New("2", "room-perm-grant", "guestname", nil)
	_ = r.Join(host)
	_ = r.Join(guest)
	_ = drainFrames(host)
	_ = drainFrames(guest)

	grant := wire.EncodePermissionFrame(2, true)
	r.Ops() <- BroadcastMsg{Sender: host, Data: grant}

	for _, c := range []*client.Client{host, guest} {
		select {
		case f := <-c.SendChan():
			if !bytes.Equal(f, grant) {
				t.Fatalf("client %s received %x, want permission frame %x", c.ID, f, grant)
			}
		case <-time.After(time.Second):
			t.Fatalf("client %s did not receive the permission broadcast", c.ID)
		}
	}
	if !guest.CanWrite() {
		t.Fatal("guest write permission was not applied")
	}

	op := []byte{wire.PrefixOp, 0xBB}
	r.Ops() <- BroadcastMsg{Sender: guest, Data: op}
	select {
	case f := <-host.SendChan():
		if !bytes.Equal(f, op) {
			t.Fatalf("host received %x, want op %x", f, op)
		}
	case <-time.After(time.Second):
		t.Fatal("host did not receive op from granted guest")
	}

	r.Leave(host, nil)
	r.Leave(guest, nil)
	<-runDone
}

func TestRoom_PermissionChangeFromGuestIsDropped(t *testing.T) {
	r := New("room-perm-guest", gatewaymetrics.New())
	runDone := make(chan struct{})
	go func() { r.Run(); close(runDone) }()

	host := client.New("1", "room-perm-guest", "hostname", nil)
	guest := client.New("2", "room-perm-guest", "guestname", nil)
	_ = r.Join(host)
	_ = r.Join(guest)
	_ = drainFrames(host)
	_ = drainFrames(guest)

	r.Ops() <- BroadcastMsg{Sender: guest, Data: wire.EncodePermissionFrame(2, true)}

	select {
	case f := <-host.SendChan():
		t.Fatalf("host received %x after guest-authored permission change, want drop", f)
	case <-time.After(100 * time.Millisecond):
	}
	if guest.CanWrite() {
		t.Fatal("guest granted itself write access")
	}

	r.Leave(host, nil)
	r.Leave(guest, nil)
	<-runDone
}

func TestRoom_PermissionChangeForHostIsDropped(t *testing.T) {
	r := New("room-perm-host", gatewaymetrics.New())
	runDone := make(chan struct{})
	go func() { r.Run(); close(runDone) }()

	host := client.New("1", "room-perm-host", "hostname", nil)
	guest := client.New("2", "room-perm-host", "guestname", nil)
	_ = r.Join(host)
	_ = r.Join(guest)
	_ = drainFrames(host)
	_ = drainFrames(guest)

	r.Ops() <- BroadcastMsg{Sender: host, Data: wire.EncodePermissionFrame(1, false)}

	select {
	case f := <-guest.SendChan():
		t.Fatalf("guest received %x after host-targeted permission change, want drop", f)
	case <-time.After(100 * time.Millisecond):
	}
	if !host.CanWrite() {
		t.Fatal("host lost write access; host permission must be immutable")
	}

	r.Leave(host, nil)
	r.Leave(guest, nil)
	<-runDone
}

func TestRoom_GcCommitFromGuestIsDropped(t *testing.T) {
	r := New("room-gc-guest", gatewaymetrics.New())
	runDone := make(chan struct{})
	go func() { r.Run(); close(runDone) }()

	host := client.New("1", "room-gc-guest", "hostname", nil)
	guest := client.New("2", "room-gc-guest", "guestname", nil)
	_ = r.Join(host)
	_ = r.Join(guest)
	_ = drainFrames(host)
	_ = drainFrames(guest)

	gc := []byte{wire.PrefixGcCommit, 0xAA, 0xBB}
	r.Ops() <- BroadcastMsg{Sender: guest, Data: gc}

	select {
	case f := <-host.SendChan():
		t.Fatalf("host received guest-authored gc-commit %x, want drop", f)
	case <-time.After(100 * time.Millisecond):
	}

	r.Leave(host, nil)
	r.Leave(guest, nil)
	<-runDone
}
