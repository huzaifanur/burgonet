from __future__ import annotations

from dataclasses import dataclass

from detection.face import Landmark
from detection.zone import FaceZone


FINGERTIP_INDICES = (4, 8, 12, 16, 20)


@dataclass(slots=True)
class ProximityState:
    phase: str = "outside"
    entered_at: float | None = None
    cooldown_until: float = 0.0

    def reset(self) -> None:
        self.phase = "outside"
        self.entered_at = None
        self.cooldown_until = 0.0

    def update(
        self,
        hands: list[list[Landmark]],
        zone: FaceZone | None,
        now: float,
        delay_sec: float,
    ) -> bool:
        hand_in_zone = is_hand_in_zone(hands, zone)

        if self.phase == "cooldown":
            if now >= self.cooldown_until:
                self.phase = "outside"
            else:
                return False

        if self.phase == "outside":
            if not hand_in_zone:
                return False
            if delay_sec <= 0:
                self.phase = "cooldown"
                self.cooldown_until = now + 1.0
                return True
            self.phase = "dwelling"
            self.entered_at = now
            return False

        if self.phase == "dwelling":
            if not hand_in_zone:
                self.phase = "outside"
                self.entered_at = None
                return False
            if self.entered_at is not None and now - self.entered_at >= delay_sec:
                self.phase = "cooldown"
                self.cooldown_until = now + 1.0
                self.entered_at = None
                return True

        return False


def is_hand_in_zone(hands: list[list[Landmark]], zone: FaceZone | None) -> bool:
    if zone is None:
        return False

    for hand in hands:
        for index in FINGERTIP_INDICES:
            if len(hand) > index and zone.contains(hand[index].x, hand[index].y):
                return True
    return False
