from __future__ import annotations

from detection.face import Landmark
from detection.proximity import ProximityState
from detection.zone import FaceZone


def fingertip_hand(x: float, y: float) -> list[Landmark]:
    hand = [Landmark(0.0, 0.0, 0.0) for _ in range(21)]
    hand[8] = Landmark(x, y, 0.0)
    return hand


def test_instant_alert_fires_on_entry() -> None:
    state = ProximityState()
    zone = FaceZone(0.2, 0.2, 0.8, 0.8)
    assert state.update([fingertip_hand(0.5, 0.5)], zone, now=0.0, delay_sec=0) is True


def test_delayed_alert_requires_dwell() -> None:
    state = ProximityState()
    zone = FaceZone(0.2, 0.2, 0.8, 0.8)
    assert state.update([fingertip_hand(0.5, 0.5)], zone, now=0.0, delay_sec=1) is False
    assert state.update([fingertip_hand(0.5, 0.5)], zone, now=0.5, delay_sec=1) is False
    assert state.update([fingertip_hand(0.5, 0.5)], zone, now=1.0, delay_sec=1) is True


def test_decimal_delay_requires_exact_dwell_time() -> None:
    state = ProximityState()
    zone = FaceZone(0.2, 0.2, 0.8, 0.8)
    assert state.update([fingertip_hand(0.5, 0.5)], zone, now=0.0, delay_sec=1.5) is False
    assert state.update([fingertip_hand(0.5, 0.5)], zone, now=1.4, delay_sec=1.5) is False
    assert state.update([fingertip_hand(0.5, 0.5)], zone, now=1.5, delay_sec=1.5) is True


def test_moving_outside_cancels_dwell() -> None:
    state = ProximityState()
    zone = FaceZone(0.2, 0.2, 0.8, 0.8)
    assert state.update([fingertip_hand(0.5, 0.5)], zone, now=0.0, delay_sec=1) is False
    assert state.update([fingertip_hand(0.1, 0.1)], zone, now=0.2, delay_sec=1) is False
    assert state.phase == "outside"


def test_cooldown_suppresses_repeat_alerts() -> None:
    state = ProximityState()
    zone = FaceZone(0.2, 0.2, 0.8, 0.8)
    assert state.update([fingertip_hand(0.5, 0.5)], zone, now=0.0, delay_sec=0) is True
    assert state.update([fingertip_hand(0.5, 0.5)], zone, now=0.2, delay_sec=0) is False
    assert state.update([fingertip_hand(0.5, 0.5)], zone, now=1.2, delay_sec=0) is True
