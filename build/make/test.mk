.PHONY: test-crdt test-tauri test-go test-all

# ------------- testing ---------------
test-crdt:
	cd crdt-core && cargo test

test-tauri:
	cd tauri-app/src-tauri && cargo test

test-frontend:
	cd tauri-app && npm test

test-go:
	cd gateway && go test -v ./...

test-all: test-crdt test-tauri test-frontend test-go 
