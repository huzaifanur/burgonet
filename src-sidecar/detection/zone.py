from __future__ import annotations

from dataclasses import dataclass
from math import sqrt

from detection.face import Landmark


@dataclass(slots=True)
class FaceZone:
    x_min: float
    y_min: float
    x_max: float
    y_max: float

    def contains(self, x: float, y: float) -> bool:
        return self.x_min <= x <= self.x_max and self.y_min <= y <= self.y_max


def _distance(a: Landmark, b: Landmark) -> float:
    return sqrt((a.x - b.x) ** 2 + (a.y - b.y) ** 2)


def _clamp(value: float) -> float:
    return max(0.0, min(1.0, value))


def calculate_full_face_zone(
    landmarks: list[Landmark] | None,
    offsets: dict[str, int] | None = None,
) -> FaceZone | None:
    if not landmarks:
        return None

    offsets = offsets or {}
    forehead = landmarks[10]
    chin = landmarks[152]
    left_jaw = landmarks[234]
    right_jaw = landmarks[454]

    face_height = _distance(forehead, chin)
    face_width = _distance(left_jaw, right_jaw)
    # The preview is mirrored, so the edge users see on the left is the
    # image-space right edge (and vice versa). left/right_offset_pct are
    # display-space; apply them to the opposite image-space edge.
    left_offset_padding = face_width * (0.05 + offsets.get("left_offset_pct", 0) / 100.0)
    right_offset_padding = face_width * (0.05 + offsets.get("right_offset_pct", 0) / 100.0)
    top_padding = face_height * (0.05 + offsets.get("top_offset_pct", 0) / 100.0)
    bottom_padding = face_height * (0.05 + offsets.get("bottom_offset_pct", 0) / 100.0)

    return FaceZone(
        x_min=_clamp(left_jaw.x - right_offset_padding),
        y_min=_clamp(forehead.y - top_padding),
        x_max=_clamp(right_jaw.x + left_offset_padding),
        y_max=_clamp(chin.y + bottom_padding),
    )
