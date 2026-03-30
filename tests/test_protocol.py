from __future__ import annotations

import json

from config import DEFAULT_CONFIG, RuntimeConfig, merge_dicts


def test_merge_dicts_overrides_nested_values() -> None:
    merged = merge_dicts(DEFAULT_CONFIG, {"alert": {"sound": "beep"}})
    assert merged["alert"]["sound"] == "beep"


def test_protocol_lines_are_json_serializable() -> None:
    line = {"event": "status", "state": "active", "fps": 20}
    assert json.loads(json.dumps(line)) == line


def test_runtime_config_clamps_invalid_values() -> None:
    config = RuntimeConfig(raw={"alert": {"confidence_value": 999, "sound": "bogus"}})
    assert config.raw["alert"]["confidence_value"] == 100
    assert config.raw["alert"]["sound"] == "whistle"
