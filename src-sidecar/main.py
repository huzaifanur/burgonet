from __future__ import annotations

import argparse
import base64
import json
import select
import signal
import sys
import time
import traceback
from dataclasses import dataclass
from datetime import UTC, datetime
from typing import Any

import cv2

from alert import AlertEngine
from camera import CameraConfig, CameraManager, CameraUnavailableError
from config import RuntimeConfig, normalize_config
from conflict import identify_conflicting_process
from detection.face import FaceDetector
from detection.hands import HandDetector
from detection.proximity import FINGERTIP_INDICES, ProximityState, is_hand_in_zone
from detection.zone import calculate_full_face_zone

PREVIEW_INTERVAL_SEC = 0.35
PREVIEW_JPEG_QUALITY = 60
PREVIEW_MAX_EDGE = 720


def emit(event: dict[str, Any]) -> None:
    sys.stdout.write(json.dumps(event) + "\n")
    sys.stdout.flush()


def log(message: str) -> None:
    sys.stderr.write(message + "\n")
    sys.stderr.flush()


def utc_now() -> str:
    return datetime.now(UTC).isoformat()


def mirrored_x(value: float) -> float:
    return max(0.0, min(1.0, 1.0 - value))


def prepare_preview_frame(frame: Any) -> Any:
    mirrored = cv2.flip(frame, 1)
    height, width = mirrored.shape[:2]
    longest_edge = max(height, width)
    if longest_edge <= PREVIEW_MAX_EDGE:
        return mirrored

    scale = PREVIEW_MAX_EDGE / longest_edge
    return cv2.resize(
        mirrored,
        (int(width * scale), int(height * scale)),
        interpolation=cv2.INTER_AREA,
    )


def encode_preview_jpeg(frame: Any) -> str | None:
    ok, encoded = cv2.imencode(".jpg", frame, [int(cv2.IMWRITE_JPEG_QUALITY), PREVIEW_JPEG_QUALITY])
    if not ok:
        return None
    return base64.b64encode(encoded.tobytes()).decode("ascii")


def build_preview_payload(
    frame: Any,
    zone: Any,
    hands: list[list[Any]],
    fps: int,
) -> dict[str, Any] | None:
    preview_frame = prepare_preview_frame(frame)
    jpeg = encode_preview_jpeg(preview_frame)
    if jpeg is None:
        return None

    height, width = preview_frame.shape[:2]
    fingertips: list[dict[str, Any]] = []
    for hand in hands:
        for index in FINGERTIP_INDICES:
            if len(hand) <= index:
                continue
            tip = hand[index]
            x = mirrored_x(tip.x)
            y = max(0.0, min(1.0, tip.y))
            fingertips.append(
                {
                    "x": x,
                    "y": y,
                    "active": bool(zone and zone.contains(tip.x, tip.y)),
                }
            )

    payload: dict[str, Any] = {
        "event": "preview",
        "jpeg": jpeg,
        "width": width,
        "height": height,
        "fps": fps,
        "has_face": zone is not None,
        "has_hands": bool(hands),
        "fingertips": fingertips,
        "hand_in_zone": is_hand_in_zone(hands, zone),
    }

    if zone is not None:
        payload["zone"] = {
            "x_min": mirrored_x(zone.x_max),
            "y_min": zone.y_min,
            "x_max": mirrored_x(zone.x_min),
            "y_max": zone.y_max,
        }

    return payload


def poll_stdin() -> list[dict[str, Any]]:
    if sys.stdin.closed:
        return []

    ready, _, _ = select.select([sys.stdin], [], [], 0)
    commands: list[dict[str, Any]] = []

    if not ready:
        return commands

    line = sys.stdin.readline()
    if not line:
        return commands

    try:
        commands.append(json.loads(line))
    except json.JSONDecodeError:
        emit({"event": "error", "message": "Invalid JSON command"})
    return commands


@dataclass(slots=True)
class RuntimeState:
    config: RuntimeConfig
    camera: CameraManager
    face_detector: FaceDetector
    hand_detector: HandDetector
    alert_engine: AlertEngine
    proximity: ProximityState
    running: bool = True
    manually_paused: bool = False
    camera_available: bool = True
    status_state: str = "active"
    last_preview_at: float = 0.0

    def update_status(self, fps: int = 0) -> None:
        emit({"event": "status", "state": self.status_state, "fps": fps})

    def refresh_models(self) -> None:
        confidence = self.config.confidence
        self.face_detector.reconfigure(confidence)
        self.hand_detector.reconfigure(confidence)

    def refresh_alert_engine(self) -> None:
        self.alert_engine.sound = self.config.raw["alert"]["sound"]
        self.alert_engine.volume = self.config.raw["alert"]["volume"]

    def refresh_camera_config(self) -> None:
        self.camera.config = CameraConfig(
            device_index=self.config.raw["camera"]["device_index"],
            resolution=tuple(self.config.raw["camera"]["resolution"]),
            fps=self.config.raw["camera"]["fps"],
        )

    def set_paused(self, paused: bool, reason: str | None = None) -> None:
        self.manually_paused = paused
        if paused:
            self.status_state = "paused"
        else:
            self.status_state = "active" if self.camera_available else "error"
        payload = {"event": "status", "state": self.status_state, "fps": 0}
        if reason:
            payload["reason"] = reason
        emit(payload)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--config", default="{}", help="JSON config override")
    return parser.parse_args()


def create_runtime_state(config: RuntimeConfig) -> RuntimeState:
    camera = CameraManager(
        CameraConfig(
            device_index=config.raw["camera"]["device_index"],
            resolution=tuple(config.raw["camera"]["resolution"]),
            fps=config.raw["camera"]["fps"],
        )
    )
    face_detector = FaceDetector(config.confidence)
    hand_detector = HandDetector(config.confidence)
    alert_engine = AlertEngine(
        sound=config.raw["alert"]["sound"],
        volume=config.raw["alert"]["volume"],
    )

    return RuntimeState(
        config=config,
        camera=camera,
        face_detector=face_detector,
        hand_detector=hand_detector,
        alert_engine=alert_engine,
        proximity=ProximityState(),
    )


def handle_command(state: RuntimeState, payload: dict[str, Any]) -> None:
    command = payload.get("cmd")

    if command == "pause":
        state.set_paused(True, reason="manual")
        return

    if command == "resume":
        state.set_paused(False, reason="manual")
        return

    if command == "stop":
        state.running = False
        return

    if command == "update_config":
        state.config.merge(payload.get("config", {}))
        state.refresh_models()
        state.refresh_alert_engine()
        state.refresh_camera_config()
        emit({"event": "config_updated", "timestamp": utc_now()})
        state.update_status()
        return

    if command == "test_sound":
        state.alert_engine.play(payload.get("sound", state.config.raw["alert"]["sound"]))
        emit({"event": "test_sound", "sound": payload.get("sound", state.config.raw["alert"]["sound"])})
        return

    emit({"event": "error", "message": f"Unknown command: {command}"})


def attempt_camera_recovery(state: RuntimeState, error: CameraUnavailableError) -> None:
    retry_interval = state.config.raw["camera_conflict"]["retry_interval_sec"]
    process_name = identify_conflicting_process(str(state.camera.device_path()))
    reason = "conflict" if process_name else error.reason
    state.camera_available = False
    state.status_state = "paused" if (state.manually_paused or reason == "conflict") else "error"
    state.proximity.reset()
    emit({"event": "camera_lost", "process": process_name or "unknown process", "reason": reason})
    state.update_status()
    next_retry_at = time.monotonic()

    while state.running:
        for command in poll_stdin():
            handle_command(state, command)
            if not state.running:
                return

        if time.monotonic() < next_retry_at:
            time.sleep(0.1)
            continue

        try:
            state.camera.open()
            state.camera_available = True
            state.status_state = "paused" if state.manually_paused else "active"
            state.proximity.reset()
            emit({"event": "camera_recovered"})
            state.update_status()
            return
        except CameraUnavailableError:
            next_retry_at = time.monotonic() + retry_interval


def close_runtime(state: RuntimeState) -> None:
    for closer in (
        state.camera.release,
        state.face_detector.close,
        state.hand_detector.close,
        state.alert_engine.close,
    ):
        try:
            closer()
        except Exception:
            continue


def run() -> int:
    args = parse_args()

    try:
        initial_patch = json.loads(args.config)
    except json.JSONDecodeError:
        initial_patch = {}

    config = RuntimeConfig(raw=normalize_config(initial_patch))
    state = create_runtime_state(config)

    def stop_handler(*_: object) -> None:
        state.running = False

    signal.signal(signal.SIGINT, stop_handler)
    signal.signal(signal.SIGTERM, stop_handler)

    try:
        emit({"event": "model_loaded", "model": "face_mesh"})
        emit({"event": "model_loaded", "model": "hands"})

        last_status_time = 0.0
        frame_count = 0
        fps_started_at = time.monotonic()

        try:
            state.camera.open()
        except CameraUnavailableError as error:
            attempt_camera_recovery(state, error)

        while state.running:
            for command in poll_stdin():
                handle_command(state, command)

            if state.manually_paused:
                time.sleep(0.05)
                if time.monotonic() - last_status_time >= 1.0:
                    state.update_status(fps=0)
                    last_status_time = time.monotonic()
                continue

            try:
                frame = state.camera.read()
            except CameraUnavailableError as error:
                attempt_camera_recovery(state, error)
                continue

            if frame_count == 0:
                width, height = state.camera.frame_size(frame)
                log(f"Frame: {width}x{height}")

            now = time.monotonic()
            timestamp_ms = int(now * 1000)
            face_landmarks = state.face_detector.detect(frame, timestamp_ms)
            hand_landmarks = state.hand_detector.detect(frame, timestamp_ms)

            if face_landmarks:
                landmark_count = len(face_landmarks)
                sample = face_landmarks[152]
                log(f"Face landmarks: {landmark_count} chin={sample.x:.3f},{sample.y:.3f},{sample.z:.3f}")
            if hand_landmarks:
                log(
                    f"Hands detected: {len(hand_landmarks)} "
                    f"index_tip={hand_landmarks[0][8].x:.3f},{hand_landmarks[0][8].y:.3f}"
                )

            zone = calculate_full_face_zone(face_landmarks, state.config.raw["zone"])
            if zone:
                log(
                    "Zone: "
                    f"x_min={zone.x_min:.2f} y_min={zone.y_min:.2f} "
                    f"x_max={zone.x_max:.2f} y_max={zone.y_max:.2f}"
                )

            if state.proximity.update(hand_landmarks, zone, now, state.config.delay_sec):
                emit({"event": "alert", "timestamp": utc_now()})
                state.alert_engine.play()

            frame_count += 1
            elapsed = now - fps_started_at
            fps = int(frame_count / elapsed) if elapsed > 0 else 0

            if now - state.last_preview_at >= PREVIEW_INTERVAL_SEC:
                preview_payload = build_preview_payload(
                    frame,
                    zone,
                    hand_landmarks,
                    fps,
                )
                if preview_payload is not None:
                    emit(preview_payload)
                state.last_preview_at = now

            if now - last_status_time >= 1.0:
                state.update_status(fps=fps)
                last_status_time = now

        emit({"event": "shutdown", "timestamp": utc_now()})
        return 0
    except Exception as error:
        emit({"event": "error", "message": str(error)})
        log(traceback.format_exc())
        return 1
    finally:
        close_runtime(state)


if __name__ == "__main__":
    raise SystemExit(run())
