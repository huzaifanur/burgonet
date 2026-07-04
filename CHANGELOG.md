# Changelog

## v0.2.1

- Replaced the packaged sidecar `simpleaudio` backend with WAV playback through system audio players to avoid native `SIGSEGV` crashes on Ubuntu
- Added a camera warmup grace period so macOS camera feeds initialize instead of failing on startup (#2), and skip transient empty frames instead of crashing the sidecar
- Fixed the flash overlay window intercepting mouse clicks (including the titlebar close button): it is now click-through from creation and no longer touches GTK from background threads
- Fixed zone left/right offsets applying to the wrong edge relative to the mirrored preview, and the draft preview no longer disagrees with the saved zone
- Fixed the zone box and fingertip dots drifting off the face when the preview is cropped by the stage aspect ratio
- Fixed camera settings (FPS/resolution/device) not taking effect until restart; the sidecar now reopens the camera on config change
- Fixed "Pause for 5 mins" force-resuming tracking after a later manual pause
- Fixed Quit stalling for a fixed 2 seconds while stopping the packaged sidecar
- Fixed the Preview FPS metric reporting a lifetime average instead of the current rate
- Removed per-frame debug logging and duplicate BGR→RGB conversions from the sidecar hot loop
- Flash overlay now animates opacity instead of background-color, and the status banner no longer uses a backdrop blur over the live preview

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
