.PHONY: dev prod prod-build prod-run dev-gateway build-gateway build install-cloudflared install-rust-dev-tools

FRONTEND_BUILD_OUT := tauri-app/dist/index.html
FRONTEND_BUILD_INPUTS := $(shell git ls-files tauri-app/src) tauri-app/index.html tauri-app/vite.config.ts tauri-app/package.json tauri-app/package-lock.json tauri-app/.env.production

RUST_RELEASE_BIN := tauri-app/src-tauri/target/release/tauri-app
RUST_RELEASE_INPUTS := $(shell git ls-files tauri-app/src-tauri/src crdt-core/src) tauri-app/src-tauri/Cargo.toml tauri-app/src-tauri/Cargo.lock crdt-core/Cargo.toml

TARGET_TRIPLE := $(shell rustc -vV | sed -n 's|host: ||p')
GATEWAY_EXT := $(if $(findstring windows,$(TARGET_TRIPLE)),.exe,)
GATEWAY_BIN := tauri-app/src-tauri/binaries/peercode-gateway-$(TARGET_TRIPLE)$(GATEWAY_EXT)
GATEWAY_INPUTS := $(shell git ls-files gateway/)

CLOUDFLARED_BIN := tauri-app/src-tauri/binaries/cloudflared-$(TARGET_TRIPLE)

# Downloads cloudflared only when the binary is absent (Make file-target semantics)
$(CLOUDFLARED_BIN):
	bash build/scripts/install-cloudflared.sh

install-cloudflared: $(CLOUDFLARED_BIN)

install-rust-dev-tools:
	bash build/scripts/install-rust-dev-tools.sh

$(GATEWAY_BIN): $(GATEWAY_INPUTS)
	bash build/scripts/build-gateway.sh "$(TARGET_TRIPLE)" "$(GATEWAY_BIN)"

build-gateway: $(GATEWAY_BIN)

PORT ?= 1420
RUSTC_WRAPPER ?= sccache
MOLD_EXISTS := $(shell command -v mold >/dev/null 2>&1 && echo yes || echo no)
RUSTFLAGS ?= $(if $(filter yes,$(MOLD_EXISTS)),-C link-arg=-fuse-ld=mold,)

dev: install-rust-dev-tools $(CLOUDFLARED_BIN) $(GATEWAY_BIN)
	cd tauri-app && RUSTC_WRAPPER=$(RUSTC_WRAPPER) RUSTFLAGS="$(RUSTFLAGS)" VITE_PORT=$(PORT) VITE_DEV_FEATURES=true npx tauri dev --config '{"build":{"devUrl":"http://localhost:$(PORT)"}}'

$(FRONTEND_BUILD_OUT): $(FRONTEND_BUILD_INPUTS)
	cd tauri-app && npm run build

$(RUST_RELEASE_BIN): $(RUST_RELEASE_INPUTS)
	cd tauri-app/src-tauri && RUSTC_WRAPPER=$(RUSTC_WRAPPER) RUSTFLAGS="$(RUSTFLAGS)" cargo build --release

prod-build: $(CLOUDFLARED_BIN) $(GATEWAY_BIN) $(FRONTEND_BUILD_OUT) $(RUST_RELEASE_BIN)

prod-run: $(RUST_RELEASE_BIN)
	cd tauri-app/src-tauri && ./target/release/tauri-app

prod: prod-build prod-run

dev-gateway:
	cd gateway && go run main.go

build:
	cd gateway && go build -o bin/gateway main.go
	cd tauri-app && npm run build
	cd tauri-app && npm run tauri build
