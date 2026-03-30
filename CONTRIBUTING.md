# Contributing

## Development Flow

1. Install the system dependencies listed in `README.md`.
2. Run `npm install`.
3. Create `.venv` with `python3 -m venv .venv`.
4. Install Python dependencies with `.venv/bin/pip install -r src-sidecar/requirements.txt pytest`.
5. Run checks before changing behavior:

```bash
npm run check
.venv/bin/pytest -q
cargo check --manifest-path src-tauri/Cargo.toml
```

## Code Guidelines

- Keep Python `stdout` reserved for JSON protocol messages only.
- Send human-readable sidecar logs to `stderr`.
- Do not add network-dependent behavior to the detection pipeline.
- Preserve the tray-first workflow: app launches hidden, settings open on demand.

## Validation

Before opening a PR or handing off a change:

- Verify the app launches with `npm run tauri -- dev`
- Verify the sidecar starts and emits status events
- Verify settings save to `~/.config/burgonet/config.json`
- Verify `npm run check` and `.venv/bin/pytest -q` both pass
- Verify `npm run tauri -- build` still produces a working package when packaging-related files change
