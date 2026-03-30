from __future__ import annotations

from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass, field
from math import pi
from pathlib import Path
import shutil
import subprocess
import tempfile
import wave

import numpy as np


SAMPLE_RATE = 44_100
SUPPORTED_AUDIO_EXTENSIONS = {".wav", ".mp3"}


def _time_array(duration: float) -> np.ndarray:
    return np.linspace(0, duration, int(SAMPLE_RATE * duration), False)


def sine_wave(freq: float | np.ndarray, duration: float, volume: float) -> np.ndarray:
    t = _time_array(duration)
    if isinstance(freq, np.ndarray):
        waveform = np.sin(2 * pi * freq[: len(t)] * t)
    else:
        waveform = np.sin(2 * pi * freq * t)
    return waveform * volume


def render_sound(sound: str, volume: float) -> np.ndarray:
    if sound == "none":
        return np.zeros(0, dtype=np.int16)

    if sound == "beep":
        samples = sine_wave(440.0, 0.2, volume)
    elif sound == "alarm":
        t = _time_array(0.6)
        sweep = np.interp(t, [0.0, 0.3, 0.6], [420.0, 880.0, 420.0])
        samples = np.sin(2 * pi * sweep * t) * volume
    elif sound == "vibrate":
        pulse = sine_wave(80.0, 0.1, volume)
        gap = np.zeros(int(SAMPLE_RATE * 0.05))
        samples = np.concatenate([pulse, gap, pulse, gap, pulse])
    elif sound == "whistle":
        t = _time_array(0.3)
        sweep = np.linspace(800.0, 1600.0, len(t))
        samples = np.sin(2 * pi * sweep * t) * volume
    elif sound == "affirm":
        first = sine_wave(523.25, 0.1, volume)
        second = sine_wave(659.25, 0.2, volume)
        samples = np.concatenate([first, second])
    else:
        samples = sine_wave(440.0, 0.2, volume)

    clipped = np.clip(samples, -1.0, 1.0)
    return (clipped * np.iinfo(np.int16).max).astype(np.int16)


def is_supported_audio_file(sound: str) -> bool:
    suffix = Path(sound).suffix.lower()
    return suffix in SUPPORTED_AUDIO_EXTENSIONS


def write_wave_file(path: Path, buffer: np.ndarray) -> None:
    with wave.open(str(path), "wb") as wav_file:
        wav_file.setnchannels(1)
        wav_file.setsampwidth(2)
        wav_file.setframerate(SAMPLE_RATE)
        wav_file.writeframes(buffer.tobytes())


def player_commands(audio_path: Path) -> list[list[str]]:
    commands: list[list[str]] = []
    if audio_path.suffix.lower() == ".wav":
        candidates = (
            ("pw-play", ["pw-play", str(audio_path)]),
            ("paplay", ["paplay", str(audio_path)]),
            ("aplay", ["aplay", "-q", str(audio_path)]),
            ("ffplay", ["ffplay", "-v", "error", "-nodisp", "-autoexit", str(audio_path)]),
        )
    else:
        candidates = (("ffplay", ["ffplay", "-v", "error", "-nodisp", "-autoexit", str(audio_path)]),)

    for binary, command in candidates:
        if shutil.which(binary):
            commands.append(command)

    return commands


@dataclass(slots=True)
class AlertEngine:
    sound: str = "whistle"
    volume: float = 0.7
    _executor: ThreadPoolExecutor = field(init=False, repr=False)

    def __post_init__(self) -> None:
        self._executor = ThreadPoolExecutor(max_workers=1, thread_name_prefix="burgonet-audio")

    def play(self, sound: str | None = None) -> None:
        self._executor.submit(self._play_blocking, sound or self.sound, self.volume)

    def _play_blocking(self, sound: str, volume: float) -> None:
        custom_audio_path = Path(sound).expanduser()
        if is_supported_audio_file(sound):
            if custom_audio_path.is_file():
                for command in player_commands(custom_audio_path):
                    try:
                        completed = subprocess.run(
                            command,
                            check=False,
                            stdout=subprocess.DEVNULL,
                            stderr=subprocess.DEVNULL,
                        )
                    except OSError:
                        continue

                    if completed.returncode == 0:
                        return
            return

        buffer = render_sound(sound, volume)
        if buffer.size == 0:
            return

        temp_path: Path | None = None
        try:
            with tempfile.NamedTemporaryFile(prefix="burgonet-alert-", suffix=".wav", delete=False) as handle:
                temp_path = Path(handle.name)

            write_wave_file(temp_path, buffer)
            for command in player_commands(temp_path):
                try:
                    completed = subprocess.run(
                        command,
                        check=False,
                        stdout=subprocess.DEVNULL,
                        stderr=subprocess.DEVNULL,
                    )
                except OSError:
                    continue

                if completed.returncode == 0:
                    return
        finally:
            if temp_path is not None:
                temp_path.unlink(missing_ok=True)

    def close(self) -> None:
        self._executor.shutdown(wait=False, cancel_futures=True)
