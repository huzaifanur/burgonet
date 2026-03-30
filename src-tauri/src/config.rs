use std::fs;
use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlertConfig {
  pub sound: String,
  #[serde(default)]
  pub custom_sound_paths: Vec<String>,
  pub delay_sec: f32,
  pub confidence: String,
  pub confidence_value: u8,
  pub volume: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CameraConfig {
  pub device_index: u8,
  pub resolution: [u16; 2],
  pub fps: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZoneConfig {
  pub left_offset_pct: i8,
  pub right_offset_pct: i8,
  pub top_offset_pct: i8,
  pub bottom_offset_pct: i8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CameraConflictConfig {
  pub mode: String,
  pub retry_interval_sec: u8,
  pub notify_on_pause: bool,
  pub notify_on_resume: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppBehaviorConfig {
  pub autostart: bool,
  pub start_minimized: bool,
  pub notifications_enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
  pub alert: AlertConfig,
  pub camera: CameraConfig,
  #[serde(default)]
  pub zone: ZoneConfig,
  pub camera_conflict: CameraConflictConfig,
  pub app: AppBehaviorConfig,
}

impl Default for AppConfig {
  fn default() -> Self {
    Self {
      alert: AlertConfig {
        sound: "whistle".into(),
        custom_sound_paths: Vec::new(),
        delay_sec: 0.0,
        confidence: "high".into(),
        confidence_value: 70,
        volume: 0.7,
      },
      camera: CameraConfig {
        device_index: 0,
        resolution: [640, 480],
        fps: 30,
      },
      zone: ZoneConfig::default(),
      camera_conflict: CameraConflictConfig {
        mode: "auto_resume".into(),
        retry_interval_sec: 5,
        notify_on_pause: true,
        notify_on_resume: true,
      },
      app: AppBehaviorConfig {
        autostart: true,
        start_minimized: true,
        notifications_enabled: true,
      },
    }
  }
}

impl Default for ZoneConfig {
  fn default() -> Self {
    Self {
      left_offset_pct: 0,
      right_offset_pct: 0,
      top_offset_pct: 0,
      bottom_offset_pct: 0,
    }
  }
}

pub fn config_path() -> anyhow::Result<PathBuf> {
  let mut path = dirs::config_dir().context("missing config dir")?;
  path.push("burgonet");
  fs::create_dir_all(&path)?;
  path.push("config.json");
  Ok(path)
}

pub fn load_config_from_disk() -> anyhow::Result<AppConfig> {
  let path = config_path()?;
  if !path.exists() {
    let config = AppConfig::default();
    save_config_to_disk(&config)?;
    return Ok(config);
  }

  let raw = fs::read_to_string(&path)?;
  let parsed: AppConfig = serde_json::from_str(&raw).unwrap_or_default();
  Ok(parsed)
}

pub fn save_config_to_disk(config: &AppConfig) -> anyhow::Result<()> {
  let path = config_path()?;
  let raw = serde_json::to_string_pretty(config)?;
  fs::write(path, raw)?;
  Ok(())
}
