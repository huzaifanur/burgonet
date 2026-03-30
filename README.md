# Burgonet

Burgonet is a Linux desktop app for webcam-based hand-to-face detection aimed at reducing trichotillomania and related BFRB face-touching behaviors. It runs locally on-device with a Tauri desktop shell, a Svelte settings UI, and a Python sidecar that uses MediaPipe to detect face and hand landmarks in real time.

The app starts in the system tray, monitors the camera, and plays a configurable alert sound when a fingertip enters the full-face zone. It supports pause/resume from the tray, hot-reloaded settings, camera conflict recovery, and local config persistence.

## Current Architecture

- `src-tauri/`: Rust Tauri shell, tray, config persistence, sidecar lifecycle
- `src/`: Svelte 5 settings UI
- `src-sidecar/`: Python detection pipeline, MediaPipe models, alert generation
- `tests/`: Python unit tests for protocol, zone math, proximity logic, and alert generation

## Requirements

System packages:

```bash
sudo apt update
sudo apt install -y \
  python3 python3-venv python3-dev \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev
```

Rust / Node:

- Rust stable with `cargo`
- Node.js 20+ and `npm`
- `cargo tauri`

## Development Setup

1. Install frontend dependencies:

```bash
npm install
```

2. Create the Python virtual environment:

```bash
python3 -m venv .venv
```

3. Install Python sidecar dependencies:

```bash
.venv/bin/pip install -r src-sidecar/requirements.txt pytest
```

4. Run checks:

```bash
npm run check
.venv/bin/pytest -q
```

5. Start the desktop app:

```bash
npm run tauri -- dev
```

## Install From Package

Build the Debian package:

```bash
npm run tauri -- build
```

Install the generated package:

```bash
sudo apt install ./src-tauri/target/release/bundle/deb/Burgonet_0.1.0_amd64.deb
```

Launch Burgonet from your application menu or by running:

```bash
burgonet
```

The packaged app bundles the Python sidecar inside the application, so it does not depend on the repo-local `.venv` after installation.
Alert playback uses standard Linux audio players already present on typical Ubuntu desktop installs, preferring `pw-play`, then `paplay`, `aplay`, and `ffplay`.

## Config File

Burgonet stores settings at:

```text
~/.config/burgonet/config.json
```

Schema:

```json
{
  "alert": {
    "sound": "whistle",
    "custom_sound_paths": [],
    "delay_sec": 0,
    "confidence": "high",
    "confidence_value": 70,
    "volume": 0.7
  },
  "camera": {
    "device_index": 0,
    "resolution": [640, 480],
    "fps": 30
  },
  "camera_conflict": {
    "mode": "auto_resume",
    "retry_interval_sec": 5,
    "notify_on_pause": true,
    "notify_on_resume": true
  },
  "app": {
    "autostart": true,
    "start_minimized": true,
    "notifications_enabled": true
  }
}
```

## Features

- Full-face monitoring with no zone picker
- Alert sounds: none, alarm, beep, vibrate, whistle, affirm
- Delay input with decimal support from `0` to `10` seconds
- Confidence input as a direct percentage field
- Custom zone tuning fields for left, right, top, and bottom edges
- Built-in and custom alert sounds through a dropdown picker
- Tray-based pause/resume and settings access
- Hot-reloaded config updates from the settings window
- Local-only processing with no network requirement for detection

## Camera Conflict Handling

Burgonet watches for camera loss while it is running. If another app such as `cheese`, `ffplay`, or a video-call client grabs the webcam, Burgonet marks tracking as paused, shows a desktop notification, and keeps retrying until the camera becomes available again. When the camera is released, the sidecar reconnects automatically and monitoring resumes without restarting the desktop app.

## Building

Source checks:

```bash
npm run check
.venv/bin/pytest -q
cargo check --manifest-path src-tauri/Cargo.toml
```

Desktop build:

```bash
npm run tauri -- build
```

Build outputs:

- Debian package: `src-tauri/target/release/bundle/deb/Burgonet_0.1.0_amd64.deb`
- RPM package: `src-tauri/target/release/bundle/rpm/Burgonet-0.1.0-1.x86_64.rpm`

Notes:

- Development launches the Python sidecar from the repo-local `.venv`.
- Release packaging bundles the PyInstaller-built sidecar under the app resources.

## BFRB Resources

- IOCDF BFRB overview: https://iocdf.org/about-ocd/related-disorders/bfrb-related-disorders/
- IOCDF trichotillomania overview: https://iocdf.org/about-ocd/related-disorders/trichotillomania/

## License

MIT
