"""R0-a command line: build fixtures, prove the instrumentation, clean up.

Exit codes: 0 = gate passed, 1 = gate failed, 2 = usage error.
No command touches anything outside experiments/r0a/work except `out/`.
"""
from __future__ import annotations

import json
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

from . import common
from .common import (
    CANARY, FIXTURES, LOGS, OUT, R0A_DIR, REPO_ROOT, WORK, cleanup, ensure_layout,
    inventory_add, manifest_environment, tree_digest, worktree_digest, write_file,
)
from .fixtures import BUILDERS, build_f0_core
from .measure import child_rusage, measure
from .redact import leaks, write_redacted

CONTROL = WORK / "control"
SCHEMA = "flow-r0a-manifest/1"


def _now() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def _write_json(path: Path, obj) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def cmd_build(_args: list[str]) -> int:
    ensure_layout()
    cleanup()
    samples, fixtures, errors = [], [], []
    for name, builder in BUILDERS.items():
        fx, sample = measure(name, builder)
        samples.append(sample.to_dict())
        if fx is None:
            errors.append({"fixture": name, "error": sample.error})
            print(f"[FAIL] {name}: {sample.error}")
            continue
        fixtures.append(fx.to_dict())
        print(f"[ok] {name:14s} tree={fx.tree[:10] or '-':10s} "
              f"{sample.seconds:7.3f}s status={len(fx.status)}")
    inventory_add([FIXTURES, WORK / "f4-checkouts", WORK / "fake-home"])
    wt = worktree_digest(FIXTURES)
    manifest = {
        "schema": SCHEMA,
        "generated_at": _now(),
        "environment": manifest_environment(),
        "repo": {"head": _head_branch()},
        "fixtures": fixtures,
        "build_samples": samples,
        "errors": errors,
        "totals": {"fixtures": len(fixtures), "files": wt["files"], "bytes": wt["bytes"],
                   "worktree_digest": wt["digest"], "build_s": round(sum(s["seconds"] for s in samples), 3)},
        "safety": {"work_root": "experiments/r0a/work", "inventory": "experiments/r0a/work/created-paths.json"},
    }
    _write_json(OUT / "manifest.json", manifest)
    print(json.dumps({"manifest": str(OUT / "manifest.json"), "fixtures": len(fixtures),
                      "errors": len(errors)}))
    return 0 if not errors else 1


def _head_branch() -> str:
    cp = common.run(["git", "-C", str(REPO_ROOT), "rev-parse", "--abbrev-ref", "HEAD"],
                    check=False, timeout=20)
    return cp.stdout.strip() or "UNKNOWN"


def _check(checks: list[dict], cid: str, ok: bool, detail) -> None:
    checks.append({"id": cid, "ok": bool(ok), "detail": detail})
    print(f"[{'PASS' if ok else 'FAIL'}] {cid}")


def _clean_control() -> None:
    """Control builds live in their own root: never touch the main fixtures."""
    if CONTROL.exists():
        common._rmtree(CONTROL)


def cmd_control(_args: list[str]) -> int:
    ensure_layout()
    _clean_control()
    CONTROL.mkdir(parents=True, exist_ok=True)

    # 1. determinism: same content -> same Git tree identity, independent commits
    a, sa = measure("build-control-a", lambda: build_f0_core("f0-control-a", root=CONTROL))
    b, sb = measure("build-control-b", lambda: build_f0_core("f0-control-b", root=CONTROL))
    if a is None or b is None:
        _write_json(OUT / "control.json",
                    {"schema": SCHEMA, "generated_at": _now(), "passed": False,
                     "checks": [{"id": "build_control_fixtures", "ok": False,
                                 "detail": {"a": sa.to_dict(), "b": sb.to_dict()}}]})
        print("GATE B FAILED: control fixtures could not be built", file=sys.stderr)
        return 1
    checks: list[dict] = []
    _check(checks, "determinism", a.tree_digest == b.tree_digest,
           {"tree_a": a.tree, "tree_b": b.tree, "digest_equal": a.tree_digest == b.tree_digest,
            "commit_a": a.commit, "commit_b": b.commit,
            "commit_differ": a.commit != b.commit})

    # 2. the harness distinguishes published tree from dirty worktree (TREE-01 control)
    repo_a = a.path
    write_file(repo_a / "README.md", "# synthetic fixture\n\ncontrol mutation, not committed\n")
    dirty_tree = tree_digest(repo_a)["digest"]
    dirty_wt = worktree_digest(repo_a)["digest"]
    _check(checks, "tree_vs_worktree",
           dirty_tree == a.tree_digest and dirty_wt != a.worktree_digest,
           {"tree_unchanged": dirty_tree == a.tree_digest,
            "worktree_changed": dirty_wt != a.worktree_digest})
    common.git(repo_a, "add", "README.md")
    common.git(repo_a, "commit", "-q", "-m", "control: commit the mutation")
    _check(checks, "commit_changes_tree", tree_digest(repo_a)["digest"] != a.tree_digest,
           {"new_tree": tree_digest(repo_a)["oid"]})

    # 3. cleanup must never delete outside its root
    marker = R0A_DIR / "should-survive.txt"
    marker.write_text("this file is outside WORK and must survive cleanup\n", encoding="utf-8")
    cleanup()
    survived = marker.exists()
    if survived:
        marker.unlink()
    _check(checks, "cleanup_bounded", survived, {"marker": "experiments/r0a/should-survive.txt", "survived": survived})

    # 4. redaction happens before persistence
    probe = LOGS / "redaction-probe.log"
    write_redacted(probe, f"host env canary={CANARY}\nnetwork=none\n", label="redaction-probe")
    persisted = probe.read_text(encoding="utf-8")
    _check(checks, "redaction_before_persist", not leaks(persisted), {"leaks": leaks(persisted), "log": "experiments/r0a/work/logs/redaction-probe.log"})

    # 5. instrument sanity: the timer measures something; rusage is either real or declared unavailable
    _, sample = measure("sleep-50ms", lambda: time.sleep(0.05))
    ru = child_rusage()
    rusage_honest = ru["status"] == "NOT_MEASURED" or ru.get("maxrss_kb", -1) >= 0
    _check(checks, "instrumentation",
           sample.ok and 0.045 <= sample.seconds <= 5.0 and rusage_honest,
           {"sample": sample.to_dict(), "rusage": ru})

    report = {"schema": SCHEMA, "generated_at": _now(), "checks": checks,
              "passed": all(c["ok"] for c in checks), "environment": manifest_environment()}
    _write_json(OUT / "control.json", report)
    _clean_control()
    print(json.dumps({"control": str(OUT / "control.json"), "passed": report["passed"]}))
    if not report["passed"]:
        print("GATE B FAILED: fix the harness before running R0-b", file=sys.stderr)
        return 1
    return 0


def cmd_clean(_args: list[str]) -> int:
    result = cleanup()
    print(json.dumps(result))
    return 0


def cmd_verify(_args: list[str]) -> int:
    """Read-only sanity check of a previous build (no rebuild, no mutation)."""
    manifest = OUT / "manifest.json"
    if not manifest.exists():
        print("no manifest: run `build` first", file=sys.stderr)
        return 2
    data = json.loads(manifest.read_text(encoding="utf-8"))
    problems = []
    for fx in data["fixtures"]:
        path = REPO_ROOT / fx["path"]
        if not path.exists():
            problems.append({"fixture": fx["name"], "problem": "missing"})
            continue
        if fx["tree"] and tree_digest(path)["oid"] != fx["tree"]:
            problems.append({"fixture": fx["name"], "problem": "tree_drift"})
    print(json.dumps({"fixtures": len(data["fixtures"]), "problems": problems}))
    return 0 if not problems else 1


COMMANDS = {"build": cmd_build, "control": cmd_control, "clean": cmd_clean, "verify": cmd_verify}


def main(argv: list[str] | None = None) -> int:
    args = list(sys.argv[1:] if argv is None else argv)
    if not args or args[0] in {"-h", "--help"}:
        print(__doc__)
        print("commands:", ", ".join(COMMANDS))
        return 2
    cmd, rest = args[0], args[1:]
    if cmd == "all":
        rc = cmd_control(rest)
        return rc if rc != 0 else cmd_build(rest)
    if cmd not in COMMANDS:
        print(f"unknown command: {cmd}", file=sys.stderr)
        return 2
    return COMMANDS[cmd](rest)
