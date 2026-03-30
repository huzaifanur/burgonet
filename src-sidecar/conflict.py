from __future__ import annotations

import os
from pathlib import Path


def identify_conflicting_process(device_path: str = "/dev/video0") -> str | None:
    target = Path(device_path)
    current_pid = os.getpid()

    for proc_dir in Path("/proc").iterdir():
        if not proc_dir.name.isdigit():
            continue
        pid = int(proc_dir.name)
        if pid == current_pid:
            continue

        fd_dir = proc_dir / "fd"
        try:
            for fd_entry in fd_dir.iterdir():
                try:
                    if fd_entry.resolve() == target:
                        comm_path = proc_dir / "comm"
                        if comm_path.exists():
                            return comm_path.read_text(encoding="utf-8").strip()
                        return "unknown process"
                except OSError:
                    continue
        except OSError:
            continue

    return None
