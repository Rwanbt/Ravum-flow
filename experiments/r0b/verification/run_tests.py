#!/usr/bin/env python3
"""R0-b verification semantics harness (agent C).

Runs five negative tests on disposable synthetic fixtures under
experiments/r0b/verification/scratch and writes:

  * RESULTS.json          one record per test (required fields)
  * logs/<id>.log         raw, redacted command output
  * logs/<id>.json        structured evidence
  * logs/environment.json host/tool inventory
  * logs/artifacts/       patch files and other text evidence

Rules honoured here:
  * writes only under experiments/r0b/verification/ ;
  * never reads or writes experiments/r0a/ at runtime (fixtures are
    recreated in scratch from the r0a *definitions*, which are read-only
    documentation in fixtures.py; the r0a work/fixtures tree was empty
    after the gate-B cleanup);
  * no network, no real secret, no compiler, no build system: only the
    Python interpreter and git;
  * every git command targets a scratch fixture, never the Flow repo;
  * every output is redacted before it is persisted.

Usage: python experiments/r0b/verification/run_tests.py
Exit: 0 if all five tests produced a verdict; 1 if any is BLOCKED/NOT_RUN.
"""
from __future__ import annotations

import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent
SCRATCH = ROOT / "scratch"
LOGS = ROOT / "logs"
ARTIFACTS = LOGS / "artifacts"
TOOLS = ROOT / "tools"
REPO_ROOT = ROOT.parents[2]
HOME = SCRATCH / "home"
GITCONFIG = SCRATCH / "gitconfig"
PY = sys.executable

FIXTURE_PROVENANCE = (
    "fixtures reconstructed in scratch from experiments/r0a/harness/fixtures.py "
    "definitions (build_f4_e2e, build_f5_crash, build_f7_context); the r0a "
    "work/fixtures tree was empty at start because the validated gate-B run "
    "ends with cleanup(). The scratch f7 build script is a deliberate variant: "
    "it reads the ignored .cache/tool.bin and one undeclared env var while its "
    "context_id stays declared-only, because the unmodified r0a build script "
    "does not consume the cache and the test would otherwise be vacuous. "
    "No write ever targeted experiments/r0a/"
)

# ---------------------------------------------------------------------------
# small utilities
# ---------------------------------------------------------------------------


def now_iso() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def sha256(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def sha256_text(text: str) -> str:
    return sha256(text.encode("utf-8"))


def git_env(extra: dict | None = None) -> dict:
    """Private git/python environment: no user config, no prompts, no network."""
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    env.update({
        "GIT_CONFIG_NOSYSTEM": "1",
        "GIT_CONFIG_GLOBAL": str(GITCONFIG),
        "GIT_CONFIG_SYSTEM": str(GITCONFIG),
        "GIT_TERMINAL_PROMPT": "0",
        "HOME": str(HOME),
        "USERPROFILE": str(HOME),
        "LC_ALL": "C",
        "TZ": "UTC",
        "PYTHONIOENCODING": "utf-8",
        "PYTHONDONTWRITEBYTECODE": "1",
        "GIT_AUTHOR_NAME": "Flow R0b Fixture",
        "GIT_AUTHOR_EMAIL": "fixture@example.invalid",
        "GIT_COMMITTER_NAME": "Flow R0b Fixture",
        "GIT_COMMITTER_EMAIL": "fixture@example.invalid",
        # refuse any accidental transport before it starts
        "GIT_ALLOW_PROTOCOL": "file",
    })
    if extra:
        env.update(extra)
    return env


_REDACT_PATTERNS = [
    re.compile(r"[A-Za-z]:\\Users\\[^\\\s\"']+", re.IGNORECASE),
    re.compile(r"/home/[^/\s\"']+"),
    re.compile(r"/Users/[^/\s\"']+"),
]


def redact(text: str) -> str:
    text = text.replace(str(PY), "<PYTHON>")
    for concrete in sorted({str(Path.home()), str(REPO_ROOT), str(ROOT), str(SCRATCH)}, key=len, reverse=True):
        if concrete:
            text = text.replace(concrete, "<PATH>")
    for pat in _REDACT_PATTERNS:
        text = pat.sub("<HOME>", text)
    return text


def redact_obj(value):
    """Redact every string inside a JSON-serialisable structure before persisting it."""
    if isinstance(value, str):
        return redact(value)
    if isinstance(value, list):
        return [redact_obj(item) for item in value]
    if isinstance(value, dict):
        return {key: redact_obj(item) for key, item in value.items()}
    return value


def run(cmd: list[str], cwd: Path, *, env: dict | None = None, timeout: float = 120.0,
        stdin_text: str | None = None) -> dict:
    started = time.perf_counter()
    try:
        cp = subprocess.run(
            cmd, cwd=str(cwd), env=env if env is not None else git_env(),
            input=(stdin_text.encode("utf-8") if stdin_text is not None else None),
            stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout,
        )
        rc, out, err = cp.returncode, cp.stdout, cp.stderr
    except subprocess.TimeoutExpired as exc:
        rc = 124
        out = exc.stdout or b""
        err = (exc.stderr or b"") + b"\n[TIMEOUT]"
    elapsed = time.perf_counter() - started
    return {
        "cmd": cmd,
        "rc": rc,
        "stdout": out.decode("utf-8", errors="replace"),
        "stderr": err.decode("utf-8", errors="replace"),
        "seconds": round(elapsed, 4),
    }


def git(repo: Path, *args: str, check: bool = False, env: dict | None = None) -> dict:
    res = run(["git", "-C", str(repo), *args], cwd=repo, env=env)
    if check and res["rc"] != 0:
        raise RuntimeError(f"git {' '.join(args)} failed rc={res['rc']}: {res['stderr']}")
    return res


def write(path: Path, data: str | bytes) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data.encode("utf-8") if isinstance(data, str) else data)


def commit_all(repo: Path, message: str) -> str:
    git(repo, "add", "-A", check=True)
    git(repo, "commit", "-q", "-m", message, check=True)
    return git(repo, "rev-parse", "HEAD", check=True)["stdout"].strip()


def init_repo(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)
    git(path, "init", "-q", "-b", "main", check=True)
    for key, val in (("core.autocrlf", "false"), ("core.filemode", "false"),
                     ("core.symlinks", "false"), ("commit.gpgsign", "false"),
                     ("advice.detachedHead", "false")):
        git(path, "config", key, val, check=True)


def head_oid(repo: Path) -> str:
    return git(repo, "rev-parse", "HEAD", check=True)["stdout"].strip()


def tree_oid(repo: Path, rev: str = "HEAD") -> str:
    return git(repo, "rev-parse", f"{rev}^{{tree}}", check=True)["stdout"].strip()


def branch_oid(repo: Path, branch: str) -> str:
    return git(repo, "rev-parse", branch, check=True)["stdout"].strip()


def status_porcelain(repo: Path, *, ignored: bool = False) -> list[str]:
    args = ["status", "--porcelain=v1", "-uall"]
    if ignored:
        args.append("--ignored")
    return [ln for ln in git(repo, *args, check=True)["stdout"].splitlines() if ln]


def cleanup_scratch() -> None:
    """Delete only our own scratch tree, retrying once on Windows read-only objects."""
    if not SCRATCH.exists():
        return
    if SCRATCH.resolve().parent != ROOT.resolve():
        raise RuntimeError(f"refusing to remove unexpected path: {SCRATCH}")

    def onexc(func, path, _exc):
        try:
            os.chmod(path, 0o700)
            func(path)
        except OSError:
            pass

    try:
        shutil.rmtree(SCRATCH, onexc=onexc)
    except TypeError:  # Python < 3.12
        shutil.rmtree(SCRATCH, onerror=lambda f, p, e: onexc(f, p, e))


def prepare_layout() -> None:
    cleanup_scratch()
    for path in (SCRATCH, HOME, LOGS, ARTIFACTS, TOOLS):
        path.mkdir(parents=True, exist_ok=True)
    GITCONFIG.write_text("", encoding="utf-8")


# ---------------------------------------------------------------------------
# fixtures (recreated from the R0-a definitions, not read from r0a/work)
# ---------------------------------------------------------------------------

E2E_API = "def area(width: int, height: int) -> int:\n    return width * height\n"
E2E_API_BREAKING = (
    "def area(width: int) -> int:\n"
    "    # breaking signature: height is gone\n"
    "    return width * width\n"
)
E2E_CONSUMER = (
    "from api import area\n\n"
    "def rectangle(w: int, h: int) -> int:\n"
    "    return area(w, h)\n"
)
E2E_TEST = """import unittest

from consumer import rectangle


class TestConsumer(unittest.TestCase):
    def test_rectangle(self):
        self.assertEqual(rectangle(3, 4), 12)

    def test_zero(self):
        self.assertEqual(rectangle(0, 5), 0)


if __name__ == "__main__":
    unittest.main()
"""

F7_BUILD = r'''"""Declared-input build with one deliberately undeclared input.

The declared manifest covers inputs/flags.txt, inputs/lock.txt and two
environment variables. It does NOT cover .cache/tool.bin, which is
ignored by git and read anyway: that is the CACHE-13 / CONTEXT-03 target.
"""
import hashlib
import json
import os
from pathlib import Path

root = Path(__file__).resolve().parents[1]
declared = [Path("inputs/flags.txt"), Path("inputs/lock.txt")]
manifest = {}
for rel in declared:
    manifest[rel.as_posix()] = hashlib.sha256((root / rel).read_bytes()).hexdigest()
manifest["env.FLOW_TOOLCHAIN"] = os.environ.get("FLOW_TOOLCHAIN", "")
manifest["env.LC_ALL"] = os.environ.get("LC_ALL", "")
context_id = hashlib.sha256(json.dumps(manifest, sort_keys=True).encode()).hexdigest()

cache = root / ".cache" / "tool.bin"
cache_sha = hashlib.sha256(cache.read_bytes()).hexdigest() if cache.exists() else "ABSENT"
hidden = os.environ.get("FLOW_HIDDEN_INPUT", "UNSET")
artifact = (
    f"context_id={context_id}\n"
    f"flags={manifest['inputs/flags.txt']}\n"
    f"lock={manifest['inputs/lock.txt']}\n"
    f"cache={cache_sha}\n"
    f"hidden={hidden}\n"
)
artifact_sha = hashlib.sha256(artifact.encode()).hexdigest()
out = root / "out"
out.mkdir(exist_ok=True)
(out / "artifact.txt").write_text(artifact, encoding="utf-8")
(out / "context.json").write_text(
    json.dumps({"context_id": context_id, "artifact_sha256": artifact_sha,
                "cache_sha256": cache_sha, "manifest": manifest},
               sort_keys=True, indent=2),
    encoding="utf-8",
)
print(json.dumps({"context_id": context_id, "artifact_sha256": artifact_sha,
                  "cache_sha256": cache_sha}))
'''

PUBLISHER = r'''"""Minimal intent -> CAS -> outcome publisher with driven crash points."""
import json
import os
import subprocess
import sys


def git(repo, *args, check=True):
    return subprocess.run(["git", "-C", repo, *args], capture_output=True, text=True, check=check)


def fsync_append(path, record):
    with open(path, "a", encoding="utf-8") as fh:
        fh.write(json.dumps(record, sort_keys=True) + "\n")
        fh.flush()
        os.fsync(fh.fileno())


def main():
    repo, crash_at, payload = sys.argv[1], sys.argv[2], sys.argv[3]
    branch = "main"
    journal = os.path.join(git(repo, "rev-parse", "--git-dir").stdout.strip(), "flow-journal.jsonl")
    old = git(repo, "rev-parse", branch).stdout.strip()
    tree = git(repo, "write-tree").stdout.strip()
    commit = git(repo, "commit-tree", tree, "-p", old, "-m", payload).stdout.strip()

    if crash_at == "before-intent":
        os._exit(70)
    fsync_append(journal, {"event": "intent", "old": old, "new": commit, "tree": tree})
    if crash_at == "after-intent":
        os._exit(70)

    res = subprocess.run(["git", "-C", repo, "update-ref", f"refs/heads/{branch}", commit, old],
                         capture_output=True, text=True)
    if res.returncode != 0:
        fsync_append(journal, {"event": "outcome", "status": "REFUSED", "new": commit,
                               "stderr": res.stderr.strip()})
        sys.exit(2)
    if crash_at == "after-ref":
        os._exit(70)
    fsync_append(journal, {"event": "outcome", "status": "PUBLISHED", "new": commit})
    if crash_at == "before-outcome":
        os._exit(70)
    print(json.dumps({"status": "PUBLISHED", "commit": commit, "tree": tree}))


if __name__ == "__main__":
    main()
'''


def build_f4() -> dict:
    repo = SCRATCH / "f4-e2e"
    init_repo(repo)
    write(repo / "api.py", E2E_API)
    write(repo / "consumer.py", E2E_CONSUMER)
    write(repo / "test_e2e.py", E2E_TEST)
    green = commit_all(repo, "fixture: E2E green")
    git(repo, "checkout", "-q", "-b", "breaking", green, check=True)
    write(repo / "api.py", E2E_API_BREAKING)
    breaking = commit_all(repo, "fixture: API break, consumer untouched")
    git(repo, "checkout", "-q", "main", check=True)
    return {"repo": repo, "green": green, "breaking": breaking}


def build_f7(name: str) -> dict:
    repo = SCRATCH / name
    init_repo(repo)
    write(repo / ".gitignore", "out/\n.cache/\n")
    write(repo / "tools" / "build.py", F7_BUILD)
    write(repo / "inputs" / "flags.txt", "-O1\n")
    write(repo / "inputs" / "lock.txt", "dep-a 1.0.0\ndep-b 2.0.0\n")
    base = commit_all(repo, "fixture: declared-input build (scratch variant)")
    return {"repo": repo, "base": base}


def build_f5() -> dict:
    repo = SCRATCH / "f5-crash"
    init_repo(repo)
    write(repo / "publisher.py", PUBLISHER)
    write(repo / "state.txt", "v1\n")
    base = commit_all(repo, "fixture: crash publisher seed")
    return {"repo": repo, "base": base}


# ---------------------------------------------------------------------------
# helper tools (written to experiments/r0b/verification/tools/)
# ---------------------------------------------------------------------------

NAIVE_GATE = r'''"""Naive gate: runs the suite and ignores the return code."""
import json
import subprocess
import sys

cwd = sys.argv[1]
res = subprocess.run([sys.executable, "-m", "unittest", "discover", "-q"], cwd=cwd,
                     capture_output=True, text=True)
print(json.dumps({"policy": "ignore_return_code", "test_rc": res.returncode,
                  "gate_rc": 0, "test_stderr_tail": res.stderr.strip().splitlines()[-3:]},
                 sort_keys=True))
sys.exit(0)
'''

STRICT_GATE = r'''"""Strict gate: the gate result is the suite result."""
import json
import subprocess
import sys

cwd = sys.argv[1]
res = subprocess.run([sys.executable, "-m", "unittest", "discover", "-q"], cwd=cwd,
                     capture_output=True, text=True)
print(json.dumps({"policy": "require_return_code", "test_rc": res.returncode,
                  "gate_rc": res.returncode,
                  "test_stderr_tail": res.stderr.strip().splitlines()[-3:]},
                 sort_keys=True))
sys.exit(res.returncode)
'''

READER_NAIVE_STRICT = r'''"""Naive reader: trusts every line to be JSON. A truncated line crashes it."""
import json
import subprocess
import sys
from pathlib import Path

repo = Path(sys.argv[1])
gd = subprocess.run(["git", "-C", str(repo), "rev-parse", "--git-dir"],
                    capture_output=True, text=True, check=True).stdout.strip()
journal = (repo / gd if not Path(gd).is_absolute() else Path(gd)) / "flow-journal.jsonl"
records = [json.loads(line) for line in journal.read_text(encoding="utf-8").splitlines()]
print(json.dumps({"records": len(records), "last": records[-1] if records else None},
                 sort_keys=True))
'''

READER_NAIVE_SKIP = r'''"""Naive reader: swallows malformed lines and keeps going (silent recovery)."""
import json
import subprocess
import sys
from pathlib import Path

repo = Path(sys.argv[1])
gd = subprocess.run(["git", "-C", str(repo), "rev-parse", "--git-dir"],
                    capture_output=True, text=True, check=True).stdout.strip()
journal = (repo / gd if not Path(gd).is_absolute() else Path(gd)) / "flow-journal.jsonl"
records, bad = [], 0
for line in journal.read_text(encoding="utf-8").splitlines():
    if not line.strip():
        continue
    try:
        records.append(json.loads(line))
    except Exception:
        bad += 1
last = records[-1] if records else None
status = (last or {}).get("event", "NONE")
print(json.dumps({"records": len(records), "bad_lines": bad, "last_event": status,
                  "would_republish": status != "outcome", "last": last}, sort_keys=True))
'''

READER_ROBUST = r'''"""Robust reader: a partial or inconsistent journal is refused, never repaired silently."""
import json
import subprocess
import sys
from pathlib import Path


def git(repo, *args):
    return subprocess.run(["git", "-C", str(repo), *args],
                          capture_output=True, text=True, check=True).stdout.strip()


def refuse(reason, detail=None):
    print(json.dumps({"status": "REFUSED_CORRUPT_JOURNAL", "reason": reason,
                      "detail": detail}), flush=True)
    sys.exit(2)


repo = Path(sys.argv[1])
gd = git(repo, "rev-parse", "--git-dir")
journal = (repo / gd if not Path(gd).is_absolute() else Path(gd)) / "flow-journal.jsonl"
if not journal.exists():
    refuse("journal_missing")
raw = journal.read_bytes()
if not raw:
    refuse("journal_empty")
if not raw.endswith(b"\n"):
    refuse("last_line_not_terminated", {"bytes": len(raw)})
lines = raw.split(b"\n")[:-1]
records = []
for i, line in enumerate(lines, 1):
    if not line.strip():
        refuse("empty_line", {"line": i})
    try:
        rec = json.loads(line.decode("utf-8"))
    except Exception as exc:
        refuse("invalid_json", {"line": i, "error": str(exc)})
    if not isinstance(rec, dict) or rec.get("event") not in {"intent", "outcome"}:
        refuse("schema", {"line": i, "record": rec})
    for key in ("event",):
        if key not in rec:
            refuse("missing_field", {"line": i, "key": key})
    records.append(rec)

events = [r["event"] for r in records]
if events != ["intent", "outcome"] and events != ["intent"]:
    refuse("unexpected_sequence", {"events": events})
ref = git(repo, "rev-parse", "main")
intent = records[0]
if ref != intent.get("new"):
    refuse("ref_does_not_match_intent", {"ref": ref, "intent_new": intent.get("new")})
if events == ["intent"]:
    print(json.dumps({"status": "RECONCILED_PUBLISHED_FROM_REF", "records": len(records),
                      "ref": ref}), flush=True)
    sys.exit(0)
outcome = records[1]
if outcome.get("new") != intent.get("new") or outcome.get("status") != "PUBLISHED":
    refuse("outcome_inconsistent", {"outcome": outcome})
print(json.dumps({"status": "COMPLETE", "records": len(records), "ref": ref}), flush=True)
'''


def write_tools() -> None:
    write(TOOLS / "naive_gate.py", NAIVE_GATE)
    write(TOOLS / "strict_gate.py", STRICT_GATE)
    write(TOOLS / "reader_naive_strict.py", READER_NAIVE_STRICT)
    write(TOOLS / "reader_naive_skip.py", READER_NAIVE_SKIP)
    write(TOOLS / "reader_robust.py", READER_ROBUST)


# ---------------------------------------------------------------------------
# logging helpers
# ---------------------------------------------------------------------------


def log_block(title: str, res: dict) -> str:
    cmd = res["cmd"]
    if isinstance(cmd, list):
        cmd_text = subprocess.list2cmdline(cmd) if os.name == "nt" else " ".join(cmd)
    else:
        cmd_text = str(cmd)
    return (
        f"--- {title}\n"
        f"$ {cmd_text}\n"
        f"rc={res['rc']} seconds={res['seconds']}\n"
        f"stdout:\n{res['stdout']}\n"
        f"stderr:\n{res['stderr']}\n"
    )


def write_log(test_id: str, body: str, structured: dict | None = None) -> list[str]:
    text = redact(body)
    (LOGS / f"{test_id}.log").write_text(text, encoding="utf-8")
    evidence = [f"logs/{test_id}.log"]
    if structured is not None:
        (LOGS / f"{test_id}.json").write_text(
            json.dumps(redact_obj(structured), indent=2, sort_keys=True) + "\n", encoding="utf-8")
        evidence.append(f"logs/{test_id}.json")
    return evidence


def base_record(test_id: str, verdict: str, fixture: str, commands: list[str],
                observation: str, scope: str, evidence: list[str]) -> dict:
    return {
        "id": test_id,
        "verdict": verdict,
        "fixture": fixture,
        "os": f"{sys.platform} / {platform.platform(terse=True)} / Python {platform.python_version()}",
        "commands": [redact(command) for command in commands],
        "observation": observation,
        "scope": scope,
        "evidence": evidence,
    }


# ---------------------------------------------------------------------------
# tests
# ---------------------------------------------------------------------------


def test_e2e_12() -> dict:
    fx = build_f4()
    repo, green, breaking = fx["repo"], fx["green"], fx["breaking"]
    worktrees = SCRATCH / "worktrees"
    wt_green = worktrees / "green"
    wt_break = worktrees / "breaking"
    worktrees.mkdir(parents=True, exist_ok=True)
    res_wt_green = git(repo, "worktree", "add", "-q", "--detach", str(wt_green), green)
    res_wt_break = git(repo, "worktree", "add", "-q", "--detach", str(wt_break), breaking)
    r_green = run([PY, "-m", "unittest", "discover", "-q"], cwd=wt_green)
    r_break = run([PY, "-m", "unittest", "discover", "-q"], cwd=wt_break)

    naive = run([PY, str(TOOLS / "naive_gate.py"), str(wt_break)], cwd=wt_break)
    strict = run([PY, str(TOOLS / "strict_gate.py"), str(wt_break)], cwd=wt_break)
    cmd_amp = {"rc": None, "stdout": "", "stderr": "NOT_RUN", "cmd": [], "seconds": 0.0}
    cmd_tests_ran = False
    if os.name == "nt":
        # Windows analogue of `pytest ... || true`: `& exit /b 0` runs the
        # second command unconditionally, so the shell status is 0.
        inner = "python -m unittest discover -q & exit /b 0"
        cmd_amp = run(["cmd", "/c", inner], cwd=wt_break)
        cmd_tests_ran = ("FAILED" in cmd_amp["stderr"]) or ("Ran 2 tests" in cmd_amp["stderr"])

    body = (
        "E2E-12 — unchanged consumer, API break, three gate policies\n"
        f"fixture: f4-e2e (green={green[:10]}, breaking={breaking[:10]})\n"
        "consumer.py is byte-identical on both revisions: "
        + sha256_text(E2E_CONSUMER) + "\n\n"
        + log_block("worktree add green", res_wt_green)
        + log_block("worktree add breaking", res_wt_break)
        + log_block("tests on green", r_green)
        + log_block("tests on breaking", r_break)
        + log_block("naive gate (ignores test return code)", naive)
        + log_block("strict gate (requires test return code)", strict)
        + log_block("cmd '& exit /b 0' variant (Windows || true analogue)", cmd_amp)
    )
    structured = {
        "fixture": "f4-e2e",
        "consumer_sha256": sha256_text(E2E_CONSUMER),
        "green_rc": r_green["rc"],
        "breaking_rc": r_break["rc"],
        "naive_gate_rc": naive["rc"],
        "strict_gate_rc": strict["rc"],
        "cmd_and_exit0_rc": cmd_amp["rc"],
        "cmd_and_exit0_ran_tests": cmd_tests_ran,
        "green_tree": tree_oid(repo, green),
        "breaking_tree": tree_oid(repo, breaking),
    }
    evidence = write_log("E2E-12", body, structured)
    ok = (r_green["rc"] == 0 and r_break["rc"] != 0 and naive["rc"] == 0
          and strict["rc"] != 0
          and (os.name != "nt" or (cmd_amp["rc"] == 0 and cmd_tests_ran)))
    verdict = "REPRODUCED" if ok else "NOT_REPRODUCED"
    observation = (
        f"green rc={r_green['rc']}, breaking rc={r_break['rc']} on the same consumer; "
        f"naive gate rc={naive['rc']} while the suite failed (false green); "
        f"strict gate rc={strict['rc']} blocks; "
        f"cmd '& exit /b 0' rc={cmd_amp['rc']} with tests actually run={cmd_tests_ran}."
    )
    return base_record(
        "E2E-12", verdict, "f4-e2e (scratch copy)", [
            "git worktree add --detach <scratch>/worktrees/green <green>",
            "git worktree add --detach <scratch>/worktrees/breaking <breaking>",
            f"{PY} -m unittest discover -q  (cwd=green worktree)",
            f"{PY} -m unittest discover -q  (cwd=breaking worktree)",
            f"{PY} tools/naive_gate.py <breaking worktree>",
            f"{PY} tools/strict_gate.py <breaking worktree>",
            "cmd /c python -m unittest discover -q & exit /b 0",
        ], observation,
        "single Python fixture, unittest only; demonstrates that ignoring a return code yields "
        "a false green and that a return-code gate blocks. Not a general claim about every CI.",
        evidence)


def test_cache_13() -> dict:
    fx = build_f7("f7-context")
    repo = fx["repo"]
    write(repo / ".cache" / "tool.bin", b"ignored cache v1")
    r1 = run([PY, "tools/build.py"], cwd=repo)
    j1 = json.loads(r1["stdout"])
    write(repo / ".cache" / "tool.bin", b"ignored cache v2")
    r2 = run([PY, "tools/build.py"], cwd=repo)
    j2 = json.loads(r2["stdout"])
    status_before_flags = status_porcelain(repo)
    write(repo / "inputs" / "flags.txt", "-O2\n")
    r3 = run([PY, "tools/build.py"], cwd=repo)
    j3 = json.loads(r3["stdout"])
    status_after_flags = status_porcelain(repo)
    ignored = status_porcelain(repo, ignored=True)

    body = (
        "CACHE-13 — an undeclared input changes the artifact but not context_id\n"
        f"fixture: f7-context (scratch variant; base={fx['base'][:10]})\n"
        "the build declares inputs/flags.txt, inputs/lock.txt and two env vars;\n"
        "it also reads .cache/tool.bin (git-ignored, absent from the manifest).\n\n"
        + log_block("run 1: tool.bin v1", r1)
        + log_block("run 2: tool.bin v2 (same declared inputs)", r2)
        + log_block("run 3: declared flags changed -O1 -> -O2", r3)
        + "git status --porcelain (tracked state, before flags change):\n"
        + ("\n".join(status_before_flags) or "(empty)") + "\n"
        + "git status --porcelain (after flags change):\n"
        + ("\n".join(status_after_flags) or "(empty)") + "\n"
        + "git status --porcelain --ignored (tail):\n"
        + "\n".join(ignored[-10:]) + "\n"
    )
    structured = {
        "fixture": "f7-context",
        "context_id_v1": j1["context_id"],
        "context_id_v2": j2["context_id"],
        "context_id_flags_changed": j3["context_id"],
        "artifact_sha_v1": j1["artifact_sha256"],
        "artifact_sha_v2": j2["artifact_sha256"],
        "artifact_sha_flags_changed": j3["artifact_sha256"],
        "context_unchanged_after_undeclared_change": j1["context_id"] == j2["context_id"],
        "artifact_changed_after_undeclared_change": j1["artifact_sha256"] != j2["artifact_sha256"],
        "context_changed_after_declared_change": j3["context_id"] != j1["context_id"],
        "tree_before": tree_oid(repo),
        "status_before_flags": status_before_flags,
        "status_after_flags": status_after_flags,
    }
    evidence = write_log("CACHE-13", body, structured)
    ok = (j1["context_id"] == j2["context_id"]
          and j1["artifact_sha256"] != j2["artifact_sha256"]
          and j3["context_id"] != j1["context_id"])
    verdict = "REPRODUCED" if ok else "NOT_REPRODUCED"
    observation = (
        f"context_id unchanged ({j1['context_id'][:12]}...) when .cache/tool.bin changes, "
        f"while artifact sha changes {j1['artifact_sha256'][:12]} -> {j2['artifact_sha256'][:12]}; "
        f"a declared change (flags) does change context_id ({j3['context_id'][:12]}...). "
        "Same context_id, different artifact = cache poisoning class."
    )
    return base_record(
        "CACHE-13", verdict, "f7-context (scratch copy)", [
            f"{PY} tools/build.py  (tool.bin v1)",
            f"{PY} tools/build.py  (tool.bin v2, declared inputs unchanged)",
            f"{PY} tools/build.py  (inputs/flags.txt -O1 -> -O2)",
            "git status --porcelain / --ignored",
        ], observation,
        "one Python build script; the scratch variant intentionally reads the git-ignored "
        ".cache/tool.bin while its context_id stays declared-only. The unmodified r0a f7 "
        "build script does not consume that cache, so the original fixture alone cannot "
        "demonstrate the poisoning: this scratch deviation is the test object, not a change "
        "to r0a. Demonstrates key insufficiency, not a specific Flow cache.",
        evidence)


def test_context_03() -> dict:
    fx = build_f7("f7-context-completeness")
    repo = fx["repo"]
    runs = {}
    runs["no_cache_no_hidden"] = run([PY, "tools/build.py"], cwd=repo)
    write(repo / ".cache" / "tool.bin", b"cache-appears")
    runs["cache_appears"] = run([PY, "tools/build.py"], cwd=repo)
    env = git_env({"FLOW_HIDDEN_INPUT": "hidden-variant"})
    runs["hidden_env"] = run([PY, "tools/build.py"], cwd=repo, env=env)

    parsed = {k: json.loads(v["stdout"]) for k, v in runs.items()}
    declared_keys = sorted(json.loads((repo / "out" / "context.json").read_text())["manifest"].keys())

    checker = (
        "naive completeness checker: declared manifest keys are a subset of the inputs the "
        "script names in code -> PASS. It cannot observe reads it does not know about, so it "
        "returns PASS while the artifact changes."
    )

    body = (
        "CONTEXT-03 — validation scope cannot be proven complete from inside the tool\n"
        f"fixture: f7-context-completeness (scratch variant; base={fx['base'][:10]})\n"
        "declared manifest keys: " + ", ".join(declared_keys) + "\n\n"
        + log_block("run A: no cache file, no hidden env", runs["no_cache_no_hidden"])
        + log_block("run B: .cache/tool.bin appears (undeclared)", runs["cache_appears"])
        + log_block("run C: FLOW_HIDDEN_INPUT set (undeclared env)", runs["hidden_env"])
        + checker + "\n"
    )
    structured = {
        "fixture": "f7-context-completeness",
        "declared_keys": declared_keys,
        "context_ids": {k: v["context_id"] for k, v in parsed.items()},
        "artifact_sha256": {k: v["artifact_sha256"] for k, v in parsed.items()},
        "context_same_across_all_three": len({v["context_id"] for v in parsed.values()}) == 1,
        "artifacts_distinct": len({v["artifact_sha256"] for v in parsed.values()}) == 3,
        "tree": tree_oid(repo),
        "completeness_checker": "PASS (cannot observe undeclared reads)",
    }
    evidence = write_log("CONTEXT-03", body, structured)
    ok = (len({v["context_id"] for v in parsed.values()}) == 1
          and len({v["artifact_sha256"] for v in parsed.values()}) == 3)
    verdict = "REPRODUCED" if ok else "NOT_REPRODUCED"
    observation = (
        "three runs share one context_id but produce three distinct artifacts because two "
        "inputs (ignored file, undeclared env) are read outside the declared set; no in-tool "
        "signal can prove the declared set is complete, so the validation scope is "
        "indeterminable without an enforced boundary."
    )
    return base_record(
        "CONTEXT-03", verdict, "f7-context-completeness (scratch copy)", [
            f"{PY} tools/build.py  (baseline)",
            f"{PY} tools/build.py  (after creating .cache/tool.bin)",
            f"{PY} tools/build.py  (FLOW_HIDDEN_INPUT=hidden-variant)",
        ], observation,
        "structural demonstration on one script: absence of evidence of extra inputs is not "
        "evidence of completeness. The scratch variant reads the ignored file and an "
        "undeclared env var on purpose (the unmodified r0a build does not); not a theorem "
        "about every build tool.",
        evidence)


def test_legacy_06() -> dict:
    repo = SCRATCH / "legacy06"
    init_repo(repo)
    write(repo / "api.py", E2E_API)
    write(repo / "consumer.py", E2E_CONSUMER)
    write(repo / "test_e2e.py", E2E_TEST)
    base = commit_all(repo, "fixture: green base")
    git(repo, "checkout", "-q", "-b", "semantic-break", base, check=True)
    write(repo / "api.py",
          "def surface(width: int, height: int) -> int:\n"
          "    return width * height\n")
    broken = commit_all(repo, "fixture: semantic break, area renamed to surface")
    patch = git(repo, "diff", "main", "semantic-break", "--", "api.py", check=True)["stdout"]
    patch_path = ARTIFACTS / "legacy06-textual-break.patch"
    write(patch_path, patch)
    git(repo, "checkout", "-q", "main", check=True)
    before = status_porcelain(repo)

    check = run(["git", "-C", str(repo), "apply", "--check", str(patch_path)], cwd=repo)
    apply = run(["git", "-C", str(repo), "apply", str(patch_path)], cwd=repo)
    # in-memory syntax check: no .pyc, no worktree side effect
    compile_res = run(
        [PY, "-c",
         "import pathlib; compile(pathlib.Path('api.py').read_text(encoding='utf-8'), 'api.py', 'exec')"],
        cwd=repo)
    tests = run([PY, "-m", "unittest", "discover", "-q"], cwd=repo)
    diffstat = git(repo, "diff", "--stat", check=False)
    after = status_porcelain(repo)

    textual_gate_would_promote = check["rc"] == 0 and apply["rc"] == 0 and compile_res["rc"] == 0
    body = (
        "LEGACY-06 — textual applicability != semantic validation\n"
        f"fixture: legacy06 (base={base[:10]}, semantic-break={broken[:10]})\n"
        "patch only touches api.py; consumer.py is unchanged and still imports 'area'.\n\n"
        + log_block("git apply --check (textual applicability)", check)
        + log_block("git apply (applies textually)", apply)
        + log_block("in-memory compile() of api.py (syntactic plausibility)", compile_res)
        + log_block("python -m unittest discover -q (semantic check)", tests)
        + log_block("git diff --stat after apply", diffstat)
        + "status before apply:\n" + ("\n".join(before) or "(clean)") + "\n"
        + "status after apply:\n" + ("\n".join(after) or "(clean)") + "\n"
        + f"textual gate would promote (apply_ok and compile_ok) = {textual_gate_would_promote}\n"
    )
    structured = {
        "fixture": "legacy06",
        "base": base,
        "semantic_break": broken,
        "patch": "logs/artifacts/legacy06-textual-break.patch",
        "apply_check_rc": check["rc"],
        "apply_rc": apply["rc"],
        "syntax_check_rc": compile_res["rc"],
        "tests_rc": tests["rc"],
        "textual_gate_would_promote": textual_gate_would_promote,
        "status_before": before,
        "status_after": after,
    }
    evidence = write_log("LEGACY-06", body, structured)
    ok = check["rc"] == 0 and apply["rc"] == 0 and compile_res["rc"] == 0 and tests["rc"] != 0
    verdict = "REPRODUCED" if ok else "NOT_REPRODUCED"
    observation = (
        f"git apply --check rc={check['rc']}, git apply rc={apply['rc']}, in-memory "
        f"compile() rc={compile_res['rc']}, tests rc={tests['rc']}: the patch is textually "
        "applicable and syntactically plausible, yet the unchanged consumer breaks. A "
        "textual AUTO_SAFE label would promote it."
    )
    return base_record(
        "LEGACY-06", verdict, "legacy06 (scratch repo)", [
            "git diff main semantic-break -- api.py > patch",
            "git apply --check patch",
            "git apply patch",
            f"{PY} -c \"compile(open('api.py').read(), 'api.py', 'exec')\"",
            f"{PY} -m unittest discover -q",
        ], observation,
        "single rename patch; demonstrates the mapping AUTO_SAFE/TEXTUALLY_APPLICABLE != "
        "VALIDATED_FOR_SCOPE. It does not enumerate every semantic break class.",
        evidence)


def test_recovery_16() -> dict:
    fx = build_f5()
    repo = fx["repo"]
    pub = run([PY, "publisher.py", str(repo), "none", "r0b-payload"], cwd=repo)
    journal = repo / ".git" / "flow-journal.jsonl"
    intact = journal.read_bytes()
    records_intact = [json.loads(line) for line in intact.decode().splitlines()]

    read_intact_robust = run([PY, str(TOOLS / "reader_robust.py"), str(repo)], cwd=repo)

    # partial write: cut the last line in the middle, no trailing newline
    cut = max(1, len(intact) - 12)
    truncated = intact[:cut]
    journal.write_bytes(truncated)

    strict = run([PY, str(TOOLS / "reader_naive_strict.py"), str(repo)], cwd=repo)
    skip = run([PY, str(TOOLS / "reader_naive_skip.py"), str(repo)], cwd=repo)
    robust = run([PY, str(TOOLS / "reader_robust.py"), str(repo)], cwd=repo)

    # control: a valid intent-only journal (one complete line) must be recoverable from the ref
    intent_line = intact.split(b"\n")[0] + b"\n"
    journal.write_bytes(intent_line)
    robust_intent_only = run([PY, str(TOOLS / "reader_robust.py"), str(repo)], cwd=repo)
    ref = branch_oid(repo, "main")

    body = (
        "RECOVERY-16 — partial journal write: naive readers vs fail-closed reader\n"
        f"fixture: f5-crash (scratch copy; base={fx['base'][:10]})\n"
        f"publisher rc={pub['rc']}; journal records={len(records_intact)}; "
        f"truncated at byte {cut} of {len(intact)} (mid final line, no newline).\n\n"
        + log_block("publisher normal run", pub)
        + log_block("robust reader on intact journal", read_intact_robust)
        + log_block("naive strict reader on truncated journal", strict)
        + log_block("naive skip reader on truncated journal", skip)
        + log_block("robust reader on truncated journal", robust)
        + log_block("robust reader on valid intent-only journal (ref already published)",
                    robust_intent_only)
        + f"ref after publisher = {ref}\n"
    )
    structured = {
        "fixture": "f5-crash",
        "publisher_rc": pub["rc"],
        "journal_records": len(records_intact),
        "intact_bytes": len(intact),
        "truncated_bytes": len(truncated),
        "intact_robust": {"rc": read_intact_robust["rc"],
                          "stdout": read_intact_robust["stdout"].strip()},
        "truncated_naive_strict": {"rc": strict["rc"],
                                   "stderr_tail": strict["stderr"].strip().splitlines()[-4:]},
        "truncated_naive_skip": {"rc": skip["rc"],
                                 "stdout": skip["stdout"].strip()},
        "truncated_robust": {"rc": robust["rc"], "stdout": robust["stdout"].strip()},
        "intent_only_robust": {"rc": robust_intent_only["rc"],
                               "stdout": robust_intent_only["stdout"].strip()},
        "ref": ref,
    }
    evidence = write_log("RECOVERY-16", body, structured)
    naive_crashed = strict["rc"] != 0 and "Traceback" in strict["stderr"]
    naive_silent = skip["rc"] == 0 and '"would_republish": true' in skip["stdout"]
    robust_refused = robust["rc"] != 0 and "REFUSED_CORRUPT_JOURNAL" in robust["stdout"]
    intent_only_ok = robust_intent_only["rc"] == 0 and "RECONCILED_PUBLISHED_FROM_REF" in robust_intent_only["stdout"]
    ok = naive_crashed and naive_silent and robust_refused and intent_only_ok
    verdict = "REPRODUCED" if ok else "NOT_REPRODUCED"
    observation = (
        f"truncated final line: naive strict reader rc={strict['rc']} (uncaught JSON error), "
        f"naive skip reader rc={skip['rc']} with would_republish=true (silent recovery), "
        f"robust reader rc={robust['rc']} REFUSED_CORRUPT_JOURNAL; intact journal rc="
        f"{read_intact_robust['rc']} COMPLETE; valid intent-only journal rc="
        f"{robust_intent_only['rc']} RECONCILED_PUBLISHED_FROM_REF."
    )
    return base_record(
        "RECOVERY-16", verdict, "f5-crash (scratch copy)", [
            f"{PY} publisher.py <repo> none r0b-payload",
            "truncate .git/flow-journal.jsonl mid final line (no trailing newline)",
            f"{PY} tools/reader_naive_strict.py <repo>",
            f"{PY} tools/reader_naive_skip.py <repo>",
            f"{PY} tools/reader_robust.py <repo>",
            "restore a valid intent-only journal and re-read with the robust reader",
        ], observation,
        "fixture-local JSONL journal. Shows the required reader behaviour (refuse on partial "
        "line) and the two naive failure modes; not a crash-consistency proof of any Flow store.",
        evidence)


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------


def environment_record() -> dict:
    git_ver = run(["git", "--version"], cwd=ROOT)
    return {
        "generated_at": now_iso(),
        "os": sys.platform,
        "platform": platform.platform(),
        "python": sys.version.split()[0],
        "python_executable": "<PYTHON>",
        "git": git_ver["stdout"].strip(),
        "cpu_count": os.cpu_count(),
        "cwd_redacted": "<REPO>/experiments/r0b/verification",
        "repo_head": "<read-only>",
        "network_used": False,
        "compiler_used": False,
        "fixture_provenance": FIXTURE_PROVENANCE,
    }


def main() -> int:
    prepare_layout()
    write_tools()
    (LOGS / "environment.json").write_text(
        json.dumps(environment_record(), indent=2, sort_keys=True) + "\n", encoding="utf-8")

    tests = [test_e2e_12, test_cache_13, test_context_03, test_legacy_06, test_recovery_16]
    results = []
    for test in tests:
        try:
            results.append(test())
        except Exception as exc:  # a harness failure must be visible, not papered over
            test_id = test.__name__.replace("test_", "").replace("_", "-").upper()
            results.append(base_record(
                test_id, "BLOCKED", "unknown", [],
                f"harness exception: {type(exc).__name__}: {exc}",
                "test did not run to a verdict", []))

    (ROOT / "RESULTS.json").write_text(
        json.dumps(results, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    print(f"{'ID':14s} {'VERDICT':16s} FIXTURE")
    for rec in results:
        print(f"{rec['id']:14s} {rec['verdict']:16s} {rec['fixture']}")
    incomplete = [r for r in results if r["verdict"] in {"BLOCKED", "NOT_RUN"}]
    print(json.dumps({"results": str(ROOT / 'RESULTS.json'), "tests": len(results),
                      "incomplete": [r["id"] for r in incomplete]}))
    return 1 if incomplete else 0


if __name__ == "__main__":
    sys.exit(main())
