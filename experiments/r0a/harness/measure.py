"""Bounded instrumentation: wall time and child resource usage.

Known limitation (documented, not hidden): `ru_maxrss` from RUSAGE_CHILDREN is a
high-water mark across all children of this process, not a per-run peak; and on
Windows a terminated child's own grandchildren are not reliably attributed.
R0-b measures the descendant problem explicitly instead of assuming it away.
"""
from __future__ import annotations

import time
from dataclasses import dataclass, asdict
from typing import Callable

try:  # POSIX only; Windows has no `resource` module
    import resource
except ImportError:  # pragma: no cover - platform dependent
    resource = None


@dataclass
class Sample:
    name: str
    seconds: float
    ok: bool
    error: str | None = None

    def to_dict(self) -> dict:
        return asdict(self)


def child_rusage() -> dict:
    if resource is None:
        return {"user_s": None, "system_s": None, "maxrss_kb": None,
                "status": "NOT_MEASURED", "reason": "python `resource` module unavailable on this platform"}
    ru = resource.getrusage(resource.RUSAGE_CHILDREN)
    return {
        "user_s": round(ru.ru_utime, 4),
        "system_s": round(ru.ru_stime, 4),
        "maxrss_kb": int(ru.ru_maxrss),
        "status": "MEASURED",
    }


def measure(name: str, fn: Callable[[], object], *, timeout_s: float | None = None) -> tuple[object, Sample]:
    started = time.perf_counter()
    try:
        value = fn()
        elapsed = time.perf_counter() - started
        sample = Sample(name=name, seconds=round(elapsed, 4), ok=True)
    except Exception as exc:  # a failing measurement is data, not a crash
        elapsed = time.perf_counter() - started
        sample = Sample(name=name, seconds=round(elapsed, 4), ok=False,
                        error=f"{type(exc).__name__}: {exc}")
        value = None
    if timeout_s is not None and sample.seconds > timeout_s:
        sample.ok = False
        sample.error = f"exceeded budget {timeout_s}s"
    return value, sample


class Stopwatch:
    def __init__(self) -> None:
        self.started = time.perf_counter()

    @property
    def elapsed(self) -> float:
        return round(time.perf_counter() - self.started, 4)
