use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

use crate::{
  increment_alert_count, notify_if_enabled, reset_alert_count, set_tracking_status, AppState,
  TrackingStatus,
};

pub enum SidecarProcess {
  Std(StdSidecarProcess),
  Shell(ShellSidecarProcess),
}

pub struct StdSidecarProcess {
  child: Arc<Mutex<Child>>,
  stdin: Arc<Mutex<ChildStdin>>,
}

pub struct ShellSidecarProcess {
  child: Arc<Mutex<Option<CommandChild>>>,
}

fn project_root() -> Result<PathBuf, String> {
  let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  manifest_dir
    .parent()
    .map(|path| path.to_path_buf())
    .ok_or_else(|| "failed to derive project root".to_string())
}

fn python_path() -> Result<PathBuf, String> {
  let root = project_root()?;
  for candidate in venv_python_candidates(&root) {
    if candidate.exists() {
      return Ok(candidate);
    }
  }

  if cfg!(target_os = "windows") {
    Ok(PathBuf::from("python"))
  } else {
    Ok(PathBuf::from("python3"))
  }
}

fn venv_python_candidates(root: &std::path::Path) -> Vec<PathBuf> {
  if cfg!(target_os = "windows") {
    vec![
      root.join(".venv/Scripts/python.exe"),
      root.join(".venv/Scripts/python"),
    ]
  } else {
    vec![root.join(".venv/bin/python")]
  }
}

fn sidecar_script_path() -> Result<PathBuf, String> {
  Ok(project_root()?.join("src-sidecar/main.py"))
}

fn sidecar_base_name() -> &'static str {
  "burgonet-sidecar"
}

fn use_std_sidecar_launcher() -> bool {
  cfg!(debug_assertions) || std::env::var_os("BURGONET_SIDECAR_PATH").is_some()
}

fn create_std_sidecar_command() -> Result<Command, String> {
  if let Some(path) = std::env::var_os("BURGONET_SIDECAR_PATH") {
    return Ok(Command::new(PathBuf::from(path)));
  }

  let mut command = Command::new(python_path()?);
  command.arg(sidecar_script_path()?);
  Ok(command)
}

pub fn start_sidecar(app: &AppHandle) -> Result<(), String> {
  let config = {
    let state = app.state::<AppState>();
    let config = state.config.lock().map_err(|_| "config lock poisoned")?.clone();
    config
  };

  if use_std_sidecar_launcher() {
    start_std_sidecar(app, &config)
  } else {
    start_packaged_sidecar(app, &config)
  }
}

fn start_std_sidecar(app: &AppHandle, config: &crate::config::AppConfig) -> Result<(), String> {
  let mut command = create_std_sidecar_command()?;
  let mut child = command
    .arg("--config")
    .arg(serde_json::to_string(config).map_err(|error| error.to_string())?)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .stdin(Stdio::piped())
    .spawn()
    .map_err(|error| error.to_string())?;

  let stdin = child.stdin.take().ok_or("missing sidecar stdin")?;
  let stdout = child.stdout.take().ok_or("missing sidecar stdout")?;
  let stderr = child.stderr.take().ok_or("missing sidecar stderr")?;

  let child = Arc::new(Mutex::new(child));
  let stdin = Arc::new(Mutex::new(stdin));

  {
    let state = app.state::<AppState>();
    let mut guard = state.sidecar.lock().map_err(|_| "sidecar lock poisoned")?;
    *guard = Some(SidecarProcess::Std(StdSidecarProcess {
      child: child.clone(),
      stdin: stdin.clone(),
    }));
  }

  spawn_stdout_reader(app.clone(), stdout);
  spawn_stderr_reader(stderr);
  spawn_std_exit_watcher(app.clone(), child);
  Ok(())
}

fn start_packaged_sidecar(app: &AppHandle, config: &crate::config::AppConfig) -> Result<(), String> {
  let sidecar_command = app
    .shell()
    .sidecar(sidecar_base_name())
    .map_err(|error| error.to_string())?
    .args([
      "--config",
      &serde_json::to_string(config).map_err(|error| error.to_string())?,
    ]);

  let (mut rx, child) = sidecar_command.spawn().map_err(|error| error.to_string())?;
  let child = Arc::new(Mutex::new(Some(child)));

  {
    let state = app.state::<AppState>();
    let mut guard = state.sidecar.lock().map_err(|_| "sidecar lock poisoned")?;
    *guard = Some(SidecarProcess::Shell(ShellSidecarProcess {
      child: child.clone(),
    }));
  }

  let app_handle = app.clone();
  tauri::async_runtime::spawn(async move {
    while let Some(event) = rx.recv().await {
      match event {
        CommandEvent::Stdout(line) => handle_sidecar_stdout(&app_handle, line),
        CommandEvent::Stderr(line) => log::info!("sidecar: {}", String::from_utf8_lossy(&line)),
        CommandEvent::Error(error) => log::error!("sidecar event error: {error}"),
        CommandEvent::Terminated(payload) => {
          if let Ok(mut guard) = child.lock() {
            *guard = None;
          }
          handle_sidecar_termination(&app_handle, payload.code);
          break;
        }
        _ => {}
      }
    }
  });

  Ok(())
}

fn spawn_stdout_reader(app: AppHandle, stdout: impl std::io::Read + Send + 'static) {
  thread::spawn(move || {
    for line in BufReader::new(stdout).lines() {
      match line {
        Ok(line) => handle_sidecar_stdout(&app, line.into_bytes()),
        Err(error) => {
          log::error!("sidecar stdout read failed: {error}");
          break;
        }
      }
    }
  });
}

fn handle_sidecar_stdout(app: &AppHandle, line: Vec<u8>) {
  let line = String::from_utf8_lossy(&line);
  let trimmed = line.trim();
  if trimmed.is_empty() {
    return;
  }

  if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
    handle_sidecar_event(app, value);
  } else {
    log::warn!("ignored non-JSON stdout from sidecar: {trimmed}");
  }
}

fn spawn_stderr_reader(stderr: impl std::io::Read + Send + 'static) {
  thread::spawn(move || {
    for line in BufReader::new(stderr).lines() {
      match line {
        Ok(line) => log::info!("sidecar: {line}"),
        Err(error) => {
          log::error!("sidecar stderr read failed: {error}");
          break;
        }
      }
    }
  });
}

fn spawn_std_exit_watcher(app: AppHandle, child: Arc<Mutex<Child>>) {
  thread::spawn(move || {
    let status = {
      let mut guard = child.lock().unwrap();
      guard.wait()
    };

    let Ok(status) = status else {
      return;
    };

    handle_sidecar_termination(&app, status.code());
  });
}

fn handle_sidecar_termination(app: &AppHandle, status_code: Option<i32>) {
  let state = app.state::<AppState>();
  let shutting_down = *state
    .shutting_down
    .lock()
    .unwrap_or_else(|error| error.into_inner());

  if shutting_down {
    return;
  }

  {
    let mut sidecar = state.sidecar.lock().unwrap_or_else(|error| error.into_inner());
    *sidecar = None;
  }

  set_tracking_status(app, TrackingStatus::Error);

  let mut restart_attempts = state
    .restart_attempts
    .lock()
    .unwrap_or_else(|error| error.into_inner());
  if *restart_attempts >= 3 {
    log::error!(
      "sidecar exited with {}; giving up after 3 attempts",
      format_exit_status(status_code)
    );
    notify_if_enabled(
      app,
      "Burgonet",
      "Detection stopped unexpectedly. Restart limit reached.",
    );
    return;
  }
  *restart_attempts += 1;
  let attempt = *restart_attempts;
  drop(restart_attempts);

  notify_if_enabled(
    app,
    "Burgonet",
    "Detection stopped unexpectedly. Attempting to restart.",
  );
  thread::sleep(Duration::from_secs(5));
  if let Err(error) = start_sidecar(app) {
    log::error!("sidecar restart attempt {attempt} failed: {error}");
    set_tracking_status(app, TrackingStatus::Error);
    notify_if_enabled(
      app,
      "Burgonet",
      format!("Restart attempt {attempt} failed: {error}"),
    );
  }
}

fn format_exit_status(status_code: Option<i32>) -> String {
  status_code
    .map(|code| code.to_string())
    .unwrap_or_else(|| "unknown status".to_string())
}

fn handle_sidecar_event(app: &AppHandle, value: Value) {
  if let Some(event_name) = value.get("event").and_then(Value::as_str) {
    match event_name {
      "status" => match value.get("state").and_then(Value::as_str) {
        Some("active") => {
          {
            let state = app.state::<AppState>();
            let result = state.restart_attempts.lock();
            if let Ok(mut guard) = result {
              *guard = 0;
            }
          }
          set_tracking_status(app, TrackingStatus::Active);
        }
        Some("paused") => set_tracking_status(app, TrackingStatus::Paused),
        Some("error") => set_tracking_status(app, TrackingStatus::Error),
        _ => {}
      },
      "alert" => increment_alert_count(app),
      "camera_recovered" => {
        {
          let state = app.state::<AppState>();
          let result = state.restart_attempts.lock();
          if let Ok(mut guard) = result {
            *guard = 0;
          }
        }
        reset_alert_count(app);
        set_tracking_status(app, TrackingStatus::Active);

        let config = app.state::<AppState>().config.lock().ok().map(|guard| guard.clone());
        if let Some(config) = config {
          if config.app.notifications_enabled && config.camera_conflict.notify_on_resume {
            let _ = app
              .notification()
              .builder()
              .title("Burgonet resumed")
              .body("Camera recovered and monitoring resumed.")
              .show();
          }
        }
      }
      "camera_lost" => {
        let reason = value.get("reason").and_then(Value::as_str).unwrap_or("conflict");
        if reason == "conflict" {
          set_tracking_status(app, TrackingStatus::Paused);
        } else {
          set_tracking_status(app, TrackingStatus::Error);
        }

        let config = app.state::<AppState>().config.lock().ok().map(|guard| guard.clone());
        if let Some(config) = config {
          if config.app.notifications_enabled && config.camera_conflict.notify_on_pause {
            let process = value
              .get("process")
              .and_then(Value::as_str)
              .unwrap_or("unknown process");
            let body = if reason == "conflict" {
              format!("Burgonet paused — camera in use by {process}.")
            } else {
              "Camera became unavailable. Waiting for it to return.".to_string()
            };
            let _ = app
              .notification()
              .builder()
              .title("Burgonet paused")
              .body(body)
              .show();
          }
        }
      }
      "error" => {
        set_tracking_status(app, TrackingStatus::Error);
        if let Some(message) = value.get("message").and_then(Value::as_str) {
          notify_if_enabled(app, "Burgonet error", message);
        }
      }
      _ => {}
    }
  }

  let _ = app.emit("sidecar-event", value);
}

pub fn send_json_command(app: &AppHandle, payload: Value) -> Result<(), String> {
  let state = app.state::<AppState>();
  let guard = state.sidecar.lock().map_err(|_| "sidecar lock poisoned")?;
  let sidecar = guard.as_ref().ok_or("sidecar not running")?;
  let payload = serde_json::to_string(&payload).map_err(|error| error.to_string())?;
  let message = format!("{payload}\n");

  match sidecar {
    SidecarProcess::Std(process) => {
      let mut stdin = process
        .stdin
        .lock()
        .map_err(|_| "sidecar stdin lock poisoned")?;
      stdin
        .write_all(message.as_bytes())
        .map_err(|error| error.to_string())?;
      stdin.flush().map_err(|error| error.to_string())
    }
    SidecarProcess::Shell(process) => {
      let mut child = process
        .child
        .lock()
        .map_err(|_| "sidecar child lock poisoned")?;
      let child = child.as_mut().ok_or("sidecar not running")?;
      child.write(message.as_bytes()).map_err(|error| error.to_string())
    }
  }
}

pub fn stop_sidecar(app: &AppHandle) {
  {
    let state = app.state::<AppState>();
    let result = state.shutting_down.lock();
    if let Ok(mut guard) = result {
      *guard = true;
    }
  }

  let _ = send_json_command(app, serde_json::json!({ "cmd": "stop" }));

  let sidecar = {
    let state = app.state::<AppState>();
    let mut guard = match state.sidecar.lock() {
      Ok(guard) => guard,
      Err(_) => return,
    };
    guard.take()
  };

  match sidecar {
    Some(SidecarProcess::Std(process)) => stop_std_sidecar(process),
    Some(SidecarProcess::Shell(process)) => stop_shell_sidecar(process),
    None => {}
  }
}

fn stop_std_sidecar(process: StdSidecarProcess) {
  let deadline = std::time::Instant::now() + Duration::from_secs(2);
  loop {
    if let Ok(mut guard) = process.child.lock() {
      match guard.try_wait() {
        Ok(Some(_)) => break,
        Ok(None) => {}
        Err(error) => {
          log::error!("failed to wait for sidecar shutdown: {error}");
          break;
        }
      }
    }

    if std::time::Instant::now() >= deadline {
      if let Ok(mut guard) = process.child.lock() {
        if guard.try_wait().ok().flatten().is_none() {
          let _ = guard.kill();
        }
      }
      break;
    }

    thread::sleep(Duration::from_millis(50));
  }
}

fn stop_shell_sidecar(process: ShellSidecarProcess) {
  thread::sleep(Duration::from_secs(2));
  if let Ok(mut guard) = process.child.lock() {
    if let Some(child) = guard.take() {
      let _ = child.kill();
    }
  }
}

pub fn pause(app: &AppHandle) {
  let _ = send_json_command(app, serde_json::json!({ "cmd": "pause" }));
}

pub fn resume(app: &AppHandle) {
  reset_alert_count(app);
  let _ = send_json_command(app, serde_json::json!({ "cmd": "resume" }));
}
