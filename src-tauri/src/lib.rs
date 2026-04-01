mod config;
mod sidecar;
mod tray;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use config::{load_config_from_disk, save_config_to_disk, AppConfig};
#[cfg(target_os = "linux")]
use gdk::WindowTypeHint;
#[cfg(target_os = "linux")]
use gtk::prelude::GtkWindowExt;
use rfd::FileDialog;
use serde::Serialize;
use serde_json::json;
use tauri::Emitter;
use tauri::Manager;
use tauri::PhysicalPosition;
use tauri::PhysicalSize;
use tauri::State;
use tauri::WebviewUrl;
use tauri::WebviewWindowBuilder;
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
  pub flash_hide_token: AtomicU64,
  pub last_flash_at_ms: Mutex<u64>,
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
      flash_hide_token: AtomicU64::new(0),
      last_flash_at_ms: Mutex::new(0),
    }
  }
}

const FLASH_DURATION_MS: u64 = 900;
const FLASH_COOLDOWN_MS: u64 = 3000;

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

fn ensure_flash_window(app: &tauri::AppHandle) -> Result<(), String> {
  if app.get_webview_window("flash").is_some() {
    return Ok(());
  }

  let main_window = app
    .get_webview_window("main")
    .ok_or_else(|| "main window missing".to_string())?;

  let builder = WebviewWindowBuilder::new(app, "flash", WebviewUrl::App("flash.html".into()))
    .title("")
    .visible(false)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .focusable(false)
    .focused(false);

  #[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
  ))]
  let builder = builder.transient_for(&main_window).map_err(|error| error.to_string())?;

  let window = builder.build().map_err(|error| error.to_string())?;
  apply_flash_window_hints(&window);
  Ok(())
}

#[cfg(target_os = "linux")]
fn apply_flash_window_hints(window: &tauri::WebviewWindow) {
  if let Ok(gtk_window) = window.gtk_window() {
    gtk_window.set_skip_taskbar_hint(true);
    gtk_window.set_skip_pager_hint(true);
    gtk_window.set_accept_focus(false);
    gtk_window.set_focus_on_map(false);
    gtk_window.set_keep_above(true);
    gtk_window.set_decorated(false);
    gtk_window.set_type_hint(WindowTypeHint::Notification);
  }
}

#[cfg(not(target_os = "linux"))]
fn apply_flash_window_hints(_window: &tauri::WebviewWindow) {}

fn current_time_ms() -> u64 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|duration| duration.as_millis() as u64)
    .unwrap_or(0)
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
  show_flash(app);
}

fn show_flash(app: &tauri::AppHandle) {
  let Some(window) = app.get_webview_window("flash") else { return };
  let now_ms = current_time_ms();
  {
    let state = app.state::<AppState>();
    let Ok(mut last_flash_at_ms) = state.last_flash_at_ms.lock() else { return };
    if now_ms.saturating_sub(*last_flash_at_ms) < FLASH_COOLDOWN_MS {
      return;
    }
    *last_flash_at_ms = now_ms;
  }
  if let Ok(Some(monitor)) = window.current_monitor().or_else(|_| window.primary_monitor()) {
    let _ = window.set_position(PhysicalPosition::new(monitor.position().x, monitor.position().y));
    let _ = window.set_size(PhysicalSize::new(monitor.size().width, monitor.size().height));
  }
  apply_flash_window_hints(&window);
  let _ = window.set_skip_taskbar(true);
  let _ = window.set_focusable(false);
  let _ = window.show();
  let _ = window.set_ignore_cursor_events(true);
  let _ = app.emit("flash", ());
  let state = app.state::<AppState>();
  let token = state.flash_hide_token.fetch_add(1, Ordering::Relaxed) + 1;
  let app_clone = app.clone();
  thread::spawn(move || {
    thread::sleep(Duration::from_millis(FLASH_DURATION_MS));
    let state = app_clone.state::<AppState>();
    if state.flash_hide_token.load(Ordering::Relaxed) == token {
      if let Some(w) = app_clone.get_webview_window("flash") {
        let _ = w.hide();
      }
    }
  });
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
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_autostart::Builder::new().app_name("Burgonet").build())
    .setup(|app| {
      let app_handle = app.handle().clone();
      let config = app.state::<AppState>().config.lock().map_err(|_| "config lock poisoned")?.clone();
      sync_autostart(&app_handle, config.app.autostart)?;
      configure_main_window(&app_handle)?;
      ensure_flash_window(&app_handle)?;
      tray::build_tray(app)?;
      sidecar::start_sidecar(&app_handle)?;
      emit_app_state(&app_handle);
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![get_config, save_config, test_sound, pick_audio_file])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
