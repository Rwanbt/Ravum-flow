#!/usr/bin/env python3
"""S9 exploratory pilot — paired arms A/B/C/D/E on a synthetic Rust workload.

Honest scope of this harness:
  * it measures the *workflow mechanics* (copy/worktree, commit, build, verify,
    integrate, reconcile), with agent "thinking time" fixed at 0 ms by default;
  * it does NOT measure LLM latency or human supervision;
  * agent work is a deterministic edit, so differences come from the workflow;
  * arms F (existing tool) and G (Flow) are reported NOT_AVAILABLE / NOT_RUN
    unless the tool or prototype actually exists.

Usage:
  python experiments/s9/run_s9.py --runs 5 --agents 3 --out out/pilot.json
  python experiments/s9/run_s9.py --calibration 2 --runs 5      # discard 2 warmup runs
"""
from __future__ import annotations

import argparse
import json
import math
import os
import random
import shutil
import statistics
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[1]
WORK = HERE / "work"
OUT = HERE / "out"

CARGO_TOML = """[workspace]

[package]
name = "s9_workload"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"""

LIB = """pub mod alpha;
pub mod beta;
pub mod gamma;

#[cfg(test)]
mod integration {
    #[test]
    fn all_modules_present() {
        assert!(super::alpha::value() >= 100);
        assert!(super::beta::value() >= 200);
        assert!(super::gamma::value() >= 300);
    }
}
"""

MODULES = {
    "alpha": ('pub fn value() -> i32 {\n    100\n}\n', 'pub fn value() -> i32 {\n    101\n}\n'),
    "beta": ('pub fn value() -> i32 {\n    200\n}\n', 'pub fn value() -> i32 {\n    201\n}\n'),
    "gamma": ('pub fn value() -> i32 {\n    300\n}\n', 'pub fn value() -> i32 {\n    301\n}\n'),
}
# module i is owned by agent i and expected to change to the edited value
EXPECTED = {"alpha": 101, "beta": 201, "gamma": 301}


def _env() -> dict:
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    env.update({
        "GIT_CONFIG_NOSYSTEM": "1",
        "GIT_CONFIG_GLOBAL": str(WORK / "gitconfig-private"),
        "GIT_TERMINAL_PROMPT": "0",
        "CARGO_TERM_COLOR": "never",
        "RUSTFLAGS": "",
        "PYTHONIOENCODING": "utf-8",
    })
    return env


def _rmtree(path: Path) -> None:
    import stat as _stat

    def _onerror(func, p, _exc):
        try:
            os.chmod(p, _stat.S_IWRITE | _stat.S_IREAD)
            func(p)
        except OSError:
            pass

    shutil.rmtree(path, onexc=_onerror)


def run(cmd: list[str], cwd: Path, timeout: float = 180.0, check: bool = True,
        env: dict | None = None) -> subprocess.CompletedProcess:
    cp = subprocess.run(cmd, cwd=str(cwd), env=env or _env(), stdout=subprocess.PIPE,
                        stderr=subprocess.PIPE, timeout=timeout, check=False)
    cp.stdout = cp.stdout.decode("utf-8", "replace")
    cp.stderr = cp.stderr.decode("utf-8", "replace")
    if check and cp.returncode != 0:
        raise RuntimeError(f"{cmd} -> {cp.returncode}\n{cp.stdout}\n{cp.stderr}")
    return cp


def git(cwd: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess:
    return run(["git", "-C", str(cwd), *args], cwd, check=check)


def build_base() -> Path:
    base = WORK / "base"
    if base.exists():
        _rmtree(base)
    base.mkdir(parents=True)
    (base / "Cargo.toml").write_text(CARGO_TOML, encoding="utf-8")
    (base / "src").mkdir()
    (base / "src" / "lib.rs").write_text(LIB, encoding="utf-8")
    for name, (original, _) in MODULES.items():
        (base / "src" / f"{name}.rs").write_text(original, encoding="utf-8")
    git(base, "init", "-q", "-b", "main")
    git(base, "config", "user.name", "S9 Agent")
    git(base, "config", "user.email", "s9@example.invalid")
    git(base, "config", "commit.gpgsign", "false")
    git(base, "config", "core.autocrlf", "false")
    git(base, "add", "-A")
    git(base, "commit", "-q", "-m", "s9: base workload")
    return base


def base_sha(base: Path) -> str:
    return git(base, "rev-parse", "HEAD").stdout.strip()


def _agent_edit(dirpath: Path, name: str, think_ms: int) -> None:
    if think_ms:
        time.sleep(think_ms / 1000.0)
    _, edited = MODULES[name]
    (dirpath / "src" / f"{name}.rs").write_text(edited, encoding="utf-8")


def _cargo_test(dirpath: Path, target_dir: Path, timeout: float = 180.0) -> subprocess.CompletedProcess:
    env = _env()
    env["CARGO_TARGET_DIR"] = str(target_dir)
    return run(["cargo", "test", "--offline", "-q"], dirpath, timeout=timeout, check=False, env=env)


def _check_tree(dirpath: Path) -> dict:
    present = {}
    for name, value in EXPECTED.items():
        text = (dirpath / "src" / f"{name}.rs").read_text(encoding="utf-8")
        present[name] = str(value) in text
    return {"present": present, "lost": [k for k, v in present.items() if not v]}


def _record(arm: str, run_id: int, phases: dict, ok: bool, extra: dict | None = None) -> dict:
    # T_verified stops when the final tree is verified; cleanup happens afterwards
    total = round(sum(v for k, v in phases.items() if k.startswith("t_") and k != "t_cleanup"), 4)
    rec = {"arm": arm, "run": run_id, "phases": {k: round(v, 4) for k, v in phases.items()},
           "t_verified_s": total, "ok": ok}
    if extra:
        rec.update(extra)
    return rec


def arm_a(base: Path, run_id: int, agents: int, think_ms: int) -> dict:
    """A — private full copies + Git integration."""
    root = WORK / "runs" / f"{run_id:02d}-A"
    _rmtree(root)
    root.mkdir(parents=True)
    phases = {}
    t0 = time.perf_counter()
    copies = []
    for i in range(agents):
        dst = root / f"agent{i}"
        run(["git", "clone", "-q", "--no-hardlinks", str(base), str(dst)], root)
        git(dst, "config", "user.name", "S9 Agent")
        git(dst, "config", "user.email", "s9@example.invalid")
        copies.append(dst)
    phases["t_setup"] = time.perf_counter() - t0
    t0 = time.perf_counter()
    for i, dst in enumerate(copies):
        name = list(MODULES)[i]
        _agent_edit(dst, name, think_ms)
        git(dst, "add", "-A")
        git(dst, "commit", "-q", "-m", f"agent {i}: {name}")
    phases["t_work"] = time.perf_counter() - t0
    t0 = time.perf_counter()
    verify = []
    for i, dst in enumerate(copies):
        cp = _cargo_test(dst, root / f"target-agent{i}")
        verify.append(cp.returncode)
    phases["t_verify"] = time.perf_counter() - t0
    t0 = time.perf_counter()
    conflicts = 0
    for i, dst in enumerate(copies):
        git(base, "fetch", "-q", str(dst), "main")
        cp = git(base, "merge", "--no-ff", "-q", "-m", f"integrate agent {i}", "FETCH_HEAD", check=False)
        conflicts += 1 if cp.returncode != 0 else 0
    phases["t_integrate"] = time.perf_counter() - t0
    t0 = time.perf_counter()
    final = _cargo_test(base, root / "target-final")
    phases["t_final_verify"] = time.perf_counter() - t0
    state = _check_tree(base)
    ok = final.returncode == 0 and not state["lost"] and conflicts == 0 and all(rc == 0 for rc in verify)
    return _record("A", run_id, phases, ok, {"conflicts": conflicts, "lost_edits": state["lost"],
                                             "verify_rc": verify, "final_rc": final.returncode})


def arm_b(base: Path, run_id: int, agents: int, think_ms: int) -> dict:
    """B — real `git worktree add`, serialized integration."""
    root = WORK / "runs" / f"{run_id:02d}-B"
    _rmtree(root)
    root.mkdir(parents=True)
    phases = {}
    t0 = time.perf_counter()
    trees = []
    for i in range(agents):
        dst = root / f"wt{i}"
        git(base, "worktree", "add", "-q", "-b", f"s9-b-{run_id}-{i}", str(dst))
        trees.append(dst)
    phases["t_setup"] = time.perf_counter() - t0
    t0 = time.perf_counter()
    branches = []
    for i, dst in enumerate(trees):
        name = list(MODULES)[i]
        _agent_edit(dst, name, think_ms)
        git(dst, "add", "-A")
        git(dst, "commit", "-q", "-m", f"agent {i}: {name}")
        branches.append(git(dst, "rev-parse", "HEAD").stdout.strip())
    phases["t_work"] = time.perf_counter() - t0
    t0 = time.perf_counter()
    verify = [_cargo_test(dst, root / f"target-wt{i}").returncode for i, dst in enumerate(trees)]
    phases["t_verify"] = time.perf_counter() - t0
    t0 = time.perf_counter()
    conflicts = 0
    for i, oid in enumerate(branches):
        cp = git(base, "merge", "--no-ff", "-q", "-m", f"integrate agent {i}", oid, check=False)
        conflicts += 1 if cp.returncode != 0 else 0
    phases["t_integrate"] = time.perf_counter() - t0
    t0 = time.perf_counter()
    final = _cargo_test(base, root / "target-final")
    phases["t_final_verify"] = time.perf_counter() - t0
    t0 = time.perf_counter()
    for dst in trees:
        git(base, "worktree", "remove", "--force", str(dst), check=False)
    phases["t_cleanup"] = time.perf_counter() - t0
    state = _check_tree(base)
    ok = final.returncode == 0 and not state["lost"] and conflicts == 0 and all(rc == 0 for rc in verify)
    return _record("B", run_id, phases, ok, {"conflicts": conflicts, "lost_edits": state["lost"],
                                             "verify_rc": verify, "final_rc": final.returncode})


def arm_c(base: Path, run_id: int, agents: int, think_ms: int) -> dict:
    """C — one shared directory with a file lock (optimistic guard, serialized)."""
    root = WORK / "runs" / f"{run_id:02d}-C"
    _rmtree(root)
    root.mkdir(parents=True)
    shared = root / "shared"
    run(["git", "clone", "-q", str(base), str(shared)], root)
    git(shared, "config", "user.name", "S9 Agent")
    git(shared, "config", "user.email", "s9@example.invalid")
    lock = root / "writer.lock"
    phases = {"t_setup": 0.0}
    t0 = time.perf_counter()
    verify = []
    for i in range(agents):
        name = list(MODULES)[i]
        while True:  # exclusive writer guard
            try:
                fd = os.open(lock, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
                os.close(fd)
                break
            except FileExistsError:
                time.sleep(0.001)
        try:
            _agent_edit(shared, name, think_ms)
            verify.append(_cargo_test(shared, root / "target-shared").returncode)
            git(shared, "add", "-A")
            git(shared, "commit", "-q", "-m", f"agent {i}: {name}")
        finally:
            os.unlink(lock)
    phases["t_work"] = time.perf_counter() - t0
    phases["t_verify"] = 0.0
    phases["t_integrate"] = 0.0
    t0 = time.perf_counter()
    final = _cargo_test(shared, root / "target-final")
    phases["t_final_verify"] = time.perf_counter() - t0
    state = _check_tree(shared)
    ok = final.returncode == 0 and not state["lost"] and all(rc == 0 for rc in verify)
    return _record("C", run_id, phases, ok, {"conflicts": 0, "lost_edits": state["lost"],
                                             "verify_rc": verify, "final_rc": final.returncode})


def arm_d(base: Path, run_id: int, agents: int, think_ms: int) -> dict:
    """D — strictly sequential, one working copy, no guard."""
    root = WORK / "runs" / f"{run_id:02d}-D"
    _rmtree(root)
    root.mkdir(parents=True)
    phases = {"t_setup": 0.0}
    t0 = time.perf_counter()
    verify = []
    for i in range(agents):
        name = list(MODULES)[i]
        _agent_edit(base, name, think_ms)
        verify.append(_cargo_test(base, root / "target").returncode)
        git(base, "add", "-A")
        git(base, "commit", "-q", "-m", f"agent {i}: {name}")
    phases["t_work"] = time.perf_counter() - t0
    phases["t_verify"] = 0.0
    phases["t_integrate"] = 0.0
    t0 = time.perf_counter()
    final = _cargo_test(base, root / "target-final")
    phases["t_final_verify"] = time.perf_counter() - t0
    state = _check_tree(base)
    ok = final.returncode == 0 and not state["lost"] and all(rc == 0 for rc in verify)
    return _record("D", run_id, phases, ok, {"conflicts": 0, "lost_edits": state["lost"],
                                             "verify_rc": verify, "final_rc": final.returncode})


def arm_e(base: Path, run_id: int, agents: int, think_ms: int) -> dict:
    """E — instrumented O0: worktree agents + receipt per agent + final exact-tree gate."""
    root = WORK / "runs" / f"{run_id:02d}-E"
    _rmtree(root)
    root.mkdir(parents=True)
    phases = {}
    receipts = root / "receipts"
    receipts.mkdir()
    t0 = time.perf_counter()
    trees = []
    for i in range(agents):
        dst = root / f"wt{i}"
        git(base, "worktree", "add", "-q", "-b", f"s9-e-{run_id}-{i}", str(dst))
        trees.append(dst)
    phases["t_setup"] = time.perf_counter() - t0
    t0 = time.perf_counter()
    commits = []
    for i, dst in enumerate(trees):
        name = list(MODULES)[i]
        _agent_edit(dst, name, think_ms)
        git(dst, "add", "-A")
        git(dst, "commit", "-q", "-m", f"agent {i}: {name}")
        commits.append(git(dst, "rev-parse", "HEAD").stdout.strip())
    phases["t_work"] = time.perf_counter() - t0
    t0 = time.perf_counter()
    verify = []
    for i, dst in enumerate(trees):
        cp = _cargo_test(dst, root / f"target-wt{i}")
        tree = git(dst, "rev-parse", "HEAD^{tree}").stdout.strip()
        content = (dst / "src" / f"{list(MODULES)[i]}.rs").read_bytes()
        receipt = {
            "tree_oid": tree, "commit": commits[i], "command": "cargo test --offline -q",
            "exit": cp.returncode, "module": list(MODULES)[i],
            "module_sha256": __import__("hashlib").sha256(content).hexdigest(),
        }
        (receipts / f"agent{i}.json").write_text(json.dumps(receipt, indent=2), encoding="utf-8")
        verify.append(cp.returncode)
    phases["t_verify"] = time.perf_counter() - t0
    t0 = time.perf_counter()
    conflicts = 0
    for i, oid in enumerate(commits):
        cp = git(base, "merge", "--no-ff", "-q", "-m", f"integrate agent {i}", oid, check=False)
        conflicts += 1 if cp.returncode != 0 else 0
    phases["t_integrate"] = time.perf_counter() - t0
    t0 = time.perf_counter()
    final = _cargo_test(base, root / "target-final")
    phases["t_final_verify"] = time.perf_counter() - t0
    # exact-tree gate: every receipt's content hash must match the published tree
    gate = []
    for i in range(agents):
        name = list(MODULES)[i]
        receipt = json.loads((receipts / f"agent{i}.json").read_text(encoding="utf-8"))
        import hashlib
        actual = hashlib.sha256((base / "src" / f"{name}.rs").read_bytes()).hexdigest()
        gate.append(receipt["module_sha256"] == actual and receipt["exit"] == 0)
    t0 = time.perf_counter()
    for dst in trees:
        git(base, "worktree", "remove", "--force", str(dst), check=False)
    phases["t_cleanup"] = time.perf_counter() - t0
    state = _check_tree(base)
    ok = final.returncode == 0 and not state["lost"] and conflicts == 0 and all(gate)
    return _record("E", run_id, phases, ok, {"conflicts": conflicts, "lost_edits": state["lost"],
                                             "receipts_ok": gate, "final_rc": final.returncode})


ARMS = {"A": arm_a, "B": arm_b, "C": arm_c, "D": arm_d, "E": arm_e}


def tools_available() -> dict:
    def probe(cmd: list[str]) -> str:
        try:
            cp = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=10)
            return cp.stdout.decode("utf-8", "replace").strip() or f"rc={cp.returncode}"
        except FileNotFoundError:
            return "NOT_AVAILABLE"
    return {"git": probe(["git", "--version"]), "cargo": probe(["cargo", "--version"]),
            "jj": probe(["jj", "--version"]), "gitbutler": probe(["but", "--version"]),
            "scion": "NOT_AVAILABLE"}


def paired_summary(records: list[dict], arms: list[str]) -> dict:
    by_arm: dict[str, list[dict]] = {a: [r for r in records if r["arm"] == a and r["ok"]] for a in arms}
    summary: dict = {"per_arm": {}, "pairs": {}}
    for a in arms:
        vals = sorted(r["t_verified_s"] for r in by_arm[a])
        if not vals:
            summary["per_arm"][a] = {"n": 0}
            continue
        summary["per_arm"][a] = {"n": len(vals), "median_s": round(statistics.median(vals), 4),
                                 "min_s": vals[0], "max_s": vals[-1],
                                 "mean_s": round(statistics.fmean(vals), 4)}
    for i, a in enumerate(arms):
        for b in arms[i + 1:]:
            diffs, ratios = [], []
            for ra in by_arm[a]:
                for rb in by_arm[b]:
                    if ra["run"] == rb["run"]:
                        diffs.append(ra["t_verified_s"] - rb["t_verified_s"])
                        if rb["t_verified_s"] > 0:
                            ratios.append(ra["t_verified_s"] / rb["t_verified_s"])
            if not diffs:
                summary["pairs"][f"{a}-{b}"] = {"n": 0}
                continue
            wins = sum(1 for d in diffs if d > 0)
            p = _sign_test_p(wins, len(diffs))
            summary["pairs"][f"{a}-{b}"] = {
                "n": len(diffs),
                "median_diff_s": round(statistics.median(diffs), 4),
                "median_ratio": round(statistics.median(ratios), 4) if ratios else None,
                "a_slower_wins": wins, "one_sided_sign_p": p,
                "note": "exploratory only; n below confirmatory size, no ranking claim",
            }
    return summary


def _sign_test_p(wins: int, n: int) -> float:
    """Exact one-sided sign test p = P(X >= wins | p=0.5)."""
    if n == 0:
        return 1.0
    tail = sum(math.comb(n, k) for k in range(wins, n + 1))
    return round(tail / (2 ** n), 6)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", type=int, default=5)
    ap.add_argument("--calibration", type=int, default=0)
    ap.add_argument("--agents", type=int, default=3)
    ap.add_argument("--think-ms", type=int, default=0)
    ap.add_argument("--arms", default="ABCDE")
    ap.add_argument("--seed", type=int, default=20261109)
    ap.add_argument("--out", default="out/pilot.json")
    args = ap.parse_args(argv)

    WORK.mkdir(parents=True, exist_ok=True)
    OUT.mkdir(parents=True, exist_ok=True)
    (WORK / "gitconfig-private").write_text("", encoding="utf-8")
    base = build_base()
    sha = base_sha(base)
    arms = [a for a in args.arms if a in ARMS]
    rng = random.Random(args.seed)
    records = []
    started = datetime.now(timezone.utc)
    total = args.calibration + args.runs
    for run_id in range(1, total + 1):
        order = arms[:]
        rng.shuffle(order)
        for arm in order:
            # every arm starts from the same pristine commit
            git(base, "worktree", "prune", check=False)
            git(base, "checkout", "-q", "-B", "main", sha, check=False)
            git(base, "reset", "-q", "--hard", sha, check=False)
            try:
                rec = ARMS[arm](base, run_id, args.agents, args.think_ms)
            except Exception as exc:  # a failed arm is data, never a silent pass
                rec = {"arm": arm, "run": run_id, "phases": {}, "t_verified_s": None,
                       "ok": False, "error": f"{type(exc).__name__}: {exc}",
                       "calibration": run_id <= args.calibration}
            rec.setdefault("calibration", run_id <= args.calibration)
            records.append(rec)
            t = rec["t_verified_s"]
            print(f"run {run_id:02d}/{total} arm {arm}: "
                  f"{'FAILED' if t is None else format(t, '8.3f')}s ok={rec['ok']} "
                  f"lost={rec.get('lost_edits')}", flush=True)
    measured = [r for r in records if not r["calibration"]]
    summary = paired_summary(measured, arms)
    payload = {
        "schema": "flow-s9-pilot/1",
        "kind": "EXPLORATORY_PILOT",
        "confirmatory": False,
        "started_at": started.strftime("%Y-%m-%dT%H:%M:%SZ"),
        "fixture": {"path": base.relative_to(REPO_ROOT).as_posix(), "sha": sha,
                    "agents": args.agents, "think_ms": args.think_ms},
        "os": sys.platform,
        "tools": tools_available(),
        "arms": arms,
        "calibration_runs": args.calibration,
        "records": records,
        "summary": summary,
        "limitations": [
            "agent thinking time is a constant (default 0 ms): LLM latency is out of scope",
            "single machine, 16 GB, warm cargo cache inside the campaign",
            "arms are run sequentially; CPU/RAM contention between concurrent agents is not measured",
            "no arm F/G: no existing tool installed and no Flow prototype exists",
            "n is exploratory; no p95, no confirmatory claim, no ranking",
        ],
    }
    out = OUT / Path(args.out).name if "/" in args.out else OUT / args.out
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"out": str(out), "summary": summary}, indent=2))
    bad = [r for r in records if not r["ok"]]
    return 0 if not bad else 1


if __name__ == "__main__":
    sys.exit(main())
