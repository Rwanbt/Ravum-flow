"""Canary redaction applied BEFORE anything is persisted.

ENV-04c: a canary must never reach a stored log or artifact, not merely be hidden
in a terminal. `write_redacted` fails closed if a known marker survives.
"""
from __future__ import annotations

import re
from pathlib import Path

from .common import CANARY

MARKERS = [
    CANARY,
    "FLOW_FAKE_SECRET_CANARY",
    "BEGIN FIXTURE KEY",
    # generic shapes that must not be persisted even if a fixture leaks one
    re.compile(r"(?i)(authorization:\s*bearer\s+)[A-Za-z0-9._-]+"),
    re.compile(r"(?i)(gh[pousr]_[A-Za-z0-9]{20,})"),
]


def redact(text: str) -> tuple[str, int]:
    count = 0
    for marker in MARKERS:
        if isinstance(marker, str):
            if marker in text:
                count += text.count(marker)
                text = text.replace(marker, "[REDACTED]")
        else:
            text, n = marker.subn(r"\1[REDACTED]", text)
            count += n
    return text, count


def leaks(text: str) -> list[str]:
    found = []
    for marker in MARKERS:
        if isinstance(marker, str):
            if marker in text:
                found.append(marker)
        elif marker.search(text):
            found.append(marker.pattern)
    return found


def write_redacted(path: Path, text: str, *, label: str = "log") -> dict:
    clean, count = redact(text)
    remaining = leaks(clean)
    if remaining:  # fail closed rather than persist a canary
        raise RuntimeError(f"{label}: unredacted marker survived: {remaining}")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(clean, encoding="utf-8")
    return {"path": str(path), "redactions": count, "bytes": len(clean.encode())}
