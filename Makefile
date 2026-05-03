.PHONY: help install install-linux-deps clean reset-identity install-git-hooks

include build/make/build.mk
include build/make/test.mk
include build/make/fmt-lint.mk

install-linux-deps:
	sudo apt-get update
	sudo apt-get install -y build-essential pkg-config libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

# install frontend dependencies
install:
	cd tauri-app && npm install

# delete the persisted username so the first-run prompt appears again on next launch
reset-identity:
	rm -f "$${XDG_DATA_HOME:-$$HOME/.local/share}/tauri-app/identity.toml"

# clean up build artifacts
clean:
	rm -rf tauri-app/node_modules
	rm -rf tauri-app/dist
	rm -rf tauri-app/src-tauri/target
	rm -rf crdt-core/target
	rm -rf gateway/bin

install-git-hooks:
	install -m 0755 build/scripts/pre-push .git/hooks/pre-push
