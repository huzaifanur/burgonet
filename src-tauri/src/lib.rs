mod config;
mod sidecar;
mod tray;

use std::sync::Mutex;

use config::{load_config_from_disk, save_config_to_disk, AppConfig};
use rfd::FileDialog;
use serde::Serialize;
use serde_json::json;
use tauri::Emitter;
use tauri::Manager;
use tauri::State;
use tauri::WindowEvent;
use tauri_plugin_autostart::ManagerExt as AutostartExt;
use tauri_plugin_notification::NotificationExt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackingStatus {
  Active,
  Paused,
  Error,
}

pub struct AppState {
  pub config: Mutex<AppConfig>,
  pub status: Mutex<TrackingStatus>,
  pub session_alerts: Mutex<u64>,
  pub restart_attempts: Mutex<u8>,
  pub sidecar: Mutex<Option<sidecar::SidecarProcess>>,
  pub shutting_down: Mutex<bool>,
}

impl AppState {
  fn new(config: AppConfig) -> Self {
    Self {
      config: Mutex::new(config),
      status: Mutex::new(TrackingStatus::Paused),
      session_alerts: Mutex::new(0),
      restart_attempts: Mutex::new(0),
      sidecar: Mutex::new(None),
      shutting_down: Mutex::new(false),
    }
  }
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
  Ok(state.config.lock().map_err(|_| "config lock poisoned")?.clone())
}

#[tauri::command]
fn save_config(app: tauri::AppHandle, state: State<'_, AppState>, config: AppConfig) -> Result<AppConfig, String> {
  save_config_to_disk(&config).map_err(|error| error.to_string())?;
  {
    let mut guard = state.config.lock().map_err(|_| "config lock poisoned")?;
    *guard = config.clone();
  }
  sync_autostart(&app, config.app.autostart)?;
  sidecar::send_json_command(&app, json!({ "cmd": "update_config", "config": config.clone() }))?;
  Ok(config)
}

#[tauri::command]
fn test_sound(app: tauri::AppHandle, sound: String) -> Result<(), String> {
  sidecar::send_json_command(&app, json!({ "cmd": "test_sound", "sound": sound }))
}

#[tauri::command]
fn pick_audio_file() -> Option<String> {
  FileDialog::new()
    .add_filter("Audio", &["wav", "mp3"])
    .set_title("Choose alert sound")
    .pick_file()
    .map(|path| path.display().to_string())
}

fn configure_main_window(app: &tauri::AppHandle) -> Result<(), String> {
  let config = {
    let state = app.state::<AppState>();
    let config = state.config.lock().map_err(|_| "config lock poisoned")?.clone();
    config
  };

  if let Some(window) = app.get_webview_window("main") {
    let _ = window.center();
    if config.app.start_minimized {
      let _ = window.hide();
    } else {
      show_main_window(app);
    }
    let _ = window.set_title("Burgonet Settings");
    let window_handle = window.clone();
    window.on_window_event(move |event| {
      if let WindowEvent::CloseRequested { api, .. } = event {
        api.prevent_close();
        let _ = window_handle.hide();
      }
    });
  }

  Ok(())
}

pub fn show_main_window(app: &tauri::AppHandle) {
  if let Some(window) = app.get_webview_window("main") {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
  }
}

fn emit_app_state(app: &tauri::AppHandle) {
  let state = app.state::<AppState>();
  let status = *state.status.lock().unwrap_or_else(|error| error.into_inner());
  let session_alerts = *state
    .session_alerts
    .lock()
    .unwrap_or_else(|error| error.into_inner());
  let _ = app.emit(
    "app-state",
    json!({
      "status": status,
      "session_alerts": session_alerts
    }),
  );
}

fn sync_autostart(app: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
  let autostart = app.autolaunch();
  if enabled {
    autostart.enable().map_err(|error| error.to_string())
  } else {
    autostart.disable().map_err(|error| error.to_string())
  }
}

pub fn notify_if_enabled(app: &tauri::AppHandle, title: impl Into<String>, body: impl Into<String>) {
  let config = app.state::<AppState>().config.lock().ok().map(|guard| guard.clone());
  let Some(config) = config else {
    return;
  };

  if !config.app.notifications_enabled {
    return;
  }

  let _ = app
    .notification()
    .builder()
    .title(title.into())
    .body(body.into())
    .show();
}

pub fn set_tracking_status(app: &tauri::AppHandle, status: TrackingStatus) {
  {
    let state = app.state::<AppState>();
    let result = state.status.lock();
    if let Ok(mut guard) = result {
      *guard = status;
    }
  }
  tray::refresh_tray(app);
  emit_app_state(app);
}

pub fn increment_alert_count(app: &tauri::AppHandle) {
  {
    let state = app.state::<AppState>();
    let result = state.session_alerts.lock();
    if let Ok(mut guard) = result {
      *guard += 1;
    }
  }
  tray::refresh_tray(app);
  emit_app_state(app);
}

pub fn reset_alert_count(app: &tauri::AppHandle) {
  {
    let state = app.state::<AppState>();
    let result = state.session_alerts.lock();
    if let Ok(mut guard) = result {
      *guard = 0;
    }
  }
  tray::refresh_tray(app);
  emit_app_state(app);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  let initial_config = load_config_from_disk().unwrap_or_default();

  tauri::Builder::default()
    .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
      show_main_window(app);
    }))
    .manage(AppState::new(initial_config))
    .plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())
    .plugin(tauri_plugin_notification::init())
    .plugin(tauri_plugin_autostart::Builder::new().app_name("Burgonet").build())
    .setup(|app| {
      let app_handle = app.handle().clone();
      let config = app.state::<AppState>().config.lock().map_err(|_| "config lock poisoned")?.clone();
      sync_autostart(&app_handle, config.app.autostart)?;
      configure_main_window(&app_handle)?;
      tray::build_tray(app)?;
      sidecar::start_sidecar(&app_handle)?;
      emit_app_state(&app_handle);
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![get_config, save_config, test_sound, pick_audio_file])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
