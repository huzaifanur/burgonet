from __future__ import annotations

from detection.face import Landmark
from detection.zone import calculate_full_face_zone


def build_landmarks() -> list[Landmark]:
    landmarks = [Landmark(0.0, 0.0, 0.0) for _ in range(468)]
    landmarks[10] = Landmark(0.5, 0.2, 0.0)
    landmarks[152] = Landmark(0.5, 0.8, 0.0)
    landmarks[234] = Landmark(0.3, 0.5, 0.0)
    landmarks[454] = Landmark(0.7, 0.5, 0.0)
    return landmarks


def test_full_face_zone_uses_expected_landmarks() -> None:
    zone = calculate_full_face_zone(build_landmarks())
    assert zone is not None
    assert round(zone.x_min, 2) == 0.28
    assert round(zone.x_max, 2) == 0.72
    assert round(zone.y_min, 2) == 0.17
    assert round(zone.y_max, 2) == 0.83


def test_zone_returns_none_without_face() -> None:
    assert calculate_full_face_zone(None) is None


def test_zone_applies_per_edge_offsets() -> None:
    zone = calculate_full_face_zone(
        build_landmarks(),
        {
            "left_offset_pct": 10,
            "right_offset_pct": -5,
            "top_offset_pct": 5,
            "bottom_offset_pct": 10,
        },
    )
    assert zone is not None
    assert round(zone.x_min, 2) == 0.24
    assert round(zone.x_max, 2) == 0.70
    assert round(zone.y_min, 2) == 0.14
    assert round(zone.y_max, 2) == 0.89
