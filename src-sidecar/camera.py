from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import cv2
import numpy as np


class CameraUnavailableError(RuntimeError):
    def __init__(self, message: str, *, reason: str, recoverable: bool = True) -> None:
        super().__init__(message)
        self.reason = reason
        self.recoverable = recoverable


@dataclass(slots=True)
class CameraConfig:
    device_index: int
    resolution: tuple[int, int]
    fps: int


class CameraManager:
    def __init__(self, config: CameraConfig) -> None:
        self.config = config
        self.capture: cv2.VideoCapture | None = None

    def device_path(self) -> Path:
        return Path(f"/dev/video{self.config.device_index}")

    def open(self) -> None:
        self.release()
        device_path = self.device_path()
        if not device_path.exists():
            raise CameraUnavailableError(
                f"Camera device {device_path} not found",
                reason="missing",
                recoverable=False,
            )

        capture = cv2.VideoCapture(self.config.device_index)

        if not capture.isOpened():
            capture.release()
            raise CameraUnavailableError(
                f"Camera {self.config.device_index} is unavailable",
                reason="busy",
            )

        width, height = self.config.resolution
        capture.set(cv2.CAP_PROP_FRAME_WIDTH, width)
        capture.set(cv2.CAP_PROP_FRAME_HEIGHT, height)
        capture.set(cv2.CAP_PROP_FPS, self.config.fps)
        self.capture = capture

    def read(self) -> np.ndarray:
        if self.capture is None:
            raise CameraUnavailableError("Camera is not open", reason="missing", recoverable=False)

        ok, frame = self.capture.read()
        if not ok or frame is None:
            raise CameraUnavailableError("Failed to read frame from camera", reason="stream_lost")
        return frame

    def frame_size(self, frame: np.ndarray) -> tuple[int, int]:
        height, width = frame.shape[:2]
        return width, height

    def release(self) -> None:
        if self.capture is not None:
            self.capture.release()
            self.capture = None
