from __future__ import annotations

from config import DEFAULT_CONFIG, RuntimeConfig, normalize_config


def test_normalize_config_clamps_alert_values() -> None:
    config = normalize_config({"alert": {"delay_sec": 99, "volume": -1, "confidence_value": -20}})
    assert config["alert"]["delay_sec"] == 10
    assert config["alert"]["volume"] == 0.0
    assert config["alert"]["confidence_value"] == 0
    assert config["alert"]["confidence"] == "low"


def test_normalize_config_rejects_invalid_resolution_shape() -> None:
    config = normalize_config({"camera": {"resolution": [640]}})
    assert config["camera"]["resolution"] == DEFAULT_CONFIG["camera"]["resolution"]


def test_runtime_config_merge_preserves_existing_sections() -> None:
    runtime = RuntimeConfig()
    runtime.merge({"app": {"notifications_enabled": False}})
    assert runtime.raw["app"]["notifications_enabled"] is False
    assert runtime.raw["alert"]["sound"] == DEFAULT_CONFIG["alert"]["sound"]


def test_runtime_config_updates_confidence_bucket() -> None:
    runtime = RuntimeConfig(raw={"alert": {"confidence_value": 50}})
    assert runtime.raw["alert"]["confidence"] == "medium"


def test_normalize_config_clamps_zone_offsets_and_camera_fps() -> None:
    config = normalize_config({"zone": {"left_offset_pct": 100, "bottom_offset_pct": -99}, "camera": {"fps": 999}})
    assert config["zone"]["left_offset_pct"] == 40
    assert config["zone"]["bottom_offset_pct"] == -20
    assert config["camera"]["fps"] == 60


def test_normalize_config_preserves_decimal_delay() -> None:
    config = normalize_config({"alert": {"delay_sec": 1.5}})
    assert config["alert"]["delay_sec"] == 1.5


def test_normalize_config_keeps_valid_custom_sound_selection() -> None:
    path = "/tmp/custom-alert.wav"
    config = normalize_config({"alert": {"sound": path, "custom_sound_paths": [path, path, "/tmp/not-audio.txt"]}})
    assert config["alert"]["sound"] == path
    assert config["alert"]["custom_sound_paths"] == [path]
