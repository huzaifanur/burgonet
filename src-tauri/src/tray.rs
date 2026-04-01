use std::thread;
use std::time::Duration;

use tauri::{
  menu::{MenuBuilder, MenuItem, MenuItemBuilder, PredefinedMenuItem},
  tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
  App, AppHandle, Manager, Wry,
};

use crate::{show_main_window, sidecar, AppState, TrackingStatus};

pub struct TrayHandles {
  _tray_icon: TrayIcon<Wry>,
  status_item: MenuItem<Wry>,
  toggle_item: MenuItem<Wry>,
  pause_10_item: MenuItem<Wry>,
  alerts_item: MenuItem<Wry>,
}

pub fn build_tray(app: &App<Wry>) -> Result<(), String> {
  let status_item = MenuItemBuilder::with_id("status", "Burgonet — Paused")
    .enabled(false)
    .build(app)
    .map_err(|error| error.to_string())?;
  let toggle_item = MenuItemBuilder::with_id("toggle", "Resume Tracking")
    .build(app)
    .map_err(|error| error.to_string())?;
  let pause_10_item = MenuItemBuilder::with_id("pause_10", "Pause for 5 mins")
    .build(app)
    .map_err(|error| error.to_string())?;
  let settings_item = MenuItemBuilder::with_id("settings", "Settings...")
    .build(app)
    .map_err(|error| error.to_string())?;
  let alerts_item = MenuItemBuilder::with_id("alerts", "Session alerts: 0")
    .enabled(false)
    .build(app)
    .map_err(|error| error.to_string())?;
  let quit_item = MenuItemBuilder::with_id("quit", "Quit")
    .build(app)
    .map_err(|error| error.to_string())?;
  let separator_1 = PredefinedMenuItem::separator(app).map_err(|error| error.to_string())?;
  let separator_2 = PredefinedMenuItem::separator(app).map_err(|error| error.to_string())?;
  let separator_3 = PredefinedMenuItem::separator(app).map_err(|error| error.to_string())?;
  let menu = MenuBuilder::new(app)
    .items(&[
      &status_item,
      &toggle_item,
      &pause_10_item,
      &separator_1,
      &settings_item,
      &separator_2,
      &alerts_item,
      &separator_3,
      &quit_item,
    ])
    .build()
    .map_err(|error| error.to_string())?;

  let tray_icon = TrayIconBuilder::with_id("main")
    .icon(
      app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| "default window icon missing".to_string())?,
    )
    .menu(&menu)
    .show_menu_on_left_click(false)
    .on_menu_event(|app: &AppHandle<Wry>, event| match event.id().as_ref() {
      "toggle" => {
        let status = *app
          .state::<AppState>()
          .status
          .lock()
          .unwrap_or_else(|error| error.into_inner());
        match status {
          TrackingStatus::Active => sidecar::pause(app),
          TrackingStatus::Paused | TrackingStatus::Error => sidecar::resume(app),
        }
      }
      "pause_10" => {
        sidecar::pause(app);
        let app_clone = app.clone();
        thread::spawn(move || {
          thread::sleep(Duration::from_secs(300));
          sidecar::resume(&app_clone);
        });
      }
      "settings" => {
        show_main_window(app);
      }
      "quit" => {
        sidecar::stop_sidecar(app);
        app.exit(0);
      }
      _ => {}
    })
    .on_tray_icon_event(|tray, event| {
      if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        ..
      } = event
      {
        let app = tray.app_handle();
        show_main_window(&app);
      }
    })
    .build(app)
    .map_err(|error| error.to_string())?;

  let handles = TrayHandles {
    _tray_icon: tray_icon,
    status_item,
    toggle_item,
    pause_10_item,
    alerts_item,
  };

  app.manage(handles);
  refresh_tray(app.handle());
  Ok(())
}

pub fn refresh_tray(app: &AppHandle) {
  let status = *app
    .state::<AppState>()
    .status
    .lock()
    .unwrap_or_else(|error| error.into_inner());
  let session_alerts = *app
    .state::<AppState>()
    .session_alerts
    .lock()
    .unwrap_or_else(|error| error.into_inner());
  let handles = app.state::<TrayHandles>();

  let status_text = match status {
    TrackingStatus::Active => "Burgonet — Active",
    TrackingStatus::Paused => "Burgonet — Paused",
    TrackingStatus::Error => "Burgonet — Error",
  };
  let toggle_text = match status {
    TrackingStatus::Active => "Pause Tracking",
    TrackingStatus::Paused | TrackingStatus::Error => "Resume Tracking",
  };

  let _ = handles.status_item.set_text(status_text);
  let _ = handles.toggle_item.set_text(toggle_text);
  let _ = handles.pause_10_item.set_enabled(status == TrackingStatus::Active);
  let _ = handles
    .alerts_item
    .set_text(format!("Session alerts: {session_alerts}"));
}
