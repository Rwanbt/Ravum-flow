#!/usr/bin/env python3
"""Extract the executed test inventory from a `cargo test` log.

Usage: python experiments/quality/inventory.py experiments/quality/test-run-full.log
"""

import re
import sys
from pathlib import Path

RUNNING = re.compile(r"\s*Running (?:unittests )?(\S+)")
OK = re.compile(r"test (\S+) \.\.\. ok")


def main(path: str) -> int:
    current = None
    names: list[str] = []
    suites: list[tuple[str, list[str]]] = []
    for line in Path(path).read_text(encoding="utf-8", errors="replace").splitlines():
        match = RUNNING.match(line)
        if match:
            if current:
                suites.append((current, names))
            current = match.group(1)
            names = []
            continue
        match = OK.match(line)
        if match and current:
            names.append(match.group(1))
    if current:
        suites.append((current, names))

    lines = []
    total = 0
    for suite, names in suites:
        total += len(names)
        lines.append(f"## {suite} ({len(names)})")
        lines.extend(f"  {name}" for name in names)
    lines.append(f"## TOTAL ({total})")
    print("\n".join(lines))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
