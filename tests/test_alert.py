from __future__ import annotations

from pathlib import Path

import numpy as np

from alert import SAMPLE_RATE, is_supported_audio_file, player_commands, render_sound, write_wave_file


def test_render_sound_returns_pcm_samples() -> None:
    data = render_sound("beep", 0.7)
    assert data.dtype == np.int16
    assert len(data) == int(SAMPLE_RATE * 0.2)


def test_none_sound_is_silent() -> None:
    assert render_sound("none", 0.7).size == 0


def test_affirm_sound_has_audio_content() -> None:
    data = render_sound("affirm", 0.7)
    assert data.max() > 0


def test_write_wave_file_writes_riff_header(tmp_path: Path) -> None:
    path = tmp_path / "alert.wav"
    write_wave_file(path, render_sound("beep", 0.7))
    assert path.read_bytes()[:4] == b"RIFF"


def test_player_commands_only_returns_installed_players(monkeypatch) -> None:
    installed = {"pw-play", "ffplay"}
    monkeypatch.setattr("alert.shutil.which", lambda binary: f"/usr/bin/{binary}" if binary in installed else None)
    commands = player_commands(Path("/tmp/alert.wav"))
    assert commands == [
        ["pw-play", "/tmp/alert.wav"],
        ["ffplay", "-v", "error", "-nodisp", "-autoexit", "/tmp/alert.wav"],
    ]


def test_mp3_custom_audio_prefers_ffplay_only(monkeypatch) -> None:
    installed = {"pw-play", "ffplay"}
    monkeypatch.setattr("alert.shutil.which", lambda binary: f"/usr/bin/{binary}" if binary in installed else None)
    commands = player_commands(Path("/tmp/alert.mp3"))
    assert commands == [["ffplay", "-v", "error", "-nodisp", "-autoexit", "/tmp/alert.mp3"]]


def test_supported_audio_file_recognizes_wav_and_mp3() -> None:
    assert is_supported_audio_file("/tmp/alert.wav") is True
    assert is_supported_audio_file("/tmp/alert.mp3") is True
    assert is_supported_audio_file("/tmp/alert.ogg") is False
