from __future__ import annotations

from pathlib import Path

import cv2
import mediapipe as mp
import numpy as np

from detection.face import Landmark


class HandDetector:
    def __init__(self, confidence: float) -> None:
        self.confidence = confidence
        self._vision = mp.tasks.vision
        self._base_options = mp.tasks.BaseOptions
        self._running_mode = self._vision.RunningMode.VIDEO
        self._model = self._create_model(confidence)

    def _create_model(self, confidence: float) -> mp.tasks.vision.HandLandmarker:
        model_path = Path(__file__).resolve().parents[1] / "models" / "hand_landmarker.task"
        options = self._vision.HandLandmarkerOptions(
            base_options=self._base_options(model_asset_path=str(model_path)),
            running_mode=self._running_mode,
            num_hands=2,
            min_hand_detection_confidence=confidence,
            min_hand_presence_confidence=confidence,
            min_tracking_confidence=confidence,
        )
        return self._vision.HandLandmarker.create_from_options(options)

    def reconfigure(self, confidence: float) -> None:
        if abs(self.confidence - confidence) < 1e-6:
            return
        self.close()
        self.confidence = confidence
        self._model = self._create_model(confidence)

    def detect(self, frame: np.ndarray, timestamp_ms: int) -> list[list[Landmark]]:
        rgb_frame = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
        mp_image = mp.Image(image_format=mp.ImageFormat.SRGB, data=rgb_frame)
        result = self._model.detect_for_video(mp_image, timestamp_ms)
        if not result.hand_landmarks:
            return []

        return [
            [Landmark(x=point.x, y=point.y, z=point.z) for point in hand]
            for hand in result.hand_landmarks
        ]

    def close(self) -> None:
        self._model.close()
