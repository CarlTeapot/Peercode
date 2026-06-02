# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository layout

Three workspaces that are built and tested independently:

- `crdt-core/` — Rust library implementing the YATA-style CRDT (`Document`, `Snapshot`, wire codec). Shared by the Tauri desktop app and (via wire-format compatibility) by the gateway.
- `gateway/` — Go WebSocket relay. A short-lived sidecar binary launched by the desktop app for the duration of a hosted session.
- `tauri-app/` — desktop client. Frontend is React+TypeScript (Vite, Monaco editor); the native side is Rust (`src-tauri/`) and embeds `crdt-core` directly.

The top-level `Makefile` is the canonical entry point — it `include`s `build/make/{build,test,fmt-lint}.mk`. Do not assume per-workspace `make` works; always run targets from the repo root.

## Common commands

Run from the repo root:

```bash
make dev              # full dev loop: builds gateway sidecar, ensures cloudflared, runs `tauri dev`
make dev-gateway      # gateway only: `go run main.go` (no Tauri)
make build-gateway    # cross-builds gateway into tauri-app/src-tauri/binaries/peercode-gateway-<target-triple>[.exe]
make prod / prod-build / prod-run   # release build of frontend + sidecars + tauri-app binary
make build            # legacy combined build (gateway bin/ + frontend dist + tauri build)

make test-crdt | test-tauri | test-go | test-all
make lint-crdt | lint-tauri | lint-go | lint-frontend | lint-all
make format-crdt | format-tauri | format-go | format-frontend | format-all
make check            # format-all && lint-all  (what pre-push runs)

make install          # `npm install` inside tauri-app
make install-linux-deps          # apt packages required by Tauri/WebKit on Linux
make install-rust-dev-tools      # installs sccache, mold (used by `make dev` for fast incrementals)
make install-cloudflared         # downloads the cloudflared sidecar binary
make install-git-hooks           # installs build/scripts/pre-push as .git/hooks/pre-push (runs `make check`)
make reset-identity              # deletes persisted username so the first-run prompt re-appears
make clean                       # removes node_modules, dist, Cargo target dirs, gateway/bin
```

### Running a single test

- Rust (either crate): `cd crdt-core && cargo test <test_name_substring>` (same for `tauri-app/src-tauri`).
- Go gateway: `cd gateway && go test -v -run <TestName> ./internal/<pkg>` (e.g. `./internal/hub`).
- Frontend has no test suite at present — `npm run lint` / `npm run typecheck` are the only frontend checks.

### Useful environment knobs

- `GATEWAY_AUTH_TOKEN` — **required** to start `gateway` standalone; it refuses to boot without one. The Tauri app sets this automatically per-session via `gateway::auth_token_generator`.
- `GATEWAY_WS_RATE_LIMIT_RPM` — defaults to `5`; set `0` or negative to disable.
- `GATEWAY_LOG_LEVEL` — `debug|info|warn|error|off` for the gateway.
- `PORT=<n> make dev` — overrides the Vite dev port (default `1420`).
- Build acceleration: `make dev` sets `RUSTC_WRAPPER=sccache` and uses `mold` as the linker when present. Override with `RUSTC_WRAPPER=` / `RUSTFLAGS=` to disable.

### Tauri/CI binary stubs

`tauri-app/src-tauri/binaries/` must contain `peercode-gateway-<target-triple>` and `cloudflared-<target-triple>` for `tauri dev`/`tauri build` to succeed (declared as Tauri sidecars). `make dev` produces the gateway one automatically; CI creates empty placeholders so `cargo` builds don't fail when the sidecars aren't actually exercised.

## Architecture

PeerCode is a real-time collaborative editor. There are three runtime pieces:

1. **Desktop app (`tauri-app`)** — every user runs one. Embeds the CRDT, the Monaco editor, and the session lifecycle UI.
2. **Gateway (`gateway`)** — spawned only by the host as a Tauri sidecar process when they start a session. It is a stateless WebSocket relay scoped to a single room.
3. **Cloudflared tunnel** — optional sidecar that exposes the host's gateway via a public `*.trycloudflare.com` URL so peers off-LAN can join.

### Session lifecycle (host)

`session::host_commands::host_session` orchestrates startup (`tauri-app/src-tauri/src/session/host_commands.rs`):

1. `AppRole` transitions `Undecided → Starting`. Any failure path rolls it back.
2. `process_coordinator::launch` spawns gateway, then (best-effort) cloudflared. Gateway prints its listening port as JSON (`{"port":N}`) on stdout — the coordinator parses this to learn where to talk to it.
3. `gateway_api::create_room` POSTs to `http://127.0.0.1:<port>/rooms` (bearer-authed) to mint a room id.
4. `AppRole` transitions to `Host { room_id, local_room_url, public_room_url, … }` and a `session://session-ready` event is emitted to the frontend.
5. Host opens its own local WS to the gateway (`ws://127.0.0.1:<port>/ws?room=…&client_id=…`) and immediately sends an encoded snapshot so late-joining peers can be seeded.

Guest is the same minus the gateway/tunnel spawn; it just opens the WS to the public/LAN URL. Both host and guest run through `state::ws_state::WsState::connect`, which spawns three Tokio tasks per connection: `write_loop`, `receive_loop`, and `process_loop` (dispatches decoded frames to the doc actor).

### Document actor

The CRDT `Document` is **not** behind a mutex — it lives inside a single Tokio task (`state::document::actor::DocActor`) that owns it and consumes a `mpsc::Receiver<DocOp>`. All access (local edits, remote ops, snapshots, persistence reads, debug introspection) goes through `DocOp` messages and oneshot replies. When extending document behavior:

- Add a new `DocOp` variant in `state/document/types.rs`.
- Handle it in `DocActor::dispatch` (`state/document/actor.rs`), routing to one of `handlers::{local, remote, snapshot}`.
- Call it from a `#[tauri::command]` (or other caller) via `client::request(&doc_tx, |reply| DocOp::… { reply })`.

The actor emits events to the webview when state changes: `crdt://remote-change`, `crdt://snapshot-applied`, `crdt://document-reset` (constants in `types.rs`). The frontend's `remoteChangeListener.ts` / `snapshotListener.ts` apply these to Monaco without re-broadcasting (the `isApplyingRemote` ref gates the `onDidChangeModelContent` handler in `App.tsx`).

### Position index (`crdt-core/src/index/`)

The CRDT linked list of blocks (traversed via `block.right()`) is the **source of truth for order**, but walking it to convert between a `BlockId` and its visible-text character position is O(n). `Document` therefore carries a `PositionIndex` — an augmented B+ tree that aggregates each subtree's `visible_len`, giving O(log n) `position_of(id)` and `find_at_position(pos)`. It is a *secondary, derived* structure: it must be mutated in lockstep with the linked list, never independently.

- Every `Document` mutation that changes block order, length, or visibility mirrors itself into the index (`insert_after`, `split_entry`, `set_deleted`, `rebuild_from_order`) — see the call sites in `document/integrate.rs` and `Document::restore` in `document.rs`.
- **Debug oracle:** in debug builds `assert_index_matches_linked_list` (`document/debug.rs`) walks the list after each mutation and panics if the tree disagrees; `index/validate.rs` separately checks the tree's internal invariants. These are `#[cfg(debug_assertions)]` only.
- Module shape (SOLID split): `index/structs/` holds data + constructors only (`Storage`, `Leaf`, `Node`, `PositionIndex`, …); the sibling files hold operations (`mutate`, `find`, `split`, `propagate`, `descend`, `build`, `storage_ops`). Branching factors live in `index/constants.rs` and **differ between debug (tiny, to force splits in tests) and release (wide)** — a behavioral knob, not a constant.
- Full walkthrough with diagrams: `docs/b-tree-optimisation.md`.

### Wire protocol — two parallel surfaces

There is a binary wire framing **and** a JSON protocol envelope. They are not the same layer:

- **Binary framing** (the actual bytes over WS): a single prefix byte then a bitcode payload. Defined in both `crdt-core/src/wire.rs` (Rust) and `gateway/internal/wire/wire.go` (Go) — they must stay in sync. `protocol_drift_tests` in each language exists specifically to catch this drift; **update both sides when changing wire formats**. Prefixes:
  - `0x00` = op (bitcode-encoded `OpMessage::{Insert(WireBlock), Delete(DeleteSet)}`)
  - `0x01` = snapshot (bitcode-encoded `Snapshot`)
  - `0x02` = control frame (currently only `0x01` = session-ended)
- **JSON envelope** (`gateway/internal/protocol/protocol.go`) documents a higher-level `{type, room, client_id, seq, payload}` message intended for sync/peer-list semantics. Treat this file as the protocol spec; the gateway today operates on raw binary frames (snapshot vs op detection) and does not currently parse the JSON envelope.

The gateway's room (`gateway/internal/room/room.go`) keeps `latestSnapshot` and an `opsLog` accumulated since that snapshot. Frames arriving with the snapshot prefix replace `latestSnapshot` and clear `opsLog`; everything else is appended and broadcast. New joiners get a `ReplayTo` of `snapshot + buffered ops` before live frames flow.

### Auth

The gateway requires `Authorization: Bearer <GATEWAY_AUTH_TOKEN>` on all routes except `/health` and `/ws` (see `gateway/cmd/server/auth_filter.go`). The host's Tauri app generates the token (`gateway::auth_token_generator`), passes it to the spawned gateway via env, and uses it for its own `/rooms` / `/end-session` calls. The token is **not** shared with guests — they only need the WS URL (which is unauthenticated, since they need to be able to join cross-network).

### State model (Tauri side)

`AppState` (`state/appstate.rs`) wraps `Mutex<AppRole>` (the session FSM: `Undecided | Starting | Host{…} | Guest{…}`), `Mutex<HostProcesses>` (gateway+tunnel sidecar handles and the auth token), the doc actor's `DocSender`, and counters used for snapshot cadence. `WsState` holds the live websocket and its task handles. Both are `tauri::Manager`-managed singletons set up in `lib.rs`.

All session lifecycle transitions go through `AppState` — see `.claude/rules/session-state-machine.md` for the FSM rules and guard pattern.

When the host window is destroyed (see `on_window_event` in `lib.rs`), `destroy_room` and `kill_host_processes` run synchronously to avoid leaving orphaned sidecars.

## Conventions worth knowing

- **`pre-push` hook runs `make check`** (format-all + lint-all). Install with `make install-git-hooks`. CI runs the same checks plus tests and security audits (`cargo audit`, `npm audit`, `govulncheck`).
- **Clippy is `-D warnings`** in `crdt-core` and (in CI) `tauri-app/src-tauri`. Treat clippy lints as build failures.
- **Edition 2024 / Rust stable** for `crdt-core`; **edition 2021** for `tauri-app/src-tauri`. Go module pins **`go 1.25.10`**.
- The `peercode.config.toml` next to `Cargo.toml` is embedded into the binary via `include_str!` at compile time — config changes require a rebuild, not just a restart.
- When you change `WireBlock`, `OpMessage`, `DeleteSet`, or `Snapshot` encoding in `crdt-core`, also update the matching constants/tests in `gateway/internal/wire/` and re-run the `protocol_drift` tests on both sides.
- When you add a `Document` mutation that changes block order, length, or visibility, mirror it into `position_index` in the same step and let the debug oracle (`assert_index_matches_linked_list`) catch any drift — never let the index and linked list diverge.
