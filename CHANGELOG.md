# Changelog

## Unreleased

- Replaced the packaged sidecar `simpleaudio` backend with WAV playback through system audio players to avoid native `SIGSEGV` crashes on Ubuntu

## v0.1.0

- Bootstrapped the Tauri + Svelte + Python repo structure
- Added Linux tray shell and hidden settings window flow
- Added Python sidecar with MediaPipe Tasks-based face and hand landmark detection
- Added full-face zone math, proximity state logic, synthesized alert playback, and JSON protocol loop
- Added config persistence at `~/.config/burgonet/config.json`
- Added initial Svelte settings UI with save/test-sound integration
- Added app-behavior controls for autostart, start minimized, and desktop notifications
- Added Rust-side notifications, tray status icons, and camera recovery handling
- Added bundled PyInstaller sidecar packaging for release builds with `.deb` and `.rpm` outputs
- Added Python unit tests for protocol, zone, proximity, and alert generation
