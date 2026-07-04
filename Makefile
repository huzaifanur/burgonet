.PHONY: build install

DEB_DIR := src-tauri/target/release/bundle/deb

build:
	npm run tauri -- build --bundles deb

install:
	test -n "$$(ls -1t $(DEB_DIR)/Burgonet_*_amd64.deb 2>/dev/null | head -n 1)"
	pkill -x burgonet || true
	pkill -x burgonet-sidecar || true
	# Stage the .deb in /tmp with world-readable permissions; apt's _apt
	# sandbox user cannot read files inside the home directory.
	deb="$$(ls -1t $(DEB_DIR)/Burgonet_*_amd64.deb | head -n 1)"; \
	staged="$$(mktemp /tmp/burgonet-XXXXXX.deb)"; \
	install -m 644 "$$deb" "$$staged"; \
	sudo apt install --reinstall "$$staged"; \
	status=$$?; \
	rm -f "$$staged"; \
	exit $$status
