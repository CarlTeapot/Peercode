.PHONY: format-crdt format-tauri format-go format-frontend format-all lint-frontend lint-crdt lint-tauri lint-go lint-all check install-hooks

# ------------- formatting --------------
format-crdt:
	cd crdt-core && cargo fmt

format-tauri:
	cd tauri-app/src-tauri && cargo fmt

format-go:
	cd gateway && go fmt ./...

format-frontend:
	cd tauri-app && npx prettier --write "src/**/*.{ts,tsx,css}"

format-all: format-crdt format-tauri format-go format-frontend

# ------------- linting ---------------
lint-frontend:
	cd tauri-app && npm run lint

lint-crdt:
	cd crdt-core && cargo clippy --all-targets --all-features -- -D warnings

lint-tauri:
	cd tauri-app/src-tauri && cargo clippy --all-targets --all-features

lint-go:
	cd gateway && go vet ./...

lint-all: lint-frontend lint-crdt lint-tauri lint-go

check: format-all lint-all

# ------------- git hooks -------------
install-hooks:
	cp scripts/hooks/pre-push .git/hooks/pre-push
	chmod +x .git/hooks/pre-push
	@echo "Git hook installed: .git/hooks/pre-push"
