.PHONY: build install

DEB_DIR := src-tauri/target/release/bundle/deb

build:
	npm run tauri -- build --bundles deb

install:
	test -n "$$(ls -1t $(DEB_DIR)/Burgonet_*_amd64.deb 2>/dev/null | head -n 1)"
	pkill -x burgonet || true
	pkill -x burgonet-sidecar || true
	sudo apt install --reinstall "./$$(ls -1t $(DEB_DIR)/Burgonet_*_amd64.deb | head -n 1)"
