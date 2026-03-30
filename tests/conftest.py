from __future__ import annotations

import sys
from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parents[1]
SIDE_CAR_ROOT = PROJECT_ROOT / "src-sidecar"

if str(SIDE_CAR_ROOT) not in sys.path:
    sys.path.insert(0, str(SIDE_CAR_ROOT))
