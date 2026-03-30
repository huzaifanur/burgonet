use std::sync::Mutex;

use tauri::{
  image::Image,
  menu::{MenuBuilder, MenuItem, MenuItemBuilder, PredefinedMenuItem},
  tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
  App, AppHandle, Manager, Wry,
};

use crate::{show_main_window, sidecar, AppState, TrackingStatus};

pub struct TrayHandles {
  tray_icon: TrayIcon<Wry>,
  status_item: MenuItem<Wry>,
  toggle_item: MenuItem<Wry>,
  alerts_item: MenuItem<Wry>,
  last_icon_status: Mutex<TrackingStatus>,
}

fn status_icon(status: TrackingStatus) -> Image<'static> {
  let (red, green, blue) = match status {
    TrackingStatus::Active => (18, 168, 109),
    TrackingStatus::Paused => (220, 175, 46),
    TrackingStatus::Error => (213, 66, 90),
  };

  let size = 32u32;
  let radius = 10.5f32;
  let center = (size as f32 - 1.0) / 2.0;
  let mut rgba = vec![0u8; (size * size * 4) as usize];

  for y in 0..size {
    for x in 0..size {
      let dx = x as f32 - center;
      let dy = y as f32 - center;
      let distance = (dx * dx + dy * dy).sqrt();
      let index = ((y * size + x) * 4) as usize;

      if distance <= radius {
        rgba[index] = red;
        rgba[index + 1] = green;
        rgba[index + 2] = blue;
        rgba[index + 3] = 255;
      } else if distance <= radius + 1.3 {
        rgba[index] = (red as f32 * 0.7) as u8;
        rgba[index + 1] = (green as f32 * 0.7) as u8;
        rgba[index + 2] = (blue as f32 * 0.7) as u8;
        rgba[index + 3] = 120;
      }
    }
  }

  Image::new_owned(rgba, size, size)
}

pub fn build_tray(app: &App<Wry>) -> Result<(), String> {
  let status_item = MenuItemBuilder::with_id("status", "Burgonet — Paused")
    .enabled(false)
    .build(app)
    .map_err(|error| error.to_string())?;
  let toggle_item = MenuItemBuilder::with_id("toggle", "Resume Tracking")
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
    .icon(status_icon(TrackingStatus::Paused))
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
    tray_icon,
    status_item,
    toggle_item,
    alerts_item,
    last_icon_status: Mutex::new(TrackingStatus::Paused),
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
  if let Ok(mut icon_status) = handles.last_icon_status.lock() {
    if *icon_status != status {
      let _ = handles.tray_icon.set_icon(Some(status_icon(status)));
      *icon_status = status;
    }
  }
  let _ = handles
    .alerts_item
    .set_text(format!("Session alerts: {session_alerts}"));
}
