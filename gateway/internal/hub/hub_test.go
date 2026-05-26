package hub

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"log/slog"
	"net/http"
	"net/http/httptest"
	"os"
	"regexp"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/coder/websocket"

	"gateway/internal/wire"
)

func init() {
	slog.SetDefault(slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: slog.LevelError})))
}

func newTestServer(t *testing.T) (*httptest.Server, *Hub) {
	t.Helper()
	h := New()
	mux := http.NewServeMux()
	mux.HandleFunc("/ws", h.HandleWS)
	mux.HandleFunc("/rooms", h.HandleCreateRoom)
	srv := httptest.NewServer(mux)
	t.Cleanup(srv.Close)
	return srv, h
}

func wsURL(base, room, clientID string) string {
	return strings.Replace(base, "http://", "ws://", 1) +
		"/ws?room=" + room + "&client_id=" + clientID
}

func dial(t *testing.T, url string) *websocket.Conn {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	c, _, err := websocket.Dial(ctx, url, nil)
	if err != nil {
		t.Fatalf("dial %s: %v", url, err)
	}
	return c
}

func answerSnapshotRequest(t *testing.T, host *websocket.Conn) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	_, request, err := host.Read(ctx)
	if err != nil {
		t.Fatalf("host snapshot request read: %v", err)
	}
	if len(request) != 2 || request[0] != wire.PrefixControl || request[1] != wire.ControlSnapshotRequest {
		t.Fatalf("snapshot request = %v, want control snapshot request", request)
	}
	if err := host.Write(context.Background(), websocket.MessageBinary, []byte{wire.PrefixSnapshot, 0xAA}); err != nil {
		t.Fatalf("host snapshot response write: %v", err)
	}
}

func readSnapshot(t *testing.T, peer *websocket.Conn) {
	t.Helper()
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	_, snap, err := peer.Read(ctx)
	if err != nil {
		t.Fatalf("peer snapshot read: %v", err)
	}
	if len(snap) == 0 || snap[0] != wire.PrefixSnapshot {
		t.Fatalf("snapshot frame prefix = %v, want snapshot", snap)
	}
}

func TestHub_PostRoomsReturnsFreshID(t *testing.T) {
	srv, _ := newTestServer(t)

	var ids []string
	for i := 0; i < 2; i++ {
		resp, err := http.Post(srv.URL+"/rooms", "application/json", nil)
		if err != nil {
			t.Fatalf("POST /rooms: %v", err)
		}
		if resp.StatusCode != http.StatusOK {
			resp.Body.Close()
			t.Fatalf("POST /rooms: status=%d, want 200", resp.StatusCode)
		}
		var body map[string]string
		if err := json.NewDecoder(resp.Body).Decode(&body); err != nil {
			t.Fatalf("decode: %v", err)
		}
		resp.Body.Close()
		id := body["room_id"]
		if !regexp.MustCompile(`^[0-9a-f]{8}$`).MatchString(id) {
			t.Fatalf("room_id=%q, want 8 lowercase hex chars", id)
		}
		ids = append(ids, id)
	}
	if ids[0] == ids[1] {
		t.Fatalf("IDs collided: %s", ids[0])
	}
}

func TestHub_PostRoomsRejectsGet(t *testing.T) {
	srv, _ := newTestServer(t)
	resp, err := http.Get(srv.URL + "/rooms")
	if err != nil {
		t.Fatalf("GET /rooms: %v", err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != http.StatusMethodNotAllowed {
		t.Fatalf("status=%d, want 405", resp.StatusCode)
	}
}

func TestHub_RejectsMissingQuery(t *testing.T) {
	srv, _ := newTestServer(t)
	for _, path := range []string{"/ws", "/ws?room=r", "/ws?client_id=c"} {
		resp, err := http.Get(srv.URL + path)
		if err != nil {
			t.Fatalf("GET %s: %v", path, err)
		}
		resp.Body.Close()
		if resp.StatusCode != http.StatusBadRequest {
			t.Fatalf("%s: status=%d, want 400", path, resp.StatusCode)
		}
	}
}

func TestHub_TwoClientsSameRoomShareSet(t *testing.T) {
	srv, h := newTestServer(t)
	a := dial(t, wsURL(srv.URL, "shared", "alice"))
	defer a.Close(websocket.StatusNormalClosure, "")
	b := dial(t, wsURL(srv.URL, "shared", "bob"))
	defer b.Close(websocket.StatusNormalClosure, "")
	answerSnapshotRequest(t, a)
	readSnapshot(t, b)

	deadline := time.Now().Add(time.Second)
	for time.Now().Before(deadline) {
		if len(h.Rooms()) == 1 {
			break
		}
		time.Sleep(5 * time.Millisecond)
	}
	if rooms := h.Rooms(); len(rooms) != 1 || rooms[0] != "shared" {
		t.Fatalf("Rooms=%v, want [shared]", rooms)
	}
}

func TestHub_DuplicateClientIDRejected(t *testing.T) {
	srv, _ := newTestServer(t)
	first := dial(t, wsURL(srv.URL, "dup-room", "same-id"))
	defer first.Close(websocket.StatusNormalClosure, "")

	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	second, _, err := websocket.Dial(ctx, wsURL(srv.URL, "dup-room", "same-id"), nil)
	if err != nil {
		t.Fatalf("second dial: %v", err)
	}
	defer second.Close(websocket.StatusNormalClosure, "")

	readCtx, readCancel := context.WithTimeout(context.Background(), time.Second)
	defer readCancel()
	_, _, err = second.Read(readCtx)
	if err == nil {
		t.Fatal("expected read on duplicate client_id connection to fail after server closes it")
	}
}

func TestHub_DisconnectRemovesClient(t *testing.T) {
	srv, h := newTestServer(t)
	a := dial(t, wsURL(srv.URL, "ephemeral", "alice"))

	registerDeadline := time.Now().Add(time.Second)
	for time.Now().Before(registerDeadline) && len(h.Rooms()) == 0 {
		time.Sleep(5 * time.Millisecond)
	}
	if len(h.Rooms()) != 1 {
		t.Fatalf("room not registered")
	}

	_ = a.Close(websocket.StatusNormalClosure, "bye")

	disconnectDeadline := time.Now().Add(time.Second)
	for time.Now().Before(disconnectDeadline) && len(h.Rooms()) != 0 {
		time.Sleep(5 * time.Millisecond)
	}
	if got := h.Rooms(); len(got) != 0 {
		t.Fatalf("Rooms=%v, want empty after disconnect", got)
	}
}

func TestHub_RoomsIsolated(t *testing.T) {
	srv, _ := newTestServer(t)
	a := dial(t, wsURL(srv.URL, "x", "a"))
	defer a.Close(websocket.StatusNormalClosure, "")
	c := dial(t, wsURL(srv.URL, "y", "c"))
	defer c.Close(websocket.StatusNormalClosure, "")

	ctx, cancel := context.WithTimeout(context.Background(), 500*time.Millisecond)
	defer cancel()
	if err := a.Write(ctx, websocket.MessageBinary, wire.EncodeOpFrame([]byte("hello"))); err != nil {
		t.Fatalf("write: %v", err)
	}

	readCtx, readCancel := context.WithTimeout(context.Background(), 200*time.Millisecond)
	defer readCancel()
	_, _, err := c.Read(readCtx)
	if err == nil {
		t.Fatal("client in room y received a frame from room x — rooms are not isolated")
	}
}

func TestHub_ConcurrentJoinsSameRoom(t *testing.T) {
	srv, h := newTestServer(t)
	const n = 10
	host := dial(t, wsURL(srv.URL, "race", "c0"))
	defer host.Close(websocket.StatusNormalClosure, "")

	var wg sync.WaitGroup
	wg.Add(n - 1)
	conns := make(chan *websocket.Conn, n-1)
	for i := 1; i < n; i++ {
		go func(i int) {
			defer wg.Done()
			cid := fmt.Sprintf("c%d", i)
			conns <- dial(t, wsURL(srv.URL, "race", cid))
		}(i)
	}
	wg.Wait()
	close(conns)
	joined := make([]*websocket.Conn, 0, n-1)
	for c := range conns {
		defer c.Close(websocket.StatusNormalClosure, "")
		joined = append(joined, c)
	}
	for range joined {
		answerSnapshotRequest(t, host)
	}
	for _, c := range joined {
		readSnapshot(t, c)
	}

	deadline := time.Now().Add(time.Second)
	for time.Now().Before(deadline) {
		if len(h.Rooms()) == 1 {
			break
		}
		time.Sleep(5 * time.Millisecond)
	}
	if rooms := h.Rooms(); len(rooms) != 1 || rooms[0] != "race" {
		t.Fatalf("Rooms=%v, want [race]", rooms)
	}
}

func TestHub_FanOut_SenderExcluded(t *testing.T) {
	srv, _ := newTestServer(t)

	a := dial(t, wsURL(srv.URL, "fanout", "mate"))
	defer a.Close(websocket.StatusNormalClosure, "")
	b := dial(t, wsURL(srv.URL, "fanout", "gendi"))
	defer b.Close(websocket.StatusNormalClosure, "")
	answerSnapshotRequest(t, a)
	readSnapshot(t, b)

	time.Sleep(50 * time.Millisecond)

	ctx := context.Background()
	payload := wire.EncodeOpFrame([]byte("hello"))
	if err := a.Write(ctx, websocket.MessageBinary, payload); err != nil {
		t.Fatalf("write: %v", err)
	}

	readCtx, readCancel := context.WithTimeout(ctx, 2*time.Second)
	defer readCancel()
	_, data, err := b.Read(readCtx)
	if err != nil {
		t.Fatalf("gendi read: %v", err)
	}
	if !bytes.Equal(data, payload) {
		t.Fatalf("gendi got %x, want %x", data, payload)
	}

	noEchoCtx, noEchoCancel := context.WithTimeout(ctx, 200*time.Millisecond)
	defer noEchoCancel()
	_, _, err = a.Read(noEchoCtx)
	if err == nil {
		t.Fatal("mate received echo")
	}
}

func TestHub_FanOut_ThreeClients(t *testing.T) {
	srv, _ := newTestServer(t)

	a := dial(t, wsURL(srv.URL, "trio", "mate"))
	defer a.Close(websocket.StatusNormalClosure, "")
	b := dial(t, wsURL(srv.URL, "trio", "gendi"))
	defer b.Close(websocket.StatusNormalClosure, "")
	answerSnapshotRequest(t, a)
	readSnapshot(t, b)
	c := dial(t, wsURL(srv.URL, "trio", "gela"))
	defer c.Close(websocket.StatusNormalClosure, "")
	answerSnapshotRequest(t, a)
	readSnapshot(t, c)

	time.Sleep(50 * time.Millisecond)

	ctx := context.Background()
	payload := wire.EncodeOpFrame([]byte("broadcast"))
	if err := a.Write(ctx, websocket.MessageBinary, payload); err != nil {
		t.Fatalf("write: %v", err)
	}

	for _, pair := range []struct {
		name string
		conn *websocket.Conn
	}{{"gendi", b}, {"gela", c}} {
		readCtx, cancel := context.WithTimeout(ctx, 2*time.Second)
		_, data, err := pair.conn.Read(readCtx)
		cancel()
		if err != nil {
			t.Fatalf("%s read: %v", pair.name, err)
		}
		if !bytes.Equal(data, payload) {
			t.Fatalf("%s got %x, want %x", pair.name, data, payload)
		}
	}

	noEchoCtx, noEchoCancel := context.WithTimeout(ctx, 200*time.Millisecond)
	defer noEchoCancel()
	_, _, err := a.Read(noEchoCtx)
	if err == nil {
		t.Fatal("mate received echo")
	}
}

func BenchmarkHub_FanOut_1000Ops(b *testing.B) {
	h := New()
	mux := http.NewServeMux()
	mux.HandleFunc("/ws", h.HandleWS)
	srv := httptest.NewServer(mux)
	defer srv.Close()

	ctx := context.Background()

	sender, _, err := websocket.Dial(ctx, wsURL(srv.URL, "bench", "sender"), nil)
	if err != nil {
		b.Fatalf("dial sender: %v", err)
	}
	defer sender.Close(websocket.StatusNormalClosure, "")

	receiver, _, err := websocket.Dial(ctx, wsURL(srv.URL, "bench", "receiver"), nil)
	if err != nil {
		b.Fatalf("dial receiver: %v", err)
	}
	defer receiver.Close(websocket.StatusNormalClosure, "")

	time.Sleep(50 * time.Millisecond)

	payload := wire.EncodeOpFrame([]byte("op"))

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		for j := 0; j < 1000; j++ {
			if err := sender.Write(ctx, websocket.MessageBinary, payload); err != nil {
				b.Fatalf("write: %v", err)
			}
			readCtx, cancel := context.WithTimeout(ctx, 5*time.Second)
			_, _, err := receiver.Read(readCtx)
			cancel()
			if err != nil {
				b.Fatalf("read: %v", err)
			}
		}
	}
}
