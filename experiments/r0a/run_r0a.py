#!/usr/bin/env python3
"""Entry point: `python experiments/r0a/run_r0a.py {all|control|build|verify|clean}`."""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from harness.cli import main  # noqa: E402

if __name__ == "__main__":
    sys.exit(main())
