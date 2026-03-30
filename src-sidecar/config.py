from __future__ import annotations

from copy import deepcopy
from dataclasses import dataclass, field
from typing import Any


DEFAULT_CONFIG: dict[str, Any] = {
    "alert": {
        "sound": "whistle",
        "custom_sound_paths": [],
        "delay_sec": 0,
        "confidence": "high",
        "confidence_value": 70,
        "volume": 0.7,
    },
    "camera": {
        "device_index": 0,
        "resolution": [640, 480],
        "fps": 30,
    },
    "zone": {
        "left_offset_pct": 0,
        "right_offset_pct": 0,
        "top_offset_pct": 0,
        "bottom_offset_pct": 0,
    },
    "camera_conflict": {
        "mode": "auto_resume",
        "retry_interval_sec": 5,
        "notify_on_pause": True,
        "notify_on_resume": True,
    },
    "app": {
        "autostart": True,
        "start_minimized": True,
        "notifications_enabled": True,
    },
}

VALID_BUILTIN_SOUNDS = {"none", "alarm", "beep", "vibrate", "whistle", "affirm"}
SUPPORTED_AUDIO_EXTENSIONS = {".wav", ".mp3"}


def merge_dicts(base: dict[str, Any], patch: dict[str, Any]) -> dict[str, Any]:
    merged = deepcopy(base)

    for key, value in patch.items():
        if isinstance(value, dict) and isinstance(merged.get(key), dict):
            merged[key] = merge_dicts(merged[key], value)
        else:
            merged[key] = deepcopy(value)

    return merged


def clamp_int(value: Any, default: int, minimum: int, maximum: int) -> int:
    try:
        parsed = int(value)
    except (TypeError, ValueError):
        return default
    return max(minimum, min(maximum, parsed))


def clamp_float(value: Any, default: float, minimum: float, maximum: float) -> float:
    try:
        parsed = float(value)
    except (TypeError, ValueError):
        return default
    return max(minimum, min(maximum, parsed))


def normalize_custom_sound_paths(value: Any) -> list[str]:
    if not isinstance(value, list):
        return []

    normalized: list[str] = []
    seen: set[str] = set()

    for item in value:
        if not isinstance(item, str):
            continue
        path = item.strip()
        if not path:
            continue
        if not any(path.lower().endswith(extension) for extension in SUPPORTED_AUDIO_EXTENSIONS):
            continue
        if path in seen:
            continue
        seen.add(path)
        normalized.append(path)

    return normalized


def normalize_config(raw: dict[str, Any] | None) -> dict[str, Any]:
    merged = merge_dicts(DEFAULT_CONFIG, raw or {})
    alert = merged["alert"]
    camera = merged["camera"]
    zone = merged["zone"]
    camera_conflict = merged["camera_conflict"]
    app = merged["app"]

    alert["custom_sound_paths"] = normalize_custom_sound_paths(alert.get("custom_sound_paths"))

    sound = alert.get("sound", DEFAULT_CONFIG["alert"]["sound"])
    if isinstance(sound, str) and sound in VALID_BUILTIN_SOUNDS:
        alert["sound"] = sound
    elif isinstance(sound, str) and sound in alert["custom_sound_paths"]:
        alert["sound"] = sound
    else:
        alert["sound"] = DEFAULT_CONFIG["alert"]["sound"]

    alert["delay_sec"] = clamp_float(alert.get("delay_sec"), 0, 0, 10)
    alert["confidence_value"] = clamp_int(alert.get("confidence_value"), 70, 0, 100)
    alert["volume"] = clamp_float(alert.get("volume"), 0.7, 0.0, 1.0)

    confidence_value = alert["confidence_value"]
    if confidence_value <= 40:
        alert["confidence"] = "low"
    elif confidence_value <= 60:
        alert["confidence"] = "medium"
    else:
        alert["confidence"] = "high"

    camera["device_index"] = clamp_int(camera.get("device_index"), 0, 0, 16)

    resolution = camera.get("resolution", DEFAULT_CONFIG["camera"]["resolution"])
    if (
        not isinstance(resolution, list)
        or len(resolution) != 2
        or any(not isinstance(value, (int, float)) for value in resolution)
    ):
        resolution = DEFAULT_CONFIG["camera"]["resolution"]
    camera["resolution"] = [clamp_int(resolution[0], 640, 160, 3840), clamp_int(resolution[1], 480, 120, 2160)]
    camera["fps"] = clamp_int(camera.get("fps"), 30, 1, 60)
    zone["left_offset_pct"] = clamp_int(zone.get("left_offset_pct"), 0, -20, 40)
    zone["right_offset_pct"] = clamp_int(zone.get("right_offset_pct"), 0, -20, 40)
    zone["top_offset_pct"] = clamp_int(zone.get("top_offset_pct"), 0, -20, 40)
    zone["bottom_offset_pct"] = clamp_int(zone.get("bottom_offset_pct"), 0, -20, 40)

    camera_conflict["mode"] = "auto_resume"
    camera_conflict["retry_interval_sec"] = clamp_int(
        camera_conflict.get("retry_interval_sec"),
        5,
        1,
        60,
    )
    camera_conflict["notify_on_pause"] = bool(camera_conflict.get("notify_on_pause", True))
    camera_conflict["notify_on_resume"] = bool(camera_conflict.get("notify_on_resume", True))

    app["autostart"] = bool(app.get("autostart", True))
    app["start_minimized"] = bool(app.get("start_minimized", True))
    app["notifications_enabled"] = bool(app.get("notifications_enabled", True))

    return merged


@dataclass(slots=True)
class RuntimeConfig:
    raw: dict[str, Any] = field(default_factory=lambda: deepcopy(DEFAULT_CONFIG))

    def __post_init__(self) -> None:
        self.raw = normalize_config(self.raw)

    def merge(self, patch: dict[str, Any]) -> None:
        self.raw = normalize_config(merge_dicts(self.raw, patch))

    @property
    def confidence(self) -> float:
        return self.raw["alert"]["confidence_value"] / 100.0

    @property
    def delay_sec(self) -> float:
        return self.raw["alert"]["delay_sec"]
