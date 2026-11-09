#!/usr/bin/env python3
"""Adversarial QA harness for Ravum Flow MVP 0.1 (independent reviewer).

One entry point, deterministic, stdlib only:

    python attacks.py --flow D:\\App\\Ravum-flow-wt\\qa\\target\\debug\\flow.exe \
                      --out experiments/mvp-qa/results.json

Every scenario:

* creates a **disposable** Git repository under the system temp directory,
* runs the **real binary** (`--flow`) as a child process,
* asserts the *observable* contract (JSON status/code, exit code, and the
  filesystem / Git state before vs after),
* records REPRODUCED (attack succeeded = defect), REFUTED (the product refused
  or the attack did not happen) or NOT_RUN (with an explicit reason).

Exit codes: 0 = no reproduced P0; 1 = at least one reproduced P0; 3 = harness
error or a scenario that could not be evaluated. `NOT_RUN` is never a pass.

Verdict semantics for this harness
----------------------------------
REPRODUCED  the attack achieved its goal (a false PASSED/PUBLISHED/RECONCILED,
            a byte change in a user file, a write outside the workspace, an
            unjustified ref mutation, or an overclaimed guarantee).
REFUTED     the product refused loudly, or the invariant held.
NOT_RUN     the scenario could not be executed on this host; the reason is
            recorded verbatim. Never counted as evidence of safety.

New commands (`propose`, `proposals`, `assemble`, `run --candidate`,
`publish --candidate`, `--dry-run`, `approve`) do not exist at the branch this
harness was built against: the FUT-* scenarios are declared in
FUTURE_SCENARIOS and report NOT_RUN with that reason. Enable them later by
implementing their `fn` in that one list.

No network, no credential, no real project: fictional canary strings only.
"""

import argparse
import ctypes
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import traceback
from dataclasses import dataclass, field
from pathlib import Path

# --------------------------------------------------------------------------
# Constants
# --------------------------------------------------------------------------

CANARY = "CANARY-FAKE-SECRET-001"
PY = sys.executable
IS_WINDOWS = sys.platform == "win32"
DEFAULT_RUN_TIMEOUT_S = 60
MAX_SCENARIO_BYTES = 200 * 1024 * 1024  # < 200 MB per scenario (budget)


# --------------------------------------------------------------------------
# Small utilities
# --------------------------------------------------------------------------

def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def rust_json_canonical(value) -> str:
    """Reproduce `Json::to_string` (compact, insertion order, UTF-8 literal).

    Verified against a real receipt's `integrity` digest before the forgery
    scenario is allowed to draw a conclusion; if the serialisers ever diverge
    the scenario reports NOT_RUN instead of a fabricated result.
    """
    if value is None:
        return "null"
    if value is True:
        return "true"
    if value is False:
        return "false"
    if isinstance(value, (int, float)):
        if isinstance(value, float) and value.is_integer() and abs(value) < 9.0e15:
            return str(int(value))
        if isinstance(value, float):
            text = repr(value)
            return text[:-2] if text.endswith(".0") else text
        return str(value)
    if isinstance(value, str):
        out = ['"']
        for ch in value:
            code = ord(ch)
            if ch == '"':
                out.append('\\"')
            elif ch == "\\":
                out.append("\\\\")
            elif ch == "\n":
                out.append("\\n")
            elif ch == "\r":
                out.append("\\r")
            elif ch == "\t":
                out.append("\\t")
            elif code < 0x20:
                out.append("\\u%04x" % code)
            else:
                out.append(ch)
        out.append('"')
        return "".join(out)
    if isinstance(value, list):
        return "[" + ",".join(rust_json_canonical(v) for v in value) + "]"
    if isinstance(value, dict):
        return "{" + ",".join(
            rust_json_canonical(k) + ":" + rust_json_canonical(v)
            for k, v in value.items()
        ) + "}"
    raise TypeError(f"unsupported JSON value {type(value)!r}")


def seal_receipt(value: dict) -> dict:
    body = {k: v for k, v in value.items() if k != "integrity"}
    sealed = dict(body)
    sealed["integrity"] = sha256_bytes(rust_json_canonical(body).encode("utf-8"))
    return sealed


def safe_remove_tree(path: Path) -> None:
    """Remove a fixture tree without ever following a link out of the run root.

    Symlinks and junctions are unlinked; files are made writable; failures are
    ignored (the directory lives under the system temp directory).
    """
    if not path.exists():
        return
    for entry in sorted(path.rglob("*"), key=lambda p: len(p.parts), reverse=True):
        try:
            if entry.is_symlink():
                entry.unlink()
        except OSError:
            pass
    def onerror(func, target, exc):  # noqa: ANN001
        try:
            os.chmod(target, 0o700)
            func(target)
        except OSError:
            pass
    shutil.rmtree(path, onerror=onerror)


# --------------------------------------------------------------------------
# Process execution (bounded, with an optional Windows peak-working-set sample)
# --------------------------------------------------------------------------

if IS_WINDOWS:
    try:
        _kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
        _psapi = ctypes.WinDLL("psapi", use_last_error=True)

        class _PROCESS_MEMORY_COUNTERS(ctypes.Structure):
            _fields_ = [
                ("cb", ctypes.c_uint32),
                ("PageFaultCount", ctypes.c_uint32),
                ("PeakWorkingSetSize", ctypes.c_size_t),
                ("WorkingSetSize", ctypes.c_size_t),
                ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
                ("QuotaPagedPoolUsage", ctypes.c_size_t),
                ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
                ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
                ("PagefileUsage", ctypes.c_size_t),
                ("PeakPagefileUsage", ctypes.c_size_t),
            ]

        _MEM_AVAILABLE = True
    except Exception:  # pragma: no cover - non-Windows or restricted
        _MEM_AVAILABLE = False
else:
    _MEM_AVAILABLE = False


class _PeakMemorySampler(threading.Thread):
    """Samples the flow process working set; never raises into the harness."""

    def __init__(self, pid: int) -> None:
        super().__init__(daemon=True)
        self.pid = pid
        self.peak = 0
        self._stop = threading.Event()

    def run(self) -> None:
        if not _MEM_AVAILABLE:
            return
        handle = _kernel32.OpenProcess(0x0410, False, self.pid)  # QUERY_INFORMATION|VM_READ
        if not handle:
            handle = _kernel32.OpenProcess(0x1000, False, self.pid)  # LIMITED_INFORMATION
        if not handle:
            return
        try:
            while not self._stop.is_set():
                counters = _PROCESS_MEMORY_COUNTERS()
                counters.cb = ctypes.sizeof(counters)
                if _psapi.GetProcessMemoryInfo(handle, ctypes.byref(counters), counters.cb):
                    self.peak = max(self.peak, int(counters.WorkingSetSize))
                self._stop.wait(0.02)
        except Exception:
            pass
        finally:
            _kernel32.CloseHandle(handle)

    def stop(self) -> int:
        self._stop.set()
        try:
            self.join(timeout=2)
        except Exception:
            pass
        return self.peak


@dataclass
class Res:
    argv: list
    cwd: str
    env: dict
    exit: int
    stdout: str
    stderr: str
    duration_ms: int
    timed_out: bool = False
    peak_ws: int = 0

    @property
    def json(self):
        try:
            value = json.loads(self.stdout)
        except Exception:
            return None
        return value if isinstance(value, dict) else None

    @property
    def status(self):
        return (self.json or {}).get("status")

    @property
    def code(self):
        j = self.json or {}
        return j.get("code") or (j.get("status") if j.get("status") in
                                 {"ERROR", "UNKNOWN", "PARTIAL", "INVALID_POLICY"} else None)

    @property
    def reason(self):
        j = self.json or {}
        return j.get("reason") or j.get("error") or ""

    def brief(self) -> dict:
        j = self.json or {}
        out = {"exit": self.exit}
        for key in ("status", "code", "reason", "error"):
            if j.get(key) is not None:
                out[key] = j.get(key)
        if self.timed_out:
            out["timed_out"] = True
        if self.peak_ws:
            out["peak_ws_mb"] = round(self.peak_ws / (1024 * 1024), 1)
        return out

    def print_argv(self) -> str:
        return " ".join(self.argv)


def run_process(argv, cwd, env, timeout_s=DEFAULT_RUN_TIMEOUT_S) -> Res:
    started = time.monotonic()
    proc = subprocess.Popen(
        argv, cwd=str(cwd), env=env,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        stdin=subprocess.DEVNULL,
    )
    sampler = _PeakMemorySampler(proc.pid) if _MEM_AVAILABLE else None
    if sampler:
        sampler.start()
    timed_out = False
    try:
        out_b, err_b = proc.communicate(timeout=timeout_s)
    except subprocess.TimeoutExpired:
        timed_out = True
        proc.kill()
        try:
            out_b, err_b = proc.communicate(timeout=10)
        except subprocess.TimeoutExpired:
            out_b, err_b = b"", b""
    duration_ms = int((time.monotonic() - started) * 1000)
    peak = sampler.stop() if sampler else 0
    return Res(
        argv=[str(a) for a in argv],
        cwd=str(cwd),
        env=dict(env),
        exit=proc.returncode if proc.returncode is not None else -1,
        stdout=out_b.decode("utf-8", "replace"),
        stderr=err_b.decode("utf-8", "replace"),
        duration_ms=duration_ms,
        timed_out=timed_out,
        peak_ws=peak,
    )


# --------------------------------------------------------------------------
# Snapshot of user files / Git state
# --------------------------------------------------------------------------

@dataclass
class Snapshot:
    files: dict = field(default_factory=dict)      # rel -> (sha256, is_writable)
    status: str = ""
    index: str = ""
    refs: str = ""
    head: str = ""
    journal_sha: str = ""
    journal_len: int = 0

    def changed_user_files(self, other: "Snapshot") -> list:
        keys = set(self.files) | set(other.files)
        diffs = []
        for key in sorted(keys):
            if self.files.get(key) != other.files.get(key):
                diffs.append(key)
        return diffs


def take_snapshot(root: Path) -> Snapshot:
    files = {}
    for path in sorted(root.rglob("*")):
        rel = path.relative_to(root).as_posix()
        if rel == ".git" or rel.startswith(".git/"):
            continue
        if rel == ".flow" or rel.startswith(".flow/"):
            continue
        try:
            if path.is_symlink():
                files[rel] = ("symlink:" + os.readlink(path), True)
            elif path.is_file():
                writable = bool(path.stat().st_mode & 0o200)
                files[rel] = (sha256_file(path), writable)
        except OSError:
            files[rel] = ("<unreadable>", False)
    journal = root / ".flow" / "journal.jsonl"
    journal_sha, journal_len = "", 0
    try:
        if journal.exists():
            data = journal.read_bytes()
            journal_sha, journal_len = sha256_bytes(data), len(data)
    except OSError:
        pass
    git = lambda *args: git_capture(root, *args)  # noqa: E731
    return Snapshot(
        files=files,
        status=git("status", "--porcelain=v1", "-uall"),
        index=git("ls-files", "-s"),
        refs=git("for-each-ref", "--format=%(refname)=%(objectname)"),
        head=git("rev-parse", "HEAD"),
        journal_sha=journal_sha,
        journal_len=journal_len,
    )


def git_capture(root: Path, *args) -> str:
    try:
        proc = subprocess.run(
            ["git", *args], cwd=str(root), env=BASE_ENV,
            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
            stdin=subprocess.DEVNULL, timeout=30,
        )
        return proc.stdout.decode("utf-8", "replace").strip()
    except Exception:
        return ""


# --------------------------------------------------------------------------
# Case: one disposable fixture and the helpers a scenario needs
# --------------------------------------------------------------------------

class NotRun(Exception):
    """Scenario cannot be executed on this host; the reason is recorded."""


class HarnessError(Exception):
    """The harness itself failed; this is never evidence about the product."""


class Failed(Exception):
    """A setup/assertion inside a scenario failed in a way that is a defect."""


BASE_ENV = dict(os.environ)
BASE_ENV.update({
    "GIT_AUTHOR_NAME": "Flow QA",
    "GIT_AUTHOR_EMAIL": "qa@example.invalid",
    "GIT_COMMITTER_NAME": "Flow QA",
    "GIT_COMMITTER_EMAIL": "qa@example.invalid",
    "GIT_CONFIG_NOSYSTEM": "1",
    "GIT_TERMINAL_PROMPT": "0",
})


class Case:
    """A scenario's disposable workspace and command recorder."""

    def __init__(self, scenario: "Scn", run_root: Path, flow: Path) -> None:
        self.scn = scenario
        self.root = run_root / scenario.id
        self.root.mkdir(parents=True, exist_ok=True)
        self.flow_bin = flow
        self.commands = []
        self.evidence = {}
        self.repo = None
        self.env = dict(BASE_ENV)
        home = self.root / "home"
        home.mkdir(exist_ok=True)
        self.env["HOME"] = str(home)
        self.env["USERPROFILE"] = str(home)
        self.env["TMPDIR"] = str(self.root)
        self.env["TEMP"] = str(self.root)
        self.env["TMP"] = str(self.root)

    # -- fixture -------------------------------------------------------
    def new_repo(self, name="repo", files=None, staged=False, branch="main") -> Path:
        repo = self.root / name
        repo.mkdir(parents=True, exist_ok=True)
        git = lambda *args: self.git(*args, cwd=repo)  # noqa: E731
        git("init", "-q", "-b", branch)
        git("config", "user.name", "Flow QA")
        git("config", "user.email", "qa@example.invalid")
        git("config", "core.autocrlf", "false")
        defaults = {"README.md": "hello\n", "data.txt": "v1\n"}
        if files is None:
            files = defaults
        for rel, body in files.items():
            self.write_file(rel, body, repo=repo)
        git("add", "-A")
        git("commit", "-q", "-m", "init")
        if staged:
            self.write_file("feature.txt", "feature\n", repo=repo)
            git("add", "feature.txt")
        self.repo = repo
        return repo

    def write_file(self, rel, body, repo=None) -> Path:
        repo = repo or self.repo
        path = repo / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(body, bytes):
            path.write_bytes(body)
        else:
            path.write_text(body, encoding="utf-8")
        return path

    def stage(self, rel, body=None, repo=None) -> Path:
        repo = repo or self.repo
        if body is not None:
            self.write_file(rel, body, repo=repo)
        self.git("add", rel, cwd=repo)
        return repo / rel

    def commit(self, message="commit", repo=None) -> None:
        repo = repo or self.repo
        self.git("add", "-A", cwd=repo)
        self.git("commit", "-q", "-m", message, cwd=repo)

    # -- flow / git ----------------------------------------------------
    def flow(self, *args, env=None, timeout=DEFAULT_RUN_TIMEOUT_S, repo=None) -> Res:
        repo = repo or self.repo
        child_env = dict(self.env)
        if env:
            child_env.update({k: str(v) for k, v in env.items()})
        res = run_process([str(self.flow_bin), *map(str, args)], repo, child_env, timeout)
        self.commands.append({
            "argv": display_command(res.argv, self.root),
            "exit": res.exit,
            "result": res.brief(),
        })
        return res

    def git(self, *args, cwd=None) -> Res:
        repo = cwd or self.repo
        res = run_process(["git", *map(str, args)], repo, self.env, 60)
        return res

    def git_out(self, *args, cwd=None) -> str:
        return self.git(*args, cwd=cwd).stdout.strip()

    # -- config --------------------------------------------------------
    def write_config(self, obligations, policy_version="FLOW-POLICY-1",
                     execution_boundary="local-trusted", disabled=False,
                     extras=None, indent=2, sort_keys=False) -> None:
        config = {
            "schema": "flow-config/1",
            "policy_version": policy_version,
            "execution_boundary": execution_boundary,
            "obligations": obligations,
            "disabled": disabled,
        }
        if extras:
            config.update(extras)
        flow_dir = self.repo / ".flow"
        flow_dir.mkdir(exist_ok=True)
        (flow_dir / "config.json").write_text(
            json.dumps(config, indent=indent, sort_keys=sort_keys) + "\n",
            encoding="utf-8",
        )

    def init_flow(self) -> Res:
        res = self.flow("init")
        if res.exit != 0:
            raise HarnessError(f"flow init failed: {res.brief()} {res.stderr[:300]}")
        return res

    def fresh(self, name, obligations, staged=True, **config_kwargs) -> Path:
        """Create a *new* named fixture repo and initialise Flow in it.

        Scenarios that need several independent repositories must use distinct
        names; `prepare` is the single-repository shorthand and refuses to reuse
        the `.flow` of an existing one.
        """
        repo = self.new_repo(name=name, staged=staged)
        self.init_flow()
        self.write_config(obligations, **config_kwargs)
        return repo

    def prepare(self, obligations, staged=True, **config_kwargs) -> Path:
        return self.fresh("repo", obligations, staged, **config_kwargs)

    # -- receipts / journal --------------------------------------------
    def receipt_files(self) -> list:
        receipts = self.repo / ".flow" / "receipts"
        if not receipts.exists():
            return []
        return sorted(receipts.glob("*.json"))

    def receipt_for_tree(self, tree: str) -> Path:
        return self.repo / ".flow" / "receipts" / f"{tree}.json"

    def read_receipt(self, path: Path) -> dict:
        return json.loads(path.read_text(encoding="utf-8"))

    def write_receipt(self, path: Path, value: dict) -> None:
        path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")

    def journal_path(self) -> Path:
        return self.repo / ".flow" / "journal.jsonl"

    def journal_records(self) -> list:
        path = self.journal_path()
        if not path.exists():
            return []
        records = []
        for line in path.read_text(encoding="utf-8").splitlines():
            if line.strip():
                try:
                    records.append(json.loads(line))
                except Exception:
                    records.append({"<unparsable>": line})
        return records

    def journal_bytes(self) -> bytes:
        path = self.journal_path()
        return path.read_bytes() if path.exists() else b""

    def write_journal(self, records) -> None:
        self.journal_path().parent.mkdir(exist_ok=True)
        with open(self.journal_path(), "w", encoding="utf-8", newline="\n") as fh:
            for record in records:
                fh.write(json.dumps(record) + "\n")

    # -- state ---------------------------------------------------------
    def snapshot(self) -> Snapshot:
        return take_snapshot(self.repo)

    def ref(self, name="refs/heads/main") -> str:
        return self.git_out("rev-parse", name)

    def tree_of(self, commit: str) -> str:
        return self.git_out("rev-parse", f"{commit}^{{tree}}")

    def commit_tree_of(self, tree: str, parent: str, message="external") -> str:
        return self.git_out("commit-tree", tree, "-p", parent, "-m", message)

    def user_files_changed(self, before: Snapshot) -> list:
        return before.changed_user_files(self.snapshot())

    def record(self, key, value) -> None:
        self.evidence[key] = value


def display_command(argv: list, root: Path) -> str:
    shown = []
    for arg in argv:
        text = str(arg)
        try:
            text = text.replace(str(root), "<tmp>")
        except Exception:
            pass
        if len(text) > 220:
            text = text[:217] + "..."
        shown.append(text)
    return " ".join(shown)


# --------------------------------------------------------------------------
# Obligation builders (everything runs in the scratch checkout)
# --------------------------------------------------------------------------

def ob_green(id="green", required=True, timeout_s=30) -> dict:
    return {"id": id, "program": PY, "args": ["-c", "import sys; sys.exit(0)"],
            "required": required, "timeout_s": timeout_s}


def ob_red(id="red", required=True, timeout_s=30) -> dict:
    return {"id": id, "program": PY, "args": ["-c", "import sys; sys.exit(3)"],
            "required": required, "timeout_s": timeout_s}


def ob_raise(id="boom", required=True, timeout_s=30) -> dict:
    return {"id": id, "program": PY,
            "args": ["-c", "raise RuntimeError('deliberate QA exception')"],
            "required": required, "timeout_s": timeout_s}


def ob_python(code, id="script", required=True, timeout_s=30) -> dict:
    return {"id": id, "program": PY, "args": ["-c", code],
            "required": required, "timeout_s": timeout_s}


def ob_sleep(seconds, id="slow", required=True, timeout_s=30) -> dict:
    return ob_python(f"import time; time.sleep({seconds})", id=id,
                     required=required, timeout_s=timeout_s)


# --------------------------------------------------------------------------
# Scenario declaration
# --------------------------------------------------------------------------

@dataclass
class Scn:
    id: str
    axis: str
    title: str
    severity: str          # severity if REPRODUCED
    fn: object
    requires: tuple = ()   # command names that must exist for the scenario

    def __post_init__(self):
        self.fn.scenario = self  # type: ignore[attr-defined]


def R(expected, observed, notes="", **evidence) -> dict:
    return {"verdict": "REPRODUCED", "expected": expected, "observed": observed,
            "notes": notes, "evidence": evidence}


def X(expected, observed, notes="", **evidence) -> dict:
    return {"verdict": "REFUTED", "expected": expected, "observed": observed,
            "notes": notes, "evidence": evidence}


def check(condition: bool, message: str) -> None:
    if not condition:
        raise HarnessError(message)


# ==========================================================================
# AXIS 1 — identity / scope of a receipt
# ==========================================================================

def idn_index_tree_change(c: Case):
    c.prepare([ob_green()], staged=True)
    res_run = c.flow("run")
    check(res_run.exit == 0, f"green run must pass: {res_run.brief()}")
    tree1 = c.git_out("write-tree")
    before_ref = c.ref()
    # Change the index tree after the proof.
    c.stage("extra.txt", "extra\n")
    res = c.flow("publish")
    after_ref = c.ref()
    if res.exit == 0 and res.status == "PUBLISHED" and after_ref != before_ref:
        return R("publish refuses; the receipt does not name the new index tree",
                 f"PUBLISHED {after_ref[:12]}", **{"run": res_run.brief(),
                                                  "publish": res.brief()})
    return X("publish refuses; the receipt does not name the new index tree",
             f"exit {res.exit} status={res.status} code={res.code} "
             f"ref_moved={after_ref != before_ref}",
             **{"run": res_run.brief(), "publish": res.brief(),
                "new_tree": c.git_out("write-tree"), "old_tree": tree1})


def idn_worktree_change(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    c.write_file("data.txt", "DIRTY-WORKTREE\n")
    res = c.flow("publish")
    worktree = (c.repo / "data.txt").read_bytes()
    if res.exit == 0 and res.status == "PUBLISHED":
        return X("publish is defined on the INDEX tree: a dirty worktree does not "
                 "change the verified tree",
                 "PUBLISHED; worktree bytes untouched",
                 notes="The product defines `run` as verifying the index tree "
                       "(usage + status hint), so this is not a stale receipt. "
                       "Recorded because the mandate calls a 'tree change' a "
                       "reason to invalidate; the verified object is the index "
                       "tree, not the worktree.",
                 **{"publish": res.brief(), "worktree_unchanged": worktree == b"DIRTY-WORKTREE\n"})
    return R("publish refuses after a worktree change",
             f"exit {res.exit} {res.status}", **{"publish": res.brief()})


def _policy_mutation(c: Case, mutate, label: str):
    c.prepare([ob_green(), ob_green(id="second")], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    before_ref = c.ref()
    mutate(c)
    res = c.flow("publish")
    after_ref = c.ref()
    moved = after_ref != before_ref
    if res.exit == 1 and res.status in ("REFUSED", "ERROR") and not moved:
        return X(f"publish refuses: {label}",
                 f"exit {res.exit} code={res.code}", **{"publish": res.brief(),
                                                        "ref_moved": moved})
    return R(f"publish refuses: {label}",
             f"exit {res.exit} status={res.status} ref_moved={moved}",
             **{"publish": res.brief(), "ref_moved": moved})


def idn_add_required(c: Case):
    return _policy_mutation(
        c, lambda case: case.write_config([ob_green(), ob_green(id="second"),
                                           ob_green(id="added")]),
        "a required obligation was added after the run")


def idn_change_required_args(c: Case):
    mutated = ob_green()
    mutated["args"] = ["-c", "import sys; sys.exit(0)  # changed"]
    return _policy_mutation(
        c, lambda case: case.write_config([mutated, ob_green(id="second")]),
        "a required obligation's args changed after the run")


def idn_remove_required(c: Case):
    return _policy_mutation(
        c, lambda case: case.write_config([ob_green()]),
        "a required obligation was removed after the run")


def idn_change_timeout(c: Case):
    mutated = ob_green()
    mutated["timeout_s"] = 1
    return _policy_mutation(
        c, lambda case: case.write_config([mutated, ob_green(id="second")]),
        "timeout_s changed 30 -> 1 after the run")


def idn_change_policy_version(c: Case):
    return _policy_mutation(
        c, lambda case: case.write_config([ob_green(), ob_green(id="second")],
                                          policy_version="FLOW-POLICY-2"),
        "policy_version changed after the run")


def idn_change_boundary(c: Case):
    return _policy_mutation(
        c, lambda case: case.write_config([ob_green(), ob_green(id="second")],
                                          execution_boundary="other-boundary"),
        "execution_boundary changed after the run")


def idn_timeout_override(c: Case):
    c.prepare([ob_green()], staged=True)
    run = c.flow("run", "--timeout", "1")
    check(run.exit == 0, f"green run with override must pass: {run.brief()}")
    before_ref = c.ref()
    res = c.flow("publish")
    if res.exit == 1 and not before_ref != c.ref():
        return X("publish refuses a run made under a `--timeout` override",
                 f"exit {res.exit} code={res.code}", **{"publish": res.brief()})
    return R("publish refuses a run made under a `--timeout` override",
             f"exit {res.exit} status={res.status}", **{"publish": res.brief()})


def idn_edited_receipt(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    receipt_path = c.receipt_for_tree(c.git_out("write-tree"))
    value = c.read_receipt(receipt_path)
    value["status"] = "FAILED"
    c.write_receipt(receipt_path, value)  # digest deliberately not updated
    res = c.flow("publish")
    if res.exit == 1 and res.status == "REFUSED" and res.code == "RECEIPT_INTEGRITY":
        return X("publish refuses an edited receipt with RECEIPT_INTEGRITY",
                 f"exit {res.exit} code={res.code}", **{"publish": res.brief()})
    return R("publish refuses an edited receipt with RECEIPT_INTEGRITY",
             f"exit {res.exit} code={res.code}", **{"publish": res.brief()})


def idn_semantic_config_edit(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    # Same effective policy, different bytes (indent + key order).
    c.write_config([ob_green()], indent=4, sort_keys=True)
    res = c.flow("publish")
    if res.exit == 0 and res.status == "PUBLISHED":
        return X("cosmetic config edits do not invalidate the proof (digest is over "
                 "the effective policy)", "PUBLISHED", **{"publish": res.brief()})
    return R("cosmetic config edits do not invalidate the proof",
             f"exit {res.exit} code={res.code}", **{"publish": res.brief()})


def idn_partial_changed_tree(c: Case):
    c.prepare([ob_green(), ob_green(id="second")], staged=False)
    c.stage("extra.txt", "extra\n")  # new tree without a proof
    new_tree = c.git_out("write-tree")
    res = c.flow("run", "--obligation", "green")
    if res.exit != 1 or res.status != "PARTIAL":
        return R("a selective run is PARTIAL and cannot publish",
                 f"exit {res.exit} status={res.status}", **{"run": res.brief()})
    pub = c.flow("publish")
    complete = c.receipt_for_tree(new_tree).exists()
    partial_files = [p.name for p in c.receipt_files() if ".partial." in p.name]
    if pub.exit == 1 and not complete and partial_files:
        return X("a selective run never occupies the complete receipt slot, so "
                 "publish finds no receipt",
                 f"exit {pub.exit} code={pub.code} complete_receipt={complete} "
                 f"partial={partial_files}", **{"publish": pub.brief()})
    return R("a selective run never occupies the complete receipt slot",
             f"exit {pub.exit} status={pub.status} complete_receipt={complete}",
             **{"publish": pub.brief()})


def idn_partial_after_complete(c: Case):
    c.prepare([ob_green(), ob_green(id="second")], staged=True)
    check(c.flow("run").exit == 0, "complete run must pass")
    partial = c.flow("run", "--obligation", "green")
    check(partial.exit == 1 and partial.status == "PARTIAL", "selective run is PARTIAL")
    res = c.flow("publish")
    return X("an earlier complete proof of the same tree remains valid after a "
             "diagnostic partial run",
             f"publish exit {res.exit} status={res.status}",
             notes="The partial receipt is written to a separate slot; it neither "
                   "replaces nor revokes the earlier complete proof of the same "
                   "tree and policy. Recorded as an observation, not a bypass: the "
                   "published receipt is the complete one.",
             **{"publish": res.brief(), "partial": partial.brief()})


def idn_duplicate_selection(c: Case):
    c.prepare([ob_green()], staged=True)
    res = c.flow("run", "--obligation", "green", "--obligation", "green")
    if res.exit == 1 and res.code == "DUPLICATE_OBLIGATION_ID":
        return X("a duplicated --obligation id is refused",
                 f"exit {res.exit} code={res.code}", **{"run": res.brief()})
    return R("a duplicated --obligation id is refused",
             f"exit {res.exit} status={res.status} code={res.code}",
             **{"run": res.brief()})


def idn_unknown_selection(c: Case):
    c.prepare([ob_green()], staged=True)
    res = c.flow("run", "--obligation", "does-not-exist")
    if res.exit == 1 and res.code == "UNKNOWN_OBLIGATION_ID":
        return X("an unknown --obligation id is refused",
                 f"exit {res.exit} code={res.code}", **{"run": res.brief()})
    return R("an unknown --obligation id is refused",
             f"exit {res.exit} status={res.status} code={res.code}",
             **{"run": res.brief()})


def idn_schema_edit(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    receipt_path = c.receipt_for_tree(c.git_out("write-tree"))
    value = c.read_receipt(receipt_path)
    value["schema"] = "flow-receipt/1"
    c.write_receipt(receipt_path, value)
    res = c.flow("publish")
    if res.exit == 1 and res.code == "RECEIPT_SCHEMA_STALE":
        return X("an outdated receipt schema is refused before integrity is trusted",
                 f"exit {res.exit} code={res.code}", **{"publish": res.brief()})
    return R("an outdated receipt schema is refused",
             f"exit {res.exit} code={res.code}", **{"publish": res.brief()})


def idn_copied_receipt(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    first = c.receipt_for_tree(c.git_out("write-tree"))
    c.stage("second.txt", "second\n")
    other_tree = c.git_out("write-tree")
    shutil.copyfile(first, c.receipt_for_tree(other_tree))
    res = c.flow("publish")
    if res.exit == 1 and res.code == "TREE_MISMATCH":
        return X("a receipt copied under another tree's filename is refused",
                 f"exit {res.exit} code={res.code}", **{"publish": res.brief()})
    return R("a receipt copied under another tree's filename is refused",
             f"exit {res.exit} code={res.code}", **{"publish": res.brief()})


def idn_integrity_missing(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    receipt_path = c.receipt_for_tree(c.git_out("write-tree"))
    value = c.read_receipt(receipt_path)
    del value["integrity"]
    c.write_receipt(receipt_path, value)
    res = c.flow("publish")
    if res.exit == 1 and res.code == "RECEIPT_INTEGRITY":
        return X("a receipt without an integrity digest is refused",
                 f"exit {res.exit} code={res.code}", **{"publish": res.brief()})
    return R("a receipt without an integrity digest is refused",
             f"exit {res.exit} code={res.code}", **{"publish": res.brief()})


def idn_reordered_policy(c: Case):
    return _policy_mutation(
        c, lambda case: case.write_config([ob_green(id="second"), ob_green()]),
        "obligation order changed after the run")


def idn_required_flag_flip(c: Case):
    second = ob_green(id="second")
    second["required"] = False
    return _policy_mutation(
        c, lambda case: case.write_config([ob_green(), second]),
        "an obligation's `required` flag flipped true -> false")


# ==========================================================================
# AXIS 2 — scope / portee: what may never become a green proof
# ==========================================================================

def scp_empty_policy_run(c: Case):
    c.prepare([], staged=True)
    res = c.flow("run")
    if res.exit == 1 and res.status == "UNKNOWN":
        return X("an empty policy yields UNKNOWN, never PASSED",
                 f"exit {res.exit} status={res.status}", **{"run": res.brief()})
    return R("an empty policy yields UNKNOWN, never PASSED",
             f"exit {res.exit} status={res.status}", **{"run": res.brief()})


def scp_empty_policy_publish(c: Case):
    c.prepare([], staged=True)
    res = c.flow("publish")
    if res.exit == 1 and res.status == "REFUSED" and res.code == "INVALID_POLICY":
        return X("an empty policy is not publishable",
                 f"exit {res.exit} code={res.code}", **{"publish": res.brief()})
    return R("an empty policy is not publishable",
             f"exit {res.exit} code={res.code}", **{"publish": res.brief()})


def scp_only_optional(c: Case):
    c.prepare([ob_green(required=False)], staged=True)
    res = c.flow("run")
    if res.exit == 1 and res.code == "INVALID_POLICY":
        return X("a policy with no required obligation is refused",
                 f"exit {res.exit} code={res.code}", **{"run": res.brief()})
    return R("a policy with no required obligation is refused",
             f"exit {res.exit} status={res.status} code={res.code}",
             **{"run": res.brief()})


def scp_red_required(c: Case):
    c.prepare([ob_green(), ob_red()], staged=True)
    run = c.flow("run")
    pub = c.flow("publish")
    run_ok = run.exit == 1 and run.status == "FAILED"
    pub_ok = pub.exit == 1 and pub.status == "REFUSED" and pub.code in (
        "REQUIRED_NOT_PASSED", "STATUS_NOT_PASSED")
    if run_ok and pub_ok:
        return X("a red required obligation is FAILED and never publishable",
                 f"run={run.status} publish={pub.code}", **{"run": run.brief(),
                                                            "publish": pub.brief()})
    return R("a red required obligation is FAILED and never publishable",
             f"run exit={run.exit} status={run.status}; publish exit={pub.exit} "
             f"code={pub.code}", **{"run": run.brief(), "publish": pub.brief()})


def scp_python_exception(c: Case):
    c.prepare([ob_green(), ob_raise()], staged=True)
    run = c.flow("run")
    pub = c.flow("publish")
    ok = (run.exit == 1 and run.status == "FAILED" and pub.exit == 1
          and pub.status == "REFUSED")
    return (X("a Python exception makes the run FAILED and unpublishable",
              f"run={run.status} publish={pub.status}/{pub.code}",
              **{"run": run.brief(), "publish": pub.brief()}) if ok else
            R("a Python exception makes the run FAILED and unpublishable",
              f"run exit={run.exit} status={run.status}; publish exit={pub.exit} "
              f"status={pub.status}", **{"run": run.brief(), "publish": pub.brief()}))


def scp_e2e_red(c: Case):
    c.prepare([ob_green(), ob_red(id="e2e")], staged=True)
    run = c.flow("run")
    pub = c.flow("publish")
    ok = (run.exit == 1 and run.status == "FAILED" and pub.exit == 1
          and pub.status == "REFUSED")
    return (X("a red required E2E obligation is never green, never publishable",
              f"run={run.status} publish={pub.status}/{pub.code}",
              **{"run": run.brief(), "publish": pub.brief()}) if ok else
            R("a red required E2E obligation is never green",
              f"run exit={run.exit} status={run.status}; publish exit={pub.exit} "
              f"status={pub.status}", **{"run": run.brief(), "publish": pub.brief()}))


def scp_partial_missing_required(c: Case):
    c.prepare([ob_green(), ob_red(id="e2e")], staged=True)
    run = c.flow("run", "--obligation", "green")
    pub = c.flow("publish")
    if run.exit == 1 and run.status == "PARTIAL" and pub.exit == 1:
        return X("a partial run that skips a required obligation is PARTIAL and "
                 "publishable never", f"run={run.status} publish={pub.code}",
                 **{"run": run.brief(), "publish": pub.brief()})
    return R("a partial run that skips a required obligation is never publishable",
             f"run exit={run.exit} status={run.status}; publish exit={pub.exit} "
             f"status={pub.status}", **{"run": run.brief(), "publish": pub.brief()})


def scp_optional_red_does_not_mask(c: Case):
    c.prepare([ob_green(), ob_red(id="optional-red", required=False)], staged=True)
    run = c.flow("run")
    pub = c.flow("publish")
    if run.exit == 0 and pub.exit == 0 and pub.status == "PUBLISHED":
        return X("an optional red obligation does not mask a green required set",
                 "PASSED + PUBLISHED", **{"run": run.brief(), "publish": pub.brief()})
    return R("an optional red obligation does not mask a green required set",
             f"run={run.brief()} publish={pub.brief()}",
             **{"run": run.brief(), "publish": pub.brief()})


def scp_missing_tool(c: Case):
    missing = {"id": "missing", "program": "flow-qa-no-such-program-xyz",
               "args": [], "required": True, "timeout_s": 10}
    c.prepare([missing], staged=True)
    run = c.flow("run")
    pub = c.flow("publish")
    if run.exit == 1 and run.status == "UNSUPPORTED" and pub.exit == 1:
        return X("a required obligation that cannot start is UNSUPPORTED and "
                 "unpublishable", f"run={run.status} publish={pub.code}",
                 **{"run": run.brief(), "publish": pub.brief()})
    return R("a required obligation that cannot start is UNSUPPORTED and "
             "unpublishable", f"run={run.brief()} publish={pub.brief()}",
             **{"run": run.brief(), "publish": pub.brief()})


def scp_timeout_required(c: Case):
    c.prepare([ob_sleep(30, timeout_s=1)], staged=True)
    run = c.flow("run")
    pub = c.flow("publish")
    if run.exit == 1 and run.status == "TIMEOUT" and pub.exit == 1:
        return X("a timed-out required obligation is TIMEOUT and unpublishable",
                 f"run={run.status} publish={pub.code}",
                 **{"run": run.brief(), "publish": pub.brief()})
    return R("a timed-out required obligation is TIMEOUT and unpublishable",
             f"run={run.brief()} publish={pub.brief()}",
             **{"run": run.brief(), "publish": pub.brief()})


def scp_disabled(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("disable").exit == 0, "disable must succeed")
    run = c.flow("run")
    pub = c.flow("publish")
    enabled = c.flow("enable")
    if (run.exit == 1 and run.code == "DISABLED" and pub.exit == 1
            and pub.code == "DISABLED" and enabled.exit == 0):
        return X("disabled repositories refuse run and publish",
                 f"run={run.code} publish={pub.code}",
                 **{"run": run.brief(), "publish": pub.brief(),
                    "enable": enabled.brief()})
    return R("disabled repositories refuse run and publish",
             f"run={run.brief()} publish={pub.brief()}",
             **{"run": run.brief(), "publish": pub.brief()})


# ==========================================================================
# AXIS 3 — Git refs, CAS and names
# ==========================================================================

def git_stale_expected_old(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    res = c.flow("publish", "--expected-old", "0" * 40)
    ref = c.ref()
    if res.exit == 1 and res.code == "STALE_REF":
        return X("a stale --expected-old is refused with STALE_REF",
                 f"exit {res.exit} code={res.code}",
                 **{"publish": res.brief(), "ref": ref})
    return R("a stale --expected-old is refused with STALE_REF",
             f"exit {res.exit} status={res.status} code={res.code}",
             **{"publish": res.brief()})


def git_external_move(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    old = c.ref()
    external = c.commit_tree_of(c.tree_of("HEAD"), old, "external writer")
    c.git("update-ref", "refs/heads/main", external, old)
    journal_before = c.journal_bytes()
    res = c.flow("publish", "--expected-old", old)
    after = c.ref()
    journal_after = c.journal_bytes()
    if res.exit == 1 and res.code == "STALE_REF" and after == external \
            and journal_before == journal_after:
        return X("an external `git update-ref` between run and publish is not "
                 "overwritten: STALE_REF, ref kept, no journal intent",
                 f"exit {res.exit} code={res.code} ref_is_external={after == external}",
                 **{"publish": res.brief(), "external": external,
                    "journal_unchanged": journal_before == journal_after})
    return R("an external ref move between run and publish must not be "
             "overwritten or silently replayed",
             f"exit {res.exit} code={res.code} ref_moved={after != external} "
             f"journal_changed={journal_before != journal_after}",
             **{"publish": res.brief()})


def git_ref_head(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    res = c.flow("publish", "--ref", "HEAD")
    if res.exit == 1 and res.code == "REF_NOT_ALLOWED":
        return X("`--ref HEAD` is refused", f"exit {res.exit} code={res.code}",
                 **{"publish": res.brief()})
    return R("`--ref HEAD` is refused", f"exit {res.exit} code={res.code}",
             **{"publish": res.brief()})


def git_ref_tag(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    c.git("tag", "v1")
    res = c.flow("publish", "--ref", "refs/tags/v1")
    if res.exit == 1 and res.code == "REF_NOT_ALLOWED":
        return X("`--ref refs/tags/x` is refused", f"exit {res.exit} code={res.code}",
                 **{"publish": res.brief()})
    return R("`--ref refs/tags/x` is refused", f"exit {res.exit} code={res.code}",
             **{"publish": res.brief()})


def git_invalid_ref_names(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    refs = ["refs/heads/a..b", "refs/heads/", "refs/heads/x^{}",
            "refs/heads/.hidden", "refs/heads/a/.b", "refs/heads/a b"]
    rows, failures, mutations = [], [], []
    before_refs = c.git_out("for-each-ref", "--format=%(refname)=%(objectname)")
    for ref in refs:
        valid = c.git("check-ref-format", ref).exit == 0
        res = c.flow("publish", "--ref", ref)
        rows.append({"ref": ref, "check_ref_format_ok": valid,
                     "exit": res.exit, "status": res.status, "code": res.code})
        if not (res.exit == 1 and res.status in ("REFUSED", "ERROR")):
            failures.append(ref)
    after_refs = c.git_out("for-each-ref", "--format=%(refname)=%(objectname)")
    if after_refs != before_refs:
        mutations.append("refs changed")
    evidence = {"attempts": rows, "refs_unchanged": after_refs == before_refs}
    if not failures and not mutations:
        return X("every invalid/non-branch ref name is refused before any "
                 "mutation (codes vary: REF_NOT_ALLOWED before resolution, "
                 "UNKNOWN_REF after rev-parse)",
                 f"{len(rows)} names refused, refs unchanged", **evidence)
    return R("invalid ref names must be refused before any mutation",
             f"accepted={failures} mutations={mutations}", **evidence)


def git_missing_ref(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    res = c.flow("publish", "--ref", "refs/heads/does-not-exist")
    if res.exit == 1 and res.code == "UNKNOWN_REF":
        return X("a missing destination ref is UNKNOWN_REF",
                 f"exit {res.exit} code={res.code}", **{"publish": res.brief()})
    return R("a missing destination ref is UNKNOWN_REF",
             f"exit {res.exit} code={res.code}", **{"publish": res.brief()})


def git_bogus_expected_old(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    res = c.flow("publish", "--expected-old", "not-an-object-id")
    if res.exit == 1 and res.code == "STALE_REF":
        return X("a non-OID --expected-old cannot reach `git update-ref`: STALE_REF",
                 f"exit {res.exit} code={res.code}", **{"publish": res.brief()})
    return R("a non-OID --expected-old is refused",
             f"exit {res.exit} code={res.code}", **{"publish": res.brief()})


def git_detached_head(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    check(c.git("checkout", "-q", "--detach", "HEAD").exit == 0, "detach")
    no_ref = c.flow("publish")
    explicit = c.flow("publish", "--ref", "refs/heads/main")
    ok = (no_ref.exit == 1 and no_ref.status == "ERROR"
          and "detached" in no_ref.reason.lower()
          and explicit.exit == 0 and explicit.status == "PUBLISHED")
    return (X("a detached HEAD is refused unless `--ref` names the branch; an "
              "explicit branch still publishes",
              f"no_ref={no_ref.status}/{no_ref.reason[:40]!r} explicit={explicit.status}",
              **{"no_ref": no_ref.brief(), "explicit": explicit.brief()}) if ok else
            R("a detached HEAD is refused unless `--ref` names the branch",
              f"no_ref={no_ref.brief()} explicit={explicit.brief()}",
              **{"no_ref": no_ref.brief(), "explicit": explicit.brief()}))


def git_symref_branch(c: Case):
    c.prepare([ob_green()], staged=True)
    c.git("branch", "target")
    c.git("symbolic-ref", "refs/heads/sym", "refs/heads/target")
    check(c.flow("run").exit == 0, "green run must pass")
    res = c.flow("publish", "--ref", "refs/heads/sym")
    target = c.ref("refs/heads/target")
    symref = c.git_out("symbolic-ref", "refs/heads/sym")
    tree = c.tree_of(target) if target else ""
    if res.exit == 0 and res.status == "PUBLISHED" and res.json.get("ref") == "refs/heads/sym" \
            and symref == "refs/heads/target":
        return R("a symbolic ref is refused, or the tool states that the target "
                 "moved",
                 f"PUBLISHED under the name refs/heads/sym while the real target "
                 f"refs/heads/target moved to {target[:12]} (tree "
                 f"{tree[:12]} = verified tree); symref unchanged",
                 notes="Documented residual in the hardening report §8. The "
                       "commit still carries the verified tree, but the operator "
                       "read `ref: refs/heads/sym` and a different ref moved.",
                 **{"publish": res.brief(), "target": target,
                    "symref_still": symref})
    return X("a symbolic ref is refused", f"exit {res.exit} code={res.code}",
             **{"publish": res.brief()})


def git_symref_tag(c: Case):
    c.prepare([ob_green()], staged=True)
    c.git("tag", "v1")
    tag_before = c.ref("refs/tags/v1")
    c.git("symbolic-ref", "refs/heads/sym", "refs/tags/v1")
    check(c.flow("run").exit == 0, "green run must pass")
    res = c.flow("publish", "--ref", "refs/heads/sym")
    tag_after = c.ref("refs/tags/v1")
    if res.exit == 0 and res.status == "PUBLISHED" and tag_after != tag_before:
        return R("publish must move a branch only: a symref must not make it "
                 "move a tag",
                 f"PUBLISHED reported ref=refs/heads/sym; refs/tags/v1 moved "
                 f"{tag_before[:12]} -> {tag_after[:12]}",
                 notes="The usage text says publish is branch-only and `--ref "
                       "refs/tags/...` is refused, but a symbolic ref named under "
                       "refs/heads/ dereferences to the tag and git update-ref "
                       "moves it. Signed/release tags are not protected.",
                 **{"publish": res.brief(), "tag_before": tag_before,
                    "tag_after": tag_after})
    return X("a symref to a tag does not move the tag",
             f"exit {res.exit} code={res.code} tag_moved={tag_after != tag_before}",
             **{"publish": res.brief()})


def git_blocking_reftx_hook(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    hook = c.repo / ".git" / "hooks" / "reference-transaction"
    hook.parent.mkdir(parents=True, exist_ok=True)
    hook.write_text("#!/bin/sh\n[ \"$1\" = prepared ] && exit 1\nexit 0\n", encoding="utf-8")
    before_ref = c.ref()
    before_journal = c.journal_bytes()
    res = c.flow("publish")
    after_ref = c.ref()
    records = c.journal_records()
    intent = [r for r in records if r.get("event") == "intent" and r.get("id")
              and r.get("ref") == "refs/heads/main"]
    outcome = [r for r in records if r.get("event") == "outcome" and r.get("status")
               in ("REFUSED", "REFUSED_STALE")]
    new_intent = len(intent) > 0 and before_journal != c.journal_bytes()
    if res.exit == 1 and res.code in ("UPDATE_REF_REFUSED", "REF_MOVED") \
            and after_ref == before_ref and outcome:
        return X("a blocking reference-transaction hook aborts publish with a "
                 "typed refusal, ref untouched, intent closed as refused",
                 f"exit {res.exit} code={res.code} ref_unchanged=True "
                 f"journal_outcome={outcome[-1].get('status')}",
                 **{"publish": res.brief(), "journal": records})
    return R("a blocking reference-transaction hook must abort publish without "
             "moving the ref", f"exit {res.exit} code={res.code} "
             f"ref_changed={after_ref != before_ref} outcome={bool(outcome)}",
             **{"publish": res.brief(), "journal": records})


def git_already_published(c: Case):
    c.prepare([ob_green()], staged=False)  # index tree == HEAD tree
    check(c.flow("run").exit == 0, "green run must pass")
    before = c.ref()
    res = c.flow("publish")
    if res.exit == 1 and res.code == "ALREADY_PUBLISHED" and c.ref() == before:
        return X("publishing a tree the ref already carries is refused, no empty "
                 "commit", f"exit {res.exit} code={res.code}", **{"publish": res.brief()})
    return R("publishing an already-carried tree is refused",
             f"exit {res.exit} code={res.code}", **{"publish": res.brief()})


# ==========================================================================
# AXIS 4 — durable state: journal, crash recovery, reconciliation
# ==========================================================================

def _crash_after_intent(c: Case) -> Res:
    return c.flow("publish", env={"FLOW_TEST_CRASH": "after-intent"})


def dur_truncated_journal(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    crash = _crash_after_intent(c)
    check(crash.exit == 89, f"FLOW_TEST_CRASH=after-intent must abort: {crash.brief()}")
    ref_before = c.ref()
    path = c.journal_path()
    data = path.read_bytes()
    path.write_bytes(data[:-9])  # torn last line
    torn = path.read_bytes()
    rec = c.flow("reconcile")
    status = c.flow("status")
    preserved = path.read_bytes() == torn
    if (rec.exit == 1 and rec.status == "CORRUPT_JOURNAL"
            and rec.json.get("journal_preserved") is True and preserved
            and c.ref() == ref_before and status.exit == 1):
        return X("a truncated journal is CORRUPT_JOURNAL (exit 1), preserved, and "
                 "status refuses to guess",
                 f"reconcile={rec.status}/{rec.code} status_exit={status.exit} "
                 f"journal_preserved={preserved}",
                 **{"reconcile": rec.brief(), "status": status.brief()})
    return R("a truncated journal must be CORRUPT_JOURNAL, not RECONCILED",
             f"reconcile exit={rec.exit} status={rec.status} preserved={preserved} "
             f"status_exit={status.exit}",
             **{"reconcile": rec.brief(), "status": status.brief()})


def dur_duplicate_intent_id(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    _crash_after_intent(c)
    records = c.journal_records()
    intent = records[-1]
    outcome = {"schema": "flow-journal/1", "event": "outcome", "id": intent["id"],
               "status": "PUBLISHED", "commit": intent["new"],
               "at": "2026-01-01T00:00:00Z"}
    c.write_journal([intent, dict(intent), outcome])
    before = c.journal_bytes()
    ref_before = c.ref()
    res = c.flow("reconcile")
    unchanged = c.journal_bytes() == before
    if res.exit == 1 and res.status == "NEEDS_RECONCILIATION" and unchanged \
            and c.ref() == ref_before:
        return X("a duplicated intent id cannot be closed by one outcome: "
                 "NEEDS_RECONCILIATION (exit 1), journal and ref untouched",
                 f"exit {res.exit} status={res.status} journal_unchanged={unchanged}",
                 **{"reconcile": res.brief(), "journal": c.journal_records()})
    return R("a duplicated intent id must not be reported as reconciled",
             f"exit {res.exit} status={res.status} journal_unchanged={unchanged}",
             **{"reconcile": res.brief(), "journal": c.journal_records()})


def dur_one_outcome_two_intents_moved(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    _crash_after_intent(c)
    records = c.journal_records()
    intent = records[-1]
    # The ref did move (one publication), but two intents claim the same id.
    c.git("update-ref", "refs/heads/main", intent["new"], intent["old"])
    outcome = {"schema": "flow-journal/1", "event": "outcome", "id": intent["id"],
               "status": "PUBLISHED", "commit": intent["new"],
               "at": "2026-01-01T00:00:00Z"}
    c.write_journal([intent, dict(intent), outcome])
    res = c.flow("reconcile")
    if res.exit == 1 and res.status == "NEEDS_RECONCILIATION":
        return X("one outcome cannot close two intents even when the ref matches: "
                 "NEEDS_RECONCILIATION (exit 1)",
                 f"exit {res.exit} status={res.status}", **{"reconcile": res.brief()})
    return R("one outcome closing two intents must not be RECONCILED",
             f"exit {res.exit} status={res.status}", **{"reconcile": res.brief()})


def dur_crash_after_intent(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    ref_before = c.ref()
    crash = _crash_after_intent(c)
    ref_after_crash = c.ref()
    rec = c.flow("reconcile")
    outcomes_after_reconcile = [r for r in c.journal_records()
                                if r.get("event") == "outcome"]
    rec2 = c.flow("reconcile")
    pub = c.flow("publish")
    records = c.journal_records()
    outcomes = [r for r in records if r.get("event") == "outcome"]
    intent_pending = rec.json.get("resolved") or []
    if (crash.exit == 89 and ref_after_crash == ref_before
            and rec.exit == 0 and rec.status == "RECONCILED"
            and any(r.get("resolution") == "NOT_PUBLISHED" for r in intent_pending)
            and len(outcomes_after_reconcile) == 1 and rec2.exit == 0
            and pub.exit == 0 and pub.status == "PUBLISHED"
            and len(outcomes) == 2):
        return X("crash after intent: no ref move, reconcile records NOT_PUBLISHED "
                 "once, the next publish publishes exactly once",
                 f"crash_exit=89 reconcile={rec.status} outcomes={len(outcomes)} "
                 f"publish={pub.status}", **{"crash": crash.brief(),
                                             "reconcile": rec.brief(),
                                             "reconcile2": rec2.brief(),
                                             "publish": pub.brief()})
    return R("crash after intent must reconcile honestly and never double-publish",
             f"crash={crash.exit} ref_moved={ref_after_crash != ref_before} "
             f"reconcile={rec.brief()} outcomes={len(outcomes)} publish={pub.brief()}",
             **{"reconcile": rec.brief(), "publish": pub.brief()})


def dur_crash_after_ref(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    ref_before = c.ref()
    crash = c.flow("publish", env={"FLOW_TEST_CRASH": "after-ref"})
    ref_after = c.ref()
    rec = c.flow("reconcile")
    commit_count = int(c.git_out("rev-list", "--count", "HEAD") or 0)
    again = c.flow("publish")
    commit_count2 = int(c.git_out("rev-list", "--count", "HEAD") or 0)
    if (crash.exit == 89 and ref_after != ref_before and rec.exit == 0
            and rec.status == "RECONCILED"
            and any(r.get("resolution") == "PUBLISHED" for r in rec.json.get("resolved") or [])
            and again.exit == 1 and again.code == "ALREADY_PUBLISHED"
            and commit_count2 == commit_count):
        return X("crash after CAS: reconcile observes PUBLISHED, a second publish "
                 "is ALREADY_PUBLISHED and no commit is added",
                 f"crash_exit=89 reconcile={rec.status} second={again.code} "
                 f"commits={commit_count}", **{"crash": crash.brief(),
                                               "reconcile": rec.brief(),
                                               "second_publish": again.brief()})
    return R("crash after CAS must reconcile to PUBLISHED and never publish twice",
             f"crash={crash.exit} reconcile={rec.brief()} second={again.brief()} "
             f"commits={commit_count}->{commit_count2}",
             **{"reconcile": rec.brief(), "second_publish": again.brief()})


def dur_reconcile_idempotent(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    _crash_after_intent(c)
    first = c.flow("reconcile")
    lines1 = len([r for r in c.journal_records()])
    second = c.flow("reconcile")
    lines2 = len([r for r in c.journal_records()])
    if first.exit == 0 and second.exit == 0 and lines1 == lines2:
        return X("reconcile is idempotent: a second run appends nothing",
                 f"RECONCILED twice, journal lines {lines1} == {lines2}",
                 **{"first": first.brief(), "second": second.brief()})
    return R("reconcile must be idempotent",
             f"lines {lines1} -> {lines2}, exit {first.exit}/{second.exit}",
             **{"first": first.brief(), "second": second.brief()})


def dur_ref_moved_to_third(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    _crash_after_intent(c)
    third = c.commit_tree_of(c.tree_of("HEAD"), c.ref(), "third writer")
    c.git("update-ref", "refs/heads/main", third)
    before = c.journal_bytes()
    res = c.flow("reconcile")
    unchanged = c.journal_bytes() == before
    if res.exit == 1 and res.status == "NEEDS_RECONCILIATION" and unchanged \
            and c.ref() == third:
        return X("an observation matching neither old nor new is "
                 "NEEDS_RECONCILIATION (exit 1), no outcome invented",
                 f"exit {res.exit} status={res.status} journal_unchanged={unchanged}",
                 **{"reconcile": res.brief()})
    return R("an ambiguous observation must stay open",
             f"exit {res.exit} status={res.status} journal_unchanged={unchanged}",
             **{"reconcile": res.brief()})


def dur_journal_deleted(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    _crash_after_intent(c)
    c.journal_path().unlink()
    res = c.flow("reconcile")
    status = c.flow("status")
    if res.exit == 0 and res.status == "RECONCILED":
        return X("a deleted journal is indistinguishable from a fresh repository: "
                 "reconcile reports RECONCILED with nothing pending",
                 f"reconcile={res.status} pending_intents="
                 f"{status.json.get('pending_intents')}",
                 notes="Observation, P2: the durable record is the only witness; "
                       "deleting it (or losing it) removes the pending intent from "
                       "recovery. No ref was moved in this scenario, so no double "
                       "publication results; after a crash-after-ref the second "
                       "publish is ALREADY_PUBLISHED. Recorded as an honest "
                       "limitation, not a gate breach.",
                 **{"reconcile": res.brief(), "status": status.brief()})
    return R("a deleted journal must not be reported as a clean reconciliation",
             f"exit {res.exit} status={res.status}", **{"reconcile": res.brief()})


def dur_non_object_journal_line(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    path = c.journal_path()
    with open(path, "a", encoding="utf-8") as fh:
        fh.write("[]\n")
    rec = c.flow("reconcile")
    status = c.flow("status")
    if rec.exit == 0 and rec.status == "RECONCILED" and status.exit == 0:
        return X("a valid-JSON but non-object journal line is silently ignored: "
                 "reconcile says RECONCILED",
                 f"reconcile={rec.status} status_exit={status.exit}",
                 notes="Observation, P3: `read_journal` refuses lines that fail to "
                       "parse (torn writes), but a syntactically valid non-object "
                       "line (or an object without fields) is accepted and counted "
                       "as nothing. No invariant is broken with the records Flow "
                       "itself writes; recorded for completeness.",
                 **{"reconcile": rec.brief(), "status": status.brief()})
    return R("non-object journal lines should be refused as corrupt",
             f"exit {rec.exit} status={rec.status}", **{"reconcile": rec.brief()})


def dur_crash_before_intent(c: Case):
    raise NotRun("no injection point exists for a crash before the intent append: "
                 "FLOW_TEST_CRASH supports only after-intent and after-ref, and "
                 "killing the process between commit-tree and append-journal is "
                 "not deterministic from outside")


def dur_crafted_balance(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    _crash_after_intent(c)
    records = c.journal_records()
    intent = records[-1]
    outcome = {"schema": "flow-journal/1", "event": "outcome", "id": intent["id"],
               "status": "PUBLISHED", "commit": intent["new"],
               "at": "2026-01-01T00:00:00Z"}
    c.write_journal([intent, dict(intent), outcome, dict(outcome)])
    res = c.flow("reconcile")
    if res.exit == 0 and res.status == "RECONCILED":
        return X("a hand-crafted journal with two intents and two outcomes is "
                 "accepted as reconciled",
                 f"reconcile={res.status}",
                 notes="Observation, P3: the counting rule is intentionally "
                       "mechanical; a writer who can already rewrite the journal "
                       "can synthesize a balanced history. Not reachable through "
                       "Flow's own append path (ids carry a nanosecond suffix).",
                 **{"reconcile": res.brief()})
    return R("crafted duplicate intents must stay open",
             f"exit {res.exit} status={res.status}", **{"reconcile": res.brief()})


# ==========================================================================
# AXIS 5 — user files and hostile paths
# ==========================================================================

def fil_preserved_files(c: Case):
    c.new_repo()
    c.init_flow()
    c.write_config([ob_green()])
    c.stage("feature.txt", "feature\n")          # pre-existing index entry
    c.write_file("data.txt", "DIRTY-TRACKED\n")  # dirty tracked file
    c.write_file("untracked.bin", b"\x00\x01" + CANARY.encode() + b"\x02")
    c.write_file("readonly.txt", "readonly\n")
    c.git("add", "readonly.txt")
    c.git("commit", "-q", "-m", "add readonly")
    os.chmod(c.repo / "readonly.txt", 0o444)
    before = c.snapshot()
    run = c.flow("run")
    after_run = c.snapshot()
    pub = c.flow("publish")
    after_pub = c.snapshot()
    run_changes = before.changed_user_files(after_run)
    pub_changes = after_run.changed_user_files(after_pub)
    status_ok = before.status == after_run.status
    index_ok = before.index == after_run.index
    refs_ok = before.refs == after_run.refs
    if not run_changes and not pub_changes and status_ok and index_ok and refs_ok:
        return X("`run` modifies no user file, index, status or ref; `publish` "
                 "modifies no user file or index",
                 f"run={run.exit} publish={pub.exit} changed=[]",
                 **{"run": run.brief(), "publish": pub.brief(),
                    "files_changed_by_run": run_changes,
                    "files_changed_by_publish": pub_changes,
                    "run_status_unchanged": status_ok})
    return R("no byte of a dirty/untracked/read-only user file may change",
             f"run_changes={run_changes} pub_changes={pub_changes} "
             f"status_ok={status_ok} index_ok={index_ok} refs_ok={refs_ok}",
             **{"run": run.brief(), "publish": pub.brief()})


def fil_index_not_worktree(c: Case):
    c.new_repo()
    c.init_flow()
    c.write_config([ob_python(
        "import sys; sys.exit(0 if open('data.txt').read()=='v1\\n' else 9)",
        id="reads-index")])
    c.write_file("data.txt", "DIRTY-WORKTREE\n")
    before = c.snapshot()
    run = c.flow("run")
    after = c.snapshot()
    changed = before.changed_user_files(after)
    if run.exit == 0 and run.status == "PASSED" and not changed:
        return X("the obligation sees the INDEX content while the dirty worktree "
                 "is untouched", "PASSED on index content; worktree bytes unchanged",
                 **{"run": run.brief(), "files_changed": changed})
    return R("the obligation must see the verified index tree, not a dirty worktree",
             f"run={run.brief()} changed={changed}",
             **{"run": run.brief(), "files_changed": changed})


def fil_flow_state_not_in_tree(c: Case):
    c.new_repo(staged=False)
    c.init_flow()
    c.write_config([ob_green()])
    head_tree = c.git_out("rev-parse", "HEAD^{tree}")
    run = c.flow("run")
    if run.exit != 0:
        return R("`.flow` must stay out of the verified tree",
                 f"run failed {run.brief()}", **{"run": run.brief()})
    receipt_tree = run.json.get("tree_oid")
    entries = c.git_out("ls-tree", "-r", "--name-only", receipt_tree)
    if receipt_tree == head_tree and ".flow" not in entries.split("\n"):
        return X("Flow state is gitignored and never enters the verified tree",
                 f"receipt tree == HEAD tree {head_tree[:12]}; no .flow entry",
                 **{"run": run.brief()})
    return R("Flow state must not enter the verified tree",
             f"receipt_tree={receipt_tree[:12]} head_tree={head_tree[:12]} "
             f"has_flow={'.flow' in entries}", **{"run": run.brief()})


def fil_symlink_entry(c: Case):
    c.new_repo(staged=False)
    c.git("config", "core.symlinks", "false")
    proc = subprocess.run(["git", "hash-object", "-w", "--stdin"], cwd=str(c.repo),
                          env=c.env, input="target\n", text=True, capture_output=True)
    oid = proc.stdout.strip()
    c.git("update-index", "--add", "--cacheinfo", f"120000,{oid},link.txt")
    c.init_flow()
    c.write_config([ob_green()])
    before = c.snapshot()
    run = c.flow("run")
    changed = before.changed_user_files(c.snapshot())
    if run.exit == 1 and run.code == "UNSUPPORTED_CHECKOUT" and not changed:
        return X("a symlink tree entry checked out as a regular file is refused, "
                 "not certified", f"exit {run.exit} code={run.code}",
                 **{"run": run.brief()})
    return R("an unfaithful symlink checkout must be refused",
             f"exit {run.exit} code={run.code} changed={changed}",
             **{"run": run.brief(), "files_changed": changed})


def fil_case_collision(c: Case):
    c.new_repo(staged=False)
    proc = subprocess.run(["git", "hash-object", "-w", "--stdin"], cwd=str(c.repo),
                          env=c.env, input="same\n", text=True, capture_output=True)
    oid = proc.stdout.strip()
    for name in ("Case.txt", "case.txt"):
        res = c.git("update-index", "--add", "--cacheinfo", f"100644,{oid},{name}")
        check(res.exit == 0, f"cannot craft case collision: {res.stderr[:200]}")
    c.init_flow()
    c.write_config([ob_green()])
    run = c.flow("run")
    if run.exit == 1 and run.code == "UNSUPPORTED_CHECKOUT":
        return X("a case-colliding tree is refused (the host cannot represent it "
                 "faithfully)", f"exit {run.exit} code={run.code}",
                 **{"run": run.brief()})
    return R("a case-colliding tree must be refused",
             f"exit {run.exit} code={run.code}", **{"run": run.brief()})


def fil_reserved_and_dotdot(c: Case):
    c.new_repo(staged=False)
    c.init_flow()
    c.write_config([ob_green()])
    attempts = {}
    proc = subprocess.run(["git", "hash-object", "-w", "--stdin"], cwd=str(c.repo),
                          env=c.env, input="x\n", text=True, capture_output=True)
    blob = proc.stdout.strip()
    for name in ("CON", "AUX.txt", "x.", "../evil.txt"):
        res = c.git("update-index", "--add", "--cacheinfo", f"100644,{blob},{name}")
        attempts[name] = {"exit": res.exit, "stderr": res.stderr.strip()[:160]}
    # A raw tree object bypasses index validation; read-tree still refuses to
    # materialise those paths in an index, so the fixture cannot be built.
    raw_ok = 0
    for mode, name in (("100644", "CON"), ("100644", "AUX.txt"), ("100644", "x.")):
        import binascii
        data = mode.encode() + b" " + name.encode() + b"\0" + binascii.unhexlify(blob)
        p = subprocess.run(["git", "hash-object", "-t", "tree", "-w", "--stdin"],
                           cwd=str(c.repo), env=c.env, input=data, capture_output=True)
        if p.returncode == 0:
            oid = p.stdout.decode().strip()
            rt = c.git("read-tree", oid)
            attempts[name]["raw_tree"] = {"oid": oid[:12], "read_tree_exit": rt.exit,
                                          "read_tree_stderr": rt.stderr.strip()[:120]}
            if rt.exit == 0:
                raw_ok += 1
    if raw_ok == 0:
        raise NotRun(
            "Git itself refuses to place a Windows-reserved name or a `..` path "
            "in the index (update-index: Invalid path; read-tree refuses the raw "
            "tree), so no verified index tree can contain one. Fixture impossible "
            "on this host; paths are still validated in `.flow` state paths and "
            "obligation ids (see FIL-obligation-id). Attempts: " +
            json.dumps(attempts, ensure_ascii=False)[:600])
    raise NotRun("raw tree with reserved path was accepted by read-tree; rewrite "
                 "this scenario as an active attack")


def fil_obligation_id_traversal(c: Case):
    c.new_repo(staged=False)
    c.init_flow()
    c.write_config([{"id": "../escape", "program": PY,
                     "args": ["-c", "print(1)"], "required": True, "timeout_s": 5}])
    run = c.flow("run")
    pub = c.flow("publish")
    if run.exit == 1 and run.code == "INVALID_POLICY" and pub.exit == 1:
        return X("an obligation id with `..` is refused at policy validation",
                 f"run={run.code} publish={pub.code}",
                 **{"run": run.brief(), "publish": pub.brief()})
    return R("an unsafe obligation id must be refused",
             f"run={run.brief()} publish={pub.brief()}",
             **{"run": run.brief(), "publish": pub.brief()})


def fil_eol_materialisation(c: Case):
    c.new_repo(files={"payload.txt": "line1\nline2\n",
                      ".gitattributes": "*.txt text eol=crlf\n"})
    c.git("config", "core.autocrlf", "false")
    c.init_flow()
    c.write_config([ob_python(
        "import sys, pathlib; data = pathlib.Path('payload.txt').read_bytes();\n"
        "sys.exit(0 if data == b'line1\\r\\nline2\\r\\n' else 9)",
        id="wants-crlf")])
    c.write_file("feature.txt", "f\n")
    c.git("add", "feature.txt")
    run = c.flow("run")
    blob = c.git_out("cat-file", "-p", "HEAD:payload.txt")
    checkout = c.git_out("show", "HEAD:payload.txt")  # same conversion as checkout? no; use worktree
    if run.exit == 0 and run.status == "PASSED":
        return R("the obligation must see the bytes of the verified tree (the Git "
                 "blob), or the run must be refused as unfaithful",
                 f"PASSED: the obligation asserted CRLF and succeeded, while the "
                 f"tree blob for payload.txt is LF ({len(blob)} bytes: "
                 f"{blob!r}); the receipt names the tree whose blob was never "
                 f"tested. The checkout conversion is driven by a committed "
                 f".gitattributes (`*.txt text eol=crlf`).",
                 notes="P1: this is the content-fidelity counterpart of the "
                       "P0-1 type-fidelity fix. `verify_checkout_fidelity` checks "
                       "presence and file type, not that the bytes on disk equal "
                       "the blob. `doctor` warns that eol/filter rules can change "
                       "bytes (only when .gitattributes is present), so the "
                       "limitation is partly disclosed, but the headline claim "
                       "'the receipt names the exact Git tree that was checked' is "
                       "not literally true for converted files.",
                 **{"run": run.brief(), "blob": blob,
                    "gitattributes": (c.repo / ".gitattributes").read_text()})
    return X("the obligation sees the tree bytes or the run is refused",
             f"run={run.brief()} blob={blob!r}", **{"run": run.brief()})


def fil_state_journal_symlink(c: Case):
    c.new_repo()
    c.init_flow()
    c.write_config([ob_green()])
    c.stage("feature.txt", "feature\n")
    outside = c.root / "outside-victim.txt"
    outside.write_text("USER CONTENT\n", encoding="utf-8")
    journal = c.journal_path()
    journal.unlink()
    os.symlink(str(outside), journal)
    check(journal.is_symlink(), "fixture symlink was not created")
    before = outside.read_bytes()
    run = c.flow("run")
    pub = c.flow("publish")
    after = outside.read_bytes()
    if pub.exit == 0 and pub.status == "PUBLISHED" and after != before:
        return R("Flow must not follow a symlinked `.flow/journal.jsonl` to write "
                 "outside its workspace",
                 f"PUBLISHED appended {len(after) - len(before)} bytes of journal "
                 f"records to {outside.name!r} OUTSIDE the repository "
                 f"({len(before)} -> {len(after)} bytes, sha256 "
                 f"{sha256_bytes(before)[:12]} -> {sha256_bytes(after)[:12]})",
                 notes="P0: `.flow` state paths are used without checking that they "
                       "are regular files inside the workspace. An absolute "
                       "symlink planted in `.flow` (or shipped by a cloned "
                       "repository on a symlink-capable host, see FIL-clone-*) "
                       "turns `flow publish` into an arbitrary-file append. The "
                       "report's own boundary is violated: a user file outside the "
                       "repository changed.",
                 **{"run": run.brief(), "publish": pub.brief(),
                    "outside_before": sha256_bytes(before)[:16],
                    "outside_after": sha256_bytes(after)[:16],
                    "appended_bytes": len(after) - len(before)})
    return X("a symlinked journal is refused or does not write outside the repo",
             f"publish={pub.brief()} outside_changed={after != before}",
             **{"run": run.brief(), "publish": pub.brief()})


def fil_state_receipts_symlink(c: Case):
    c.new_repo()
    c.init_flow()
    c.write_config([ob_green()])
    receipts = c.repo / ".flow" / "receipts"
    shutil.rmtree(receipts)
    outside = c.root / "outside-receipts"
    outside.mkdir()
    os.symlink(str(outside), receipts)
    check(receipts.is_symlink(), "fixture symlink was not created")
    run = c.flow("run")
    written = sorted(p.name for p in outside.iterdir())
    if run.exit == 0 and written:
        return R("Flow must not follow a symlinked `.flow/receipts` to write "
                 "outside its workspace",
                 f"run exited 0 and wrote receipt {written} into "
                 f"{outside.name!r} OUTSIDE the repository",
                 notes="P0: same unchecked-state-path class as the journal symlink. "
                       "`save_receipt` creates the directory (following a link) and "
                       "renames a temp file into it. A repository can ship "
                       "`.flow/receipts` as a symlink (see FIL-clone-receipts), so "
                       "this is reachable from a cloned candidate, not only from a "
                       "local prank.",
                 **{"run": run.brief(), "outside_files": written})
    return X("a symlinked receipts directory is refused or confined",
             f"run={run.brief()} outside_files={written}", **{"run": run.brief()})


def fil_logs_symlink_within(c: Case):
    c.new_repo()
    c.init_flow()
    c.write_config([ob_green()])
    tree = c.git_out("write-tree")
    (c.repo / ".flow" / "logs").mkdir(exist_ok=True)
    link = c.repo / ".flow" / "logs" / tree
    target = c.repo / "src"
    target.mkdir(exist_ok=True)
    subprocess.run(["cmd", "/c", "mklink", "/J", str(link), str(target)],
                   capture_output=True) if IS_WINDOWS else os.symlink(str(target), link)
    if not link.exists():
        raise NotRun("could not create a directory link inside .flow/logs on this host")
    run = c.flow("run")
    written = sorted(p.name for p in target.iterdir())
    if run.exit == 0 and written:
        return R("logs must stay under `.flow`, not be redirected into the user's "
                 "source tree",
                 f"run wrote {written} into the junction target `src/`; only the "
                 f"repository boundary is checked, not the `.flow` boundary",
                 notes="P2: canonical_within(root) is enforced for the log "
                       "directory, so an outside-the-repository redirect is "
                       "refused, but a junction to another directory *inside* the "
                       "repository is accepted and user source files appear there. "
                       "Requires a pre-existing link; low impact, recorded for "
                       "completeness.",
                 **{"run": run.brief(), "written": written})
    return X("a logs redirect is confined to .flow", f"run={run.brief()}",
             **{"run": run.brief()})


def _ship_symlink_repo(c: Case, name: str, rel_link: str, target: str,
                       extra_files=None, config_obligations=None) -> tuple:
    src = c.new_repo(name, files=extra_files or {"README.md": "hello\n"}, staged=False)
    flow_dir = src / ".flow"
    flow_dir.mkdir()
    obligations = config_obligations or [ob_green()]
    (flow_dir / "config.json").write_text(json.dumps({
        "schema": "flow-config/1", "policy_version": "FLOW-POLICY-1",
        "execution_boundary": "local-trusted", "obligations": obligations,
        "disabled": False}, indent=2) + "\n", encoding="utf-8")
    ignore = ".flow/.gitignore"
    (flow_dir / ".gitignore").write_text(
        "!config.json\n!journal.jsonl\n!receipts\n", encoding="utf-8")
    os.symlink(target, src / rel_link)
    c.git("add", "-f", ".flow/config.json", ignore, rel_link)
    c.git("commit", "-q", "-m", "candidate ships .flow state")
    clone = c.root / f"{name}-clone"
    res = c.git("clone", "-q", "-c", "core.symlinks=true", str(src), str(clone),
                cwd=c.root)
    return src, clone, res


def fil_clone_receipts_symlink(c: Case):
    outside = c.root / "clone-receipts-out"
    outside.mkdir()
    src, clone, clone_res = _ship_symlink_repo(
        c, "candidate", ".flow/receipts", str(outside))
    link = clone / ".flow" / "receipts"
    if not link.is_symlink():
        raise NotRun("the clone did not materialise the symlink (host/git refuses "
                     f"symlinks); clone exit {clone_res.exit}")
    run = c.flow("run", repo=clone)
    written = sorted(p.name for p in outside.iterdir())
    if run.exit == 0 and written:
        return R("a cloned repository must not be able to redirect Flow's receipt "
                 "write outside the checkout",
                 f"clone materialised `.flow/receipts` as a symlink; `flow run` "
                 f"wrote {written} into {outside.name!r} outside the clone",
                 notes="P0: supply-chain variant of FIL-state-receipts. The link "
                       "is committed in the candidate tree; the victim only runs "
                       "the documented workflow (`flow run`). Reproduced on hosts "
                       "where Git materialises symlinks (Linux, macOS, Windows "
                       "with Developer Mode); the default Windows clone on this "
                       "host materialises a plain file and the fidelity check "
                       "refuses (FIL-clone-default).",
                 **{"run": run.brief(), "outside_files": written,
                    "clone_exit": clone_res.exit})
    return X("a cloned receipts symlink is refused or confined",
             f"run={run.brief()} outside={written}", **{"run": run.brief()})


def fil_clone_journal_symlink(c: Case):
    outside = c.root / "clone-journal-victim.txt"
    src, clone, clone_res = _ship_symlink_repo(
        c, "candidate", ".flow/journal.jsonl", str(outside))
    link = clone / ".flow" / "journal.jsonl"
    if not link.is_symlink():
        raise NotRun("the clone did not materialise the symlink; clone exit "
                     f"{clone_res.exit}")
    outside.write_text("VICTIM CONTENT\n", encoding="utf-8")
    c.stage("change.txt", "change\n", repo=clone)
    before = outside.read_bytes()
    run = c.flow("run", repo=clone)
    pub = c.flow("publish", repo=clone)
    after = outside.read_bytes()
    if pub.exit == 0 and pub.status == "PUBLISHED" and after != before:
        return R("a cloned repository must not be able to redirect Flow's journal "
                 "append outside the checkout",
                 f"clone materialised `.flow/journal.jsonl` as a symlink; "
                 f"`flow publish` appended {len(after) - len(before)} bytes of "
                 f"journal to a file OUTSIDE the clone ({len(before)} -> "
                 f"{len(after)} bytes)",
                 notes="P0: supply-chain variant of FIL-state-journal. A candidate "
                       "tree that tracks `.flow/journal.jsonl` as a symlink makes "
                       "the victim's ordinary `flow publish` append to an "
                       "attacker-chosen file. Absolute symlink targets survive "
                       "clone verbatim.",
                 **{"run": run.brief(), "publish": pub.brief(),
                    "outside_before": sha256_bytes(before)[:16],
                    "outside_after": sha256_bytes(after)[:16],
                    "clone_exit": clone_res.exit})
    return X("a cloned journal symlink is refused or confined",
             f"run={run.brief()} publish={pub.brief()} changed={after != before}",
             **{"run": run.brief(), "publish": pub.brief()})


def fil_clone_default_symlink_refused(c: Case):
    outside = c.root / "default-clone-victim.txt"
    outside.write_text("VICTIM\n", encoding="utf-8")
    src = c.new_repo("candidate", staged=False)
    flow_dir = src / ".flow"
    flow_dir.mkdir()
    (flow_dir / "config.json").write_text(json.dumps({
        "schema": "flow-config/1", "policy_version": "FLOW-POLICY-1",
        "execution_boundary": "local-trusted", "obligations": [ob_green()],
        "disabled": False}, indent=2) + "\n", encoding="utf-8")
    (flow_dir / ".gitignore").write_text("!config.json\n!journal.jsonl\n")
    os.symlink(str(outside), flow_dir / "journal.jsonl")
    c.git("add", "-f", ".flow/config.json", ".flow/.gitignore", ".flow/journal.jsonl")
    c.git("commit", "-q", "-m", "candidate")
    clone = c.root / "default-clone"
    clone_res = c.git("clone", "-q", str(src), str(clone), cwd=c.root)
    link = clone / ".flow" / "journal.jsonl"
    run = c.flow("run", repo=clone)
    if (not link.is_symlink()) and run.exit == 1 and run.code == "UNSUPPORTED_CHECKOUT":
        return X("on a host where Git cannot materialise the symlink, the fidelity "
                 "check refuses the tree instead of certifying it",
                 f"clone wrote a regular file; run refused {run.code}",
                 notes="This is the protection that exists today: on this host "
                       "`core.symlinks=false` by default, so a committed symlink "
                       "becomes a plain file and `verify_checkout_fidelity` "
                       "refuses. It does not help on symlink-capable hosts "
                       "(Linux/macOS/Developer Mode), see FIL-clone-receipts.",
                 **{"run": run.brief(), "clone_exit": clone_res.exit})
    return R("a cloned journal symlink must be refused on this host",
             f"clone symlink={link.is_symlink()} run={run.brief()}",
             **{"run": run.brief()})


def fil_clone_ships_config(c: Case):
    marker = c.root / "config-executed.txt"
    shipped = ob_python(f"open(r'{marker}','w').write('EXECUTED-FROM-CONFIG')",
                        id="shipped")
    src = c.new_repo("candidate", files={"README.md": "hello\n"}, staged=False)
    flow_dir = src / ".flow"
    flow_dir.mkdir()
    (flow_dir / "config.json").write_text(json.dumps({
        "schema": "flow-config/1", "policy_version": "FLOW-POLICY-1",
        "execution_boundary": "local-trusted", "obligations": [shipped],
        "disabled": False}, indent=2) + "\n", encoding="utf-8")
    (flow_dir / ".gitignore").write_text("!config.json\n")
    c.git("add", "-f", ".flow/config.json", ".flow/.gitignore")
    c.git("commit", "-q", "-m", "candidate ships config")
    clone = c.root / "clone"
    c.git("clone", "-q", str(src), str(clone), cwd=c.root)
    run = c.flow("run", repo=clone)
    if run.exit == 0 and marker.exists():
        return R("`flow run` must not execute a program supplied by the repository "
                 "without saying so",
                 f"the cloned repo's `.flow/config.json` was used by `flow run` "
                 f"and its obligation executed ({marker.name} written); run "
                 f"status={run.status}",
                 notes="P1: documented residual in the hardening report §8 ('a "
                       "repository can ship .flow/config.json and have it used "
                       "without flow init'), but the operational consequence - a "
                       "cloned candidate executes arbitrary commands under the "
                       "documented `flow run` workflow - is not surfaced at run "
                       "time and contradicts the intent of verifying an untrusted "
                       "candidate. `doctor` does not warn that the active policy "
                       "came from the repository.",
                 **{"run": run.brief(), "marker_written": True})
    return X("repository-supplied config does not execute silently",
             f"run={run.brief()} marker={marker.exists()}", **{"run": run.brief()})


# ==========================================================================
# AXIS 6 — process containment: output, timeouts, hooks, environment
# ==========================================================================

def prc_massive_output(c: Case):
    code = ("import sys\n"
            "sys.stdout.write('x' * (10 * 1024 * 1024)); sys.stdout.flush()\n"
            "sys.stderr.write('y' * (10 * 1024 * 1024)); sys.stderr.flush()\n")
    c.prepare([ob_python(code, id="flood", timeout_s=20)], staged=True)
    res = c.flow("run", timeout=60)
    logs = sorted((c.repo / ".flow" / "logs").rglob("*.log"))
    sizes = {p.name: p.stat().st_size for p in logs}
    ob = (res.json or {}).get("obligations") or [{}]
    truncated = bool(ob and ob[0].get("stdout_truncated")) and bool(
        ob and ob[0].get("stderr_truncated"))
    bound = 1024 * 1024 + 256  # cap + truncation marker + slack
    bounded = all(size <= bound for size in sizes.values()) and bool(sizes)
    ok_wall = res.duration_ms < 30000
    mem_ok = (res.peak_ws == 0) or (res.peak_ws < MAX_SCENARIO_BYTES)
    if (res.exit == 0 and res.status == "PASSED" and truncated and bounded
            and ok_wall and mem_ok):
        return X("10 MiB per stream is drained and truncated with a marker; the "
                 "run returns in bounded time and memory",
                 f"PASSED in {res.duration_ms} ms; logs {sizes}; truncated={truncated}; "
                 f"peak_ws={res.peak_ws and round(res.peak_ws/1048576,1)} MB",
                 **{"run": res.brief(), "log_sizes": sizes})
    return R("massive obligation output must be bounded and flagged",
             f"exit={res.exit} status={res.status} truncated={truncated} "
             f"sizes={sizes} wall_ms={res.duration_ms} peak={res.peak_ws}",
             **{"run": res.brief(), "log_sizes": sizes})


def prc_ignores_termination(c: Case):
    code = ("import signal, time\n"
            "signal.signal(signal.SIGTERM, signal.SIG_IGN)\n"
            "signal.signal(signal.SIGINT, signal.SIG_IGN)\n"
            "end = time.time() + 20\n"
            "while time.time() < end: time.sleep(0.2)\n")
    c.prepare([ob_python(code, id="stubborn", timeout_s=2)], staged=True)
    res = c.flow("run", timeout=60)
    if (res.exit == 1 and res.status == "TIMEOUT" and res.duration_ms < 15000
            and not res.timed_out):
        return X("an obligation ignoring SIGTERM is killed with the tree and the "
                 "run returns TIMEOUT in bounded time",
                 f"TIMEOUT in {res.duration_ms} ms",
                 **{"run": res.brief()})
    return R("a non-cooperative obligation must not hang the runner",
             f"exit={res.exit} status={res.status} wall={res.duration_ms} ms "
             f"harness_timeout={res.timed_out}", **{"run": res.brief()})


def prc_detached_grandchild(c: Case):
    script = c.root / "grandchild_probe.py"
    script.write_text(
        "import subprocess, sys, time\n"
        "flags = 0x00000008 | 0x00000200  # DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP\n"
        "subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(8)'],\n"
        "                 stdout=sys.stdout, stderr=sys.stderr,\n"
        "                 creationflags=flags if sys.platform == 'win32' else 0,\n"
        "                 start_new_session=sys.platform != 'win32')\n"
        "print('parent done', flush=True)\n",
        encoding="utf-8")
    # The obligation is the probe script itself, run from the scratch checkout;
    # the absolute path is visible to the obligation (same user).
    c.prepare([ob_python(
        f"import runpy; runpy.run_path(r'{script}')", id="grandchild", timeout_s=8)],
        staged=True)
    res = c.flow("run", timeout=60)
    if res.exit == 0 and res.status == "PASSED" and res.duration_ms < 20000 \
            and not res.timed_out:
        return X("a detached grandchild holding the inherited pipe cannot hang "
                 "the CLI: the reader grace bounds the call",
                 f"PASSED in {res.duration_ms} ms with a detached grandchild "
                 f"holding stdout",
                 notes="The detached descendant survives (documented "
                       "UNSUPPORTED); it exits on its own after 8 s. Only the "
                       "bounded return is claimed here.",
                 **{"run": res.brief()})
    return R("a detached grandchild must not hang the runner",
             f"exit={res.exit} status={res.status} wall={res.duration_ms} ms "
             f"harness_timeout={res.timed_out}", **{"run": res.brief()})


def prc_short_timeout(c: Case):
    c.prepare([ob_sleep(30, timeout_s=1)], staged=True)
    res = c.flow("run", timeout=60)
    if res.exit == 1 and res.status == "TIMEOUT" and 500 <= res.duration_ms < 10000:
        return X("a 1 s timeout is honoured; the 30 s obligation is killed and the "
                 "run returns TIMEOUT",
                 f"TIMEOUT in {res.duration_ms} ms", **{"run": res.brief()})
    return R("a short timeout must bound the obligation",
             f"exit={res.exit} status={res.status} wall={res.duration_ms} ms",
             **{"run": res.brief()})


def prc_git_env_hijack(c: Case):
    a = c.fresh("repo-a", [ob_green()], staged=True)
    b = c.new_repo("repo-b", files={"other.txt": "OTHER\n"}, staged=False)
    a_tree = c.git_out("write-tree", cwd=a)
    b_tree = c.git_out("write-tree", cwd=b)
    b_before = take_snapshot(b)
    res = c.flow("run", repo=a, env={
        "GIT_DIR": str(b / ".git"),
        "GIT_WORK_TREE": str(b),
        "GIT_INDEX_FILE": str(b / ".git" / "index"),
        "GIT_OBJECT_DIRECTORY": str(b / ".git" / "objects"),
    })
    receipt_tree = (res.json or {}).get("tree_oid")
    b_after = take_snapshot(b)
    if res.exit == 0 and receipt_tree == a_tree and b_before.files == b_after.files \
            and b_before.index == b_after.index:
        return X("a hijacked GIT_DIR/GIT_WORK_TREE/GIT_INDEX_FILE does not redirect "
                 "Flow: it verifies repository A and leaves B untouched",
                 f"receipt tree == A index tree {a_tree[:12]}; B untouched",
                 **{"run": res.brief(), "b_tree": b_tree})
    return R("hostile Git environment variables must not redirect verification",
             f"exit={res.exit} receipt_tree={str(receipt_tree)[:12]} "
             f"a_tree={a_tree[:12]} b_changed={b_before.files != b_after.files}",
             **{"run": res.brief()})


def prc_git_config_hijack(c: Case):
    c.new_repo(staged=True)
    c.init_flow()
    c.write_config([ob_green()])
    evil = c.root / "evil.gitconfig"
    evil.write_text("[core]\n\tsymlinks = false\n\thooksPath = .evil-hooks\n"
                    "[filter \"evil\"]\n\tsmudge = cat /dev/null\n", encoding="utf-8")
    res = c.flow("run", env={"GIT_CONFIG_GLOBAL": str(evil),
                             "GIT_CONFIG_SYSTEM": str(evil)})
    if res.exit == 0 and res.status == "PASSED":
        return X("a hostile GIT_CONFIG_GLOBAL/GIT_CONFIG_SYSTEM is scrubbed before "
                 "every `git` child", "PASSED with the real repository",
                 **{"run": res.brief()})
    return R("hostile global Git configuration must not redirect the run",
             f"exit={res.exit} status={res.status}", **{"run": res.brief()})


def prc_missing_tool(c: Case):
    c.prepare([{"id": "missing", "program": "flow-qa-no-such-program-xyz",
                "args": [], "required": True, "timeout_s": 10}], staged=True)
    res = c.flow("run")
    if (res.exit == 1 and res.status == "UNSUPPORTED"
            and res.duration_ms < 10000 and not res.timed_out):
        return X("a missing program is UNSUPPORTED, typed and bounded",
                 f"UNSUPPORTED in {res.duration_ms} ms", **{"run": res.brief()})
    return R("a missing program must produce a typed, bounded refusal",
             f"exit={res.exit} status={res.status} wall={res.duration_ms}",
             **{"run": res.brief()})


def prc_post_checkout_hook(c: Case):
    c.new_repo(staged=True)
    c.init_flow()
    c.write_config([ob_green()])
    marker = c.root / "post-checkout.marker"
    hook = c.repo / ".git" / "hooks" / "post-checkout"
    hook.parent.mkdir(parents=True, exist_ok=True)
    hook.write_text(f"#!/bin/sh\nprintf '%s\\n' \"$PWD\" > '{marker}'\n",
                    encoding="utf-8")
    before = c.snapshot()
    res = c.flow("run")
    ran = marker.exists()
    changed = before.changed_user_files(c.snapshot())
    if ran and not changed:
        return X("documented: Flow dispatches no hook of its own, but `git "
                 "worktree add` runs a `post-checkout` hook; it executes with cwd "
                 "= the scratch checkout and user files are untouched",
                 f"post-checkout ran; cwd={marker.read_text(errors='replace').strip()[:80]!r}",
                 notes="Observed behaviour today, matching `doctor`'s non-guarantee "
                       "wording. This is the HOOK-GITDIR-19 boundary: a hook can "
                       "reach the shared git directory; Flow refuses only when the "
                       "hook aborts Git, not when it writes.",
                 **{"run": res.brief(),
                    "hook_cwd": marker.read_text(errors="replace").strip()})
    return R("hook dispatch must match the documented non-guarantee",
             f"hook_ran={ran} files_changed={changed} run={res.brief()}",
             **{"run": res.brief()})


def prc_reference_transaction_hook(c: Case):
    c.new_repo(staged=True)
    c.init_flow()
    c.write_config([ob_green()])
    marker = c.root / "reftx.marker"
    hook = c.repo / ".git" / "hooks" / "reference-transaction"
    hook.parent.mkdir(parents=True, exist_ok=True)
    hook.write_text(f"#!/bin/sh\nprintf '%s\\n' \"$1\" >> '{marker}'\ncat >> '{marker}.input'\n",
                    encoding="utf-8")
    run = c.flow("run")
    phases_during_run = marker.read_text().split() if marker.exists() else []
    # Blocking hook, installed after the run, must abort the publication.
    hook.write_text("#!/bin/sh\n[ \"$1\" = prepared ] && exit 1\nexit 0\n", encoding="utf-8")
    before_ref = c.ref()
    pub = c.flow("publish")
    if (run.exit == 0 and phases_during_run and pub.exit == 1
            and pub.code in ("UPDATE_REF_REFUSED", "REF_MOVED")
            and c.ref() == before_ref):
        return X("documented: the Git plumbing runs `reference-transaction` during "
                 "`worktree add` and `update-ref`; a blocking hook aborts publish "
                 "with a typed refusal and no ref move",
                 f"phases during run: {sorted(set(phases_during_run))}; blocking "
                 f"publish -> {pub.code}, ref unchanged",
                 **{"run": run.brief(), "publish": pub.brief(),
                    "phases": sorted(set(phases_during_run))})
    return R("reference-transaction behaviour must match the documentation and "
             "abort cleanly",
             f"run={run.brief()} phases={sorted(set(phases_during_run))} "
             f"publish={pub.brief()}", **{"run": run.brief(), "publish": pub.brief()})


def prc_test_crash_env(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    after_intent = c.flow("publish", env={"FLOW_TEST_CRASH": "after-intent"})
    after_ref = c.flow("publish", env={"FLOW_TEST_CRASH": "after-ref"})
    ignored = c.flow("publish", env={"FLOW_TEST_CRASH": "not-a-mode"})
    honored = after_intent.exit == 89 and after_ref.exit == 89
    if honored:
        return R("a test-only crash switch is honoured unconditionally by the "
                 "shipped binary and exits 89, outside the documented exit-code "
                 "contract (0/1/2)",
                 f"FLOW_TEST_CRASH=after-intent -> exit {after_intent.exit}; "
                 f"after-ref -> exit {after_ref.exit}; unrecognised value -> "
                 f"exit {ignored.exit}",
                 notes="P2: observed while testing durability. The hook is not "
                       "compiled out and not gated by an opt-in flag; it can abort "
                       "a publication between intent and CAS (or after CAS) in any "
                       "environment where the variable is set. Impact is limited "
                       "to denial/forced recovery, but it is a test backdoor in a "
                       "production binary and exit 89 is outside the documented "
                       "contract.",
                 **{"after_intent": after_intent.brief(),
                    "after_ref": after_ref.brief(), "unknown_mode": ignored.brief()})
    return X("the crash switch is only honoured in test builds",
             f"after-intent exit={after_intent.exit}, after-ref exit={after_ref.exit}",
             **{"after_intent": after_intent.brief(),
                "after_ref": after_ref.brief()})


def prc_unknown_flags(c: Case):
    c.fresh("repo-dry-run", [ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    before = c.ref()
    dry = c.flow("publish", "--dry-run")
    ref_after_dry = c.ref()
    c.fresh("repo-ref-value", [ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    before_ref = c.ref()
    no_value = c.flow("publish", "--ref")
    ref_after_no_value = c.ref()
    c.fresh("repo-selection", [ob_green(), ob_red()], staged=False)
    selection = c.flow("run", "--obligation")
    accepted = (dry.exit == 0 and dry.status == "PUBLISHED" and ref_after_dry != before)
    silent_ref = (no_value.exit == 0 and no_value.status == "PUBLISHED"
                  and ref_after_no_value != before_ref)
    full_run = selection.exit == 1 and selection.status == "FAILED"
    if accepted or silent_ref or full_run:
        return R("unknown options and options missing their value must be usage "
                 "errors (exit 2), never a live database- or ref-mutating action",
                 f"`publish --dry-run` -> {dry.status} (ref moved: {accepted}); "
                 f"`publish --ref` with no value -> {no_value.status} "
                 f"(ref moved: {silent_ref}); `run --obligation` with no value -> "
                 f"{selection.status} (both obligations executed: {full_run})",
                 notes="P1: the argv parser ignores every token it does not know "
                       "and treats a flag without its value as absent. An operator "
                       "asking for a dry run publishes; a truncated `--ref` "
                       "publishes to the current branch; a truncated `--obligation` "
                       "runs the whole policy (a red required obligation makes it "
                       "FAILED here, proving both ran). The documented usage-error "
                       "exit code (2) is never used for these cases.",
                 **{"dry_run": dry.brief(), "ref_without_value": no_value.brief(),
                    "obligation_without_value": selection.brief(),
                    "ref_moved_by_dry_run": accepted,
                    "ref_moved_without_value": silent_ref})
    return X("unknown options are refused with exit 2",
             "none of the three probes took effect", **{"dry_run": dry.brief()})


def prc_env_canary(c: Case):
    c.prepare([ob_python(
        "import os, sys\n"
        f"print('env=' + str(os.environ.get('FLOW_FAKE_SECRET_CANARY')))\n"
        f"print('{CANARY}')\n", id="canary")], staged=True)
    res = c.flow("run", env={"FLOW_FAKE_SECRET_CANARY": CANARY})
    logs = sorted((c.repo / ".flow" / "logs").rglob("*.log"))
    log_text = "".join(p.read_text(errors="replace") for p in logs)
    preview = ""
    receipt = (res.json or {}).get("obligations") or [{}]
    if receipt:
        preview = receipt[0].get("stdout_preview") or ""
    leaked = CANARY in log_text or CANARY in preview
    env_seen = "env=None" in log_text
    if res.exit == 0 and not leaked and env_seen:
        return X("the host canary is not inherited by the obligation and is "
                 "redacted before persistence",
                 "obligation env=None; log and receipt preview contain [REDACTED]",
                 **{"run": res.brief(), "logs": [p.name for p in logs],
                    "preview": preview[:120]})
    return R("a host secret must not reach the obligation or the persisted logs",
             f"leaked={leaked} env_seen={env_seen} log={log_text[:120]!r}",
             **{"run": res.brief()})


def prc_scratch_cleanup(c: Case):
    c.prepare([ob_green()], staged=True)
    # The child Flow process gets TEMP/TMP/TMPDIR pointing at this scenario's
    # root, so its scratch base is here (it must not use the harness temp dir).
    base = c.root
    before = {p.name for p in base.glob("flow-scratch-*")}
    res = c.flow("run")
    after = {p.name for p in base.glob("flow-scratch-*")}
    leftovers = sorted(after - before)
    if res.exit == 0 and not leftovers:
        return X("the owned scratch checkout is removed on drop: no residue",
                 f"no new flow-scratch-* directory under {base.name}",
                 **{"run": res.brief()})
    return R("scratch directories must be cleaned up", f"leftovers={leftovers}",
             **{"run": res.brief()})


# ==========================================================================
# AXIS 7 — proof quality: receipt forgery, evidence files, doctor claims
# ==========================================================================

def prf_forge_receipt(c: Case):
    c.prepare([ob_green(), ob_red()], staged=True)
    run = c.flow("run")
    check(run.exit == 1 and run.status == "FAILED", f"red run must fail: {run.brief()}")
    tree = c.git_out("write-tree")
    receipt_path = c.receipt_for_tree(tree)
    value = c.read_receipt(receipt_path)
    # Self-check: our canonical serializer must reproduce the real digest before
    # a forged digest means anything.
    resealed_raw = seal_receipt(dict(value))
    if resealed_raw.get("integrity") != value.get("integrity"):
        raise NotRun("the harness cannot reproduce the Rust JSON canonical form "
                     "for this receipt; the forgery scenario would be invalid")
    value["status"] = "PASSED"
    for entry in value.get("obligations", []):
        if entry.get("id") == "red":
            entry["status"] = "PASSED"
            entry["exit"] = 0
    forged = seal_receipt(value)
    c.write_receipt(receipt_path, forged)
    res = c.flow("publish")
    if res.exit == 0 and res.status == "PUBLISHED":
        return R("a receipt must not be forgeable by a writer who can also edit "
                 "and re-seal the file",
                 f"PUBLISHED a commit for a tree whose required obligation was "
                 f"FAILED: status and obligation entries edited, integrity digest "
                 f"recomputed ({forged['integrity'][:12]}...)",
                 notes="P2: exactly the documented honest boundary ('no "
                       "cryptographic protection of a receipt: anyone who can "
                       "write .flow/receipts can forge one'). Reproduced "
                       "empirically so the report's claim is measured, not "
                       "assumed. Severity P2, not P0: the limitation is stated in "
                       "`doctor` and in the hardening report; only a signature "
                       "could close it. The casual edit (no re-seal) is refused "
                       "with RECEIPT_INTEGRITY (IDN-11).",
                 **{"run": run.brief(), "publish": res.brief(),
                    "forged_integrity": forged["integrity"][:16]})
    return X("a re-sealed forged receipt is refused",
             f"publish={res.brief()}", **{"run": run.brief(), "publish": res.brief()})


def prf_deleted_log(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    logs = sorted((c.repo / ".flow" / "logs").rglob("*.log"))
    for path in logs:
        path.unlink()
    res = c.flow("publish")
    if res.exit == 0 and res.status == "PUBLISHED":
        return R("publish claims evidence, but the evidence files it names can be "
                 "deleted without any check",
                 f"PUBLISHED with all {len(logs)} log files deleted; publish output "
                 f"reports receipt_logs={res.json.get('receipt_logs')}",
                 notes="P3 observation: not a gate breach (the receipt's own "
                       "content is intact and still binds tree+policy), but "
                       "`receipt_logs` in the publish output reads as if the logs "
                       "were present. The receipt lists paths; nothing verifies "
                       "they exist.",
                 **{"publish": res.brief(), "deleted_logs": [p.name for p in logs]})
    return X("deleting the log files does not turn a green proof red",
             f"publish={res.brief()}", **{"publish": res.brief()})


def prf_change_after_run(c: Case):
    results = {}
    # (a) worktree-only change: the index tree is unchanged, so the proof stands.
    c.fresh("repo-worktree", [ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    c.write_file("data.txt", "CHANGED-AFTER-RUN\n")
    worktree_pub = c.flow("publish")
    results["worktree_change"] = worktree_pub.brief()
    # (b) staged change: the index tree moves, so the receipt no longer names it.
    c.fresh("repo-staged", [ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    c.stage("data.txt", "STAGED-AFTER-RUN\n")
    staged_pub = c.flow("publish")
    results["staged_change"] = staged_pub.brief()
    if worktree_pub.exit == 0 and staged_pub.exit == 1 and staged_pub.code == "NO_RECEIPT":
        return X("a worktree edit does not invalidate the index-tree proof (by "
                 "design); a staged edit changes the tree and is NO_RECEIPT",
                 "worktree -> PUBLISHED, staged -> NO_RECEIPT",
                 **results)
    return R("a staged change after run must never reuse the receipt",
             f"worktree={worktree_pub.brief()} staged={staged_pub.brief()}",
             **results)


def prf_tampered_status_display(c: Case):
    c.prepare([ob_green(), ob_red()], staged=True)
    run = c.flow("run")
    check(run.exit == 1 and run.status == "FAILED", "red run must fail")
    tree = c.git_out("write-tree")
    receipt_path = c.receipt_for_tree(tree)
    value = c.read_receipt(receipt_path)
    value["status"] = "PASSED"  # integrity deliberately left stale
    c.write_receipt(receipt_path, value)
    status = c.flow("status")
    explain = c.flow("explain", "--tree", tree)
    latest = (status.json or {}).get("latest_receipt") or {}
    shown = latest.get("status")
    explain_receipt = (explain.json or {}).get("receipt") or {}
    publish = c.flow("publish")
    if status.exit == 0 and shown == "PASSED" and explain_receipt.get("status") == "PASSED" \
            and publish.exit == 1 and publish.code == "RECEIPT_INTEGRITY":
        return R("`status` and `explain` must not present a receipt whose integrity "
                 "digest no longer matches",
                 f"status reports latest_receipt.status=PASSED from a receipt that "
                 f"publish refuses with RECEIPT_INTEGRITY; explain echoes the same "
                 f"tampered receipt",
                 notes="P2: the publication gate holds (publish refuses), but the "
                       "human-facing report shows a false green. An operator "
                       "checking `flow status` sees PASSED for an edited receipt.",
                 **{"status": status.brief(), "explain": explain.brief(),
                    "publish": publish.brief(), "shown_status": shown})
    return X("status/explain refuse to display an unsealed receipt",
             f"status shown={shown} explain={explain_receipt.get('status')} "
             f"publish={publish.brief()}", **{"status": status.brief()})


def prf_doctor_honesty(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    _crash_after_intent(c)
    path = c.journal_path()
    path.write_bytes(path.read_bytes()[:-5])  # genuinely torn line
    doctor = c.flow("doctor")
    report = doctor.json or {}
    journal_check = next((c for c in report.get("checks", [])
                          if c.get("id") == "journal"), {})
    overall = report.get("overall")
    doctor_exit = doctor.exit
    guarantees = report.get("guarantees") or []
    non_guarantees = report.get("non_guarantees") or []
    claims_eol = any("filter" in g or "eol" in g for g in non_guarantees)
    claims_forge = any("forge" in g or "cryptographic" in g for g in non_guarantees)
    # The report is honest about the forge boundary this harness reproduced; the
    # checkout byte-conversion limit is only disclosed by the `gitattributes`
    # check, and only when the repository actually carries one.
    honest = claims_forge
    if journal_check.get("status") == "FAILED" and overall == "FAILED" and doctor_exit == 0:
        return R("`doctor` must fail the process (or at least not exit 0) when its "
                 "own journal check is FAILED, and its disclosures must match the "
                 "code",
                 f"journal check FAILED, overall=FAILED, but exit=0; forge "
                 f"disclosed={claims_forge}, eol/filter in static non_guarantees="
                 f"{claims_eol} (only the conditional `gitattributes` check "
                 f"mentions it)",
                 notes="P2: `doctor` exits 0 because its status is REPORT (the "
                       "documented code for 'a report was printed'), so a CI job "
                       "checking only the exit code treats a corrupt journal as "
                       "success. The forgery boundary is disclosed and matches "
                       "PRF-forge-receipt; the checkout byte-conversion limit is "
                       "disclosed only when `.gitattributes` is present, which is "
                       "a weaker promise than the report implies.",
                 **{"doctor": doctor.brief(), "journal_check": journal_check,
                    "forge_disclosed": claims_forge,
                    "eol_static_disclosure": claims_eol})
    return X("doctor exits nonzero when its own report is FAILED",
             f"exit={doctor_exit} overall={overall}", **{"doctor": doctor.brief()})


def prf_explain_traversal(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    secret = c.root / "outside-secret.json"
    secret.write_text('{"secret":"' + CANARY + '"}', encoding="utf-8")
    absolute = str(secret.with_suffix(""))
    traversal = "../../../outside-secret"
    a = c.flow("explain", "--tree", absolute)
    t = c.flow("explain", "--tree", traversal)
    a_rec = (a.json or {}).get("receipt") or {}
    t_rec = (t.json or {}).get("receipt") or {}
    if a_rec.get("secret") == CANARY or t_rec.get("secret") == CANARY:
        return R("`explain --tree` must validate the tree argument, not read an "
                 "arbitrary JSON file by path traversal or absolute path",
                 f"explain --tree {absolute!r} returned the outside file content "
                 f"{json.dumps(a_rec)[:80]}; traversal {traversal!r} returned "
                 f"{json.dumps(t_rec)[:80]}",
                 notes="P2: `receipt_path` joins the unvalidated `--tree` value to "
                       "`.flow/receipts/<value>.json`; an absolute path replaces "
                       "the base. Read-only (no write), but it is the PATH-08 class "
                       "the project claims to validate, and it can echo any "
                       "readable JSON file.",
                 **{"absolute": a.brief(), "traversal": t.brief(),
                    "canary_found": True})
    return X("explain --tree refuses absolute/traversal paths",
             f"absolute={a.brief()} traversal={t.brief()}",
             **{"absolute": a.brief(), "traversal": t.brief()})


# ==========================================================================
# AXIS 8 — compatibility: output, concurrency, exit codes
# ==========================================================================

def cmp_json_default(c: Case):
    c.prepare([ob_green()], staged=True)
    c.stage("x.txt", "x\n")  # staged? already staged feature; harmless
    checks = {}
    for name, args in [
        ("status", ("status",)),
        ("version", ("version",)),
        ("doctor", ("doctor",)),
        ("explain", ("explain",)),
        ("run", ("run",)),
        ("publish", ("publish",)),
        ("reconcile", ("reconcile",)),
        ("unknown-command", ("no-such-command",)),
        ("no-args", ()),
        ("help", ("help",)),
    ]:
        res = c.flow(*args)
        parsed = res.json is not None
        checks[name] = {"exit": res.exit, "json": parsed,
                        "status": (res.json or {}).get("status")}
    text_cases = [n for n, v in checks.items() if not v["json"]]
    if not text_cases:
        return X("every command prints parseable JSON on stdout",
                 f"{len(checks)} commands parsed", **checks)
    return R("the usage text promises machine-readable JSON for every command",
             f"these invocations printed non-JSON text: {text_cases} "
             f"(exit codes {[(n, checks[n]['exit']) for n in text_cases]})",
             notes="P3: `help` (documented as a command) and a bare invocation "
                   "print the usage text; everything else is JSON. Documentation "
                   "overclaim, no gate impact.",
             **checks)


def cmp_json_flag(c: Case):
    c.prepare([ob_green()], staged=True)
    status = c.flow("status", "--json")
    run = c.flow("run", "--json")
    if status.exit == 0 and status.json is not None and run.exit == 0 \
            and run.json is not None:
        return X("`--json` is accepted (as a no-op) and the output stays JSON: JSON "
                 "is the default", f"status exit={status.exit}, run exit={run.exit}",
                 notes="The flag is not validated; it is silently ignored, which is "
                       "the same parser gap as PRC-unknown-flags.",
                 **{"status": status.brief(), "run": run.brief()})
    return R("`--json` must not break the default JSON contract",
             f"status={status.brief()} run={run.brief()}",
             **{"status": status.brief(), "run": run.brief()})


def cmp_exit_codes(c: Case):
    c.fresh("repo-a", [ob_green()], staged=True)
    green_run = c.flow("run")
    clean = c.flow("status")
    no_args = c.flow()
    unknown = c.flow("nope")
    help_res = c.flow("help")
    init_again = c.flow("init")
    c.fresh("repo-b", [ob_red()], staged=True)
    red_run = c.flow("run")
    publish_refused = c.flow("publish")
    table = {
        "green run": (green_run.exit, 0),
        "status": (clean.exit, 0),
        "no args": (no_args.exit, 2),
        "unknown command": (unknown.exit, 2),
        "help": (help_res.exit, 0),
        "red run": (red_run.exit, 1),
        "publish refusal": (publish_refused.exit, 1),
        "init twice": (init_again.exit, 1),
    }
    bad = {k: v for k, v in table.items() if v[0] != v[1]}
    evidence = {k: {"observed": v[0], "expected": v[1]} for k, v in table.items()}
    if not bad:
        return X("the documented exit-code contract (0 success, 1 refusal, 2 "
                 "usage) holds for the exercised matrix", "all 8 cases match",
                 **evidence)
    return R("documented exit codes must be stable", f"mismatches={bad}", **evidence)


def cmp_concurrent_publish(c: Case):
    c.prepare([ob_green()], staged=True)
    check(c.flow("run").exit == 0, "green run must pass")
    commit_count = int(c.git_out("rev-list", "--count", "HEAD") or 0)
    env = dict(c.env)
    procs = [subprocess.Popen([str(c.flow_bin), "publish"], cwd=str(c.repo), env=env,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                              stdin=subprocess.DEVNULL) for _ in range(2)]
    results = []
    for proc in procs:
        out, err = proc.communicate(timeout=60)
        text = out.decode("utf-8", "replace")
        try:
            parsed = json.loads(text)
        except Exception:
            parsed = {}
        results.append({"exit": proc.returncode, "status": parsed.get("status"),
                        "code": parsed.get("code"),
                        "reason": (parsed.get("reason") or parsed.get("error") or "")[:160]})
        c.commands.append({"argv": "<concurrent flow publish>", "exit": proc.returncode,
                           "result": {"status": parsed.get("status"),
                                      "code": parsed.get("code")}})
    published = [r for r in results if r["status"] == "PUBLISHED" and r["exit"] == 0]
    refused = [r for r in results if r["exit"] != 0]
    new_count = int(c.git_out("rev-list", "--count", "HEAD") or 0)
    pending = (c.flow("status").json or {}).get("pending_intents")
    if len(published) == 1 and len(refused) == 1 and new_count == commit_count + 1 \
            and pending == 0:
        return X("two concurrent publishes publish at most once: one PUBLISHED, the "
                 "other refused, one commit, no pending intent",
                 f"results={results} commits {commit_count}->{new_count} "
                 f"pending={pending}", **{"results": results})
    return R("concurrent publishes must publish at most once",
             f"results={results} commits {commit_count}->{new_count} pending={pending}",
             **{"results": results})


def cmp_concurrent_readers(c: Case):
    c.prepare([ob_green()], staged=True)
    env = dict(c.env)
    procs = [subprocess.Popen([str(c.flow_bin), "status"], cwd=str(c.repo), env=env,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                              stdin=subprocess.DEVNULL) for _ in range(2)]
    procs.append(subprocess.Popen([str(c.flow_bin), "version"], cwd=str(c.repo),
                                  env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                  stdin=subprocess.DEVNULL))
    results = []
    for proc in procs:
        out, _ = proc.communicate(timeout=60)
        try:
            parsed = json.loads(out.decode("utf-8", "replace"))
        except Exception:
            parsed = {}
        results.append({"exit": proc.returncode, "status": parsed.get("status")})
    if all(r["exit"] == 0 and r["status"] in ("REPORT", "VERSION") for r in results):
        return X("concurrent read-only CLI processes do not interfere",
                 f"{results}", **{"results": results})
    return R("concurrent readers must stay valid", f"{results}", **{"results": results})


def cmp_concurrent_runs(c: Case):
    c.prepare([ob_green()], staged=True)
    env = dict(c.env)
    procs = [subprocess.Popen([str(c.flow_bin), "run"], cwd=str(c.repo), env=env,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                              stdin=subprocess.DEVNULL) for _ in range(2)]
    results = []
    for proc in procs:
        out, _ = proc.communicate(timeout=60)
        try:
            parsed = json.loads(out.decode("utf-8", "replace"))
        except Exception:
            parsed = {}
        results.append({"exit": proc.returncode, "status": parsed.get("status"),
                        "error": (parsed.get("error") or parsed.get("reason") or "")[:160]})
        c.commands.append({"argv": "<concurrent flow run>", "exit": proc.returncode,
                           "result": {"status": parsed.get("status")}})
    receipts = [p for p in c.receipt_files() if ".partial." not in p.name]
    valid = []
    for path in receipts:
        try:
            receipt = json.loads(path.read_text(encoding="utf-8"))
            if "integrity" in receipt:
                # A complete, sealed receipt must be usable by publish.
                valid.append(path.name)
        except Exception:
            pass
    passed = [r for r in results if r["exit"] == 0 and r["status"] == "PASSED"]
    failed = [r for r in results if r["exit"] == 1]
    typed_failure = all(r["status"] == "ERROR" and r["error"] for r in failed)
    no_corruption = len(valid) == 1
    if len(passed) >= 1 and typed_failure and no_corruption:
        note = ""
        if failed:
            note = ("Concurrent runs are not serialised (only publish/reconcile take "
                    "the advisory lock). The losing process fails loudly with a "
                    "typed ERROR such as a Git index.lock collision; a retry "
                    "succeeds. No corruption and no false PASSED was observed; the "
                    "single complete receipt is well formed.")
        return X("two concurrent runs must not corrupt the receipt or report a "
                 "false PASSED",
                 f"passed={len(passed)} failed={len(failed)} "
                 f"receipts={valid} errors={[r['error'][:60] for r in failed]}",
                 notes=note, **{"results": results, "receipts": valid})
    return R("concurrent runs must not corrupt the receipt or report a false PASSED",
             f"results={results} receipts={valid}", **{"results": results})


def cmp_unknown_config_keys(c: Case):
    c.prepare([ob_python("import os; print('cfg=' + str(os.environ.get('FLOW_QA_CFG')))",
                         id="probe")], staged=True)
    config = json.loads((c.repo / ".flow" / "config.json").read_text())
    config["sandbox"] = True
    config["obligations"][0]["sandbox"] = True
    config["obligations"][0]["env"] = {"FLOW_QA_CFG": "set-by-config"}
    (c.repo / ".flow" / "config.json").write_text(json.dumps(config, indent=2) + "\n",
                                                  encoding="utf-8")
    run = c.flow("run")
    logs = sorted((c.repo / ".flow" / "logs").rglob("*.log"))
    text = "".join(p.read_text(errors="replace") for p in logs)
    if run.exit == 0 and "cfg=None" in text and "set-by-config" not in text:
        return X("unknown configuration keys are ignored (no silent effect), so the "
                 "policy digest is unchanged",
                 "`sandbox: true` and an obligation `env` map have no effect; "
                 "obligation env=None",
                 notes="P3 observation: unknown keys are accepted without a warning. "
                       "An operator who believes `sandbox: true` enables "
                       "confinement gets no error and no confinement. `doctor` does "
                       "say there is no OS isolation, but the config surface itself "
                       "does not refuse unknown keys.",
                 **{"run": run.brief(), "log_excerpt": text[:120]})
    return R("unknown configuration keys must not be silently ignored",
             f"run={run.brief()} log={text[:120]!r}", **{"run": run.brief()})


# ==========================================================================
# AXIS 9 — future commands (absent at the base commit)
# ==========================================================================

def fut_unimplemented(c: Case):
    raise NotRun("scenario declared for the new commands; implementation pending "
                 "(enable by adding its body to FUTURE_SCENARIOS once the binary "
                 "exists)")


# --------------------------------------------------------------------------
# Registry
# --------------------------------------------------------------------------

SCENARIOS = [
    # Axis 1 — identity
    Scn("IDN-01", "identity", "Index-tree change after a green run", "P0", idn_index_tree_change),
    Scn("IDN-02", "identity", "Worktree-only change after a green run", "P2", idn_worktree_change),
    Scn("IDN-03", "identity", "Required obligation added after the run", "P0", idn_add_required),
    Scn("IDN-04", "identity", "Required obligation args changed after the run", "P0", idn_change_required_args),
    Scn("IDN-05", "identity", "Required obligation removed after the run", "P0", idn_remove_required),
    Scn("IDN-06", "identity", "timeout_s changed after the run", "P0", idn_change_timeout),
    Scn("IDN-07", "identity", "policy_version changed after the run", "P0", idn_change_policy_version),
    Scn("IDN-08", "identity", "execution_boundary changed after the run", "P0", idn_change_boundary),
    Scn("IDN-09", "identity", "`--timeout` override used at run time", "P0", idn_timeout_override),
    Scn("IDN-10", "identity", "Receipt edited by hand (no re-seal)", "P0", idn_edited_receipt),
    Scn("IDN-11", "identity", "Cosmetic config edit after the run", "P2", idn_semantic_config_edit),
    Scn("IDN-12", "identity", "Partial run after the tree changed", "P0", idn_partial_changed_tree),
    Scn("IDN-13", "identity", "Partial run after a complete proof of the same tree", "P2", idn_partial_after_complete),
    Scn("IDN-14", "identity", "Duplicated `--obligation` id", "P2", idn_duplicate_selection),
    Scn("IDN-15", "identity", "Unknown `--obligation` id", "P2", idn_unknown_selection),
    Scn("IDN-16", "identity", "Receipt schema edited", "P0", idn_schema_edit),
    Scn("IDN-17", "identity", "Receipt copied to another tree's filename", "P0", idn_copied_receipt),
    Scn("IDN-18", "identity", "Receipt integrity digest removed", "P0", idn_integrity_missing),
    Scn("IDN-19", "identity", "Obligation order changed after the run", "P2", idn_reordered_policy),
    Scn("IDN-20", "identity", "Obligation `required` flag flipped true -> false", "P0", idn_required_flag_flip),

    # Axis 2 — scope
    Scn("SCP-01", "scope", "Empty policy: run", "P0", scp_empty_policy_run),
    Scn("SCP-02", "scope", "Empty policy: publish", "P0", scp_empty_policy_publish),
    Scn("SCP-03", "scope", "Policy with no required obligation", "P0", scp_only_optional),
    Scn("SCP-04", "scope", "Red required obligation", "P0", scp_red_required),
    Scn("SCP-05", "scope", "Obligation raises a Python exception", "P0", scp_python_exception),
    Scn("SCP-06", "scope", "Required E2E obligation is red", "P0", scp_e2e_red),
    Scn("SCP-07", "scope", "Partial run that skips a required obligation", "P0", scp_partial_missing_required),
    Scn("SCP-08", "scope", "Optional red obligation does not mask green", "P2", scp_optional_red_does_not_mask),
    Scn("SCP-09", "scope", "Required obligation whose program is missing", "P0", scp_missing_tool),
    Scn("SCP-10", "scope", "Required obligation times out", "P0", scp_timeout_required),
    Scn("SCP-11", "scope", "Disabled repository", "P2", scp_disabled),

    # Axis 3 — Git
    Scn("GIT-01", "git", "Stale --expected-old", "P0", git_stale_expected_old),
    Scn("GIT-02", "git", "External git update-ref between run and publish", "P0", git_external_move),
    Scn("GIT-03", "git", "`--ref HEAD`", "P0", git_ref_head),
    Scn("GIT-04", "git", "`--ref refs/tags/x`", "P0", git_ref_tag),
    Scn("GIT-05", "git", "Invalid ref names (`git check-ref-format`)", "P2", git_invalid_ref_names),
    Scn("GIT-06", "git", "Missing destination ref", "P0", git_missing_ref),
    Scn("GIT-07", "git", "Non-OID --expected-old", "P0", git_bogus_expected_old),
    Scn("GIT-08", "git", "Detached HEAD", "P2", git_detached_head),
    Scn("GIT-09", "git", "Symbolic ref under refs/heads/ (branch target)", "P2", git_symref_branch),
    Scn("GIT-10", "git", "Symbolic ref dereferencing to a tag", "P1", git_symref_tag),
    Scn("GIT-11", "git", "Blocking reference-transaction hook at publish", "P2", git_blocking_reftx_hook),
    Scn("GIT-12", "git", "Tree already carried by the ref", "P2", git_already_published),

    # Axis 4 — durable
    Scn("DUR-01", "durable", "Truncated journal line", "P0", dur_truncated_journal),
    Scn("DUR-02", "durable", "Duplicated intent id, one outcome", "P0", dur_duplicate_intent_id),
    Scn("DUR-03", "durable", "One outcome cannot close two intents (ref moved)", "P0", dur_one_outcome_two_intents_moved),
    Scn("DUR-04", "durable", "Crash after intent", "P0", dur_crash_after_intent),
    Scn("DUR-05", "durable", "Crash after CAS (after-ref)", "P0", dur_crash_after_ref),
    Scn("DUR-06", "durable", "Reconcile idempotence", "P2", dur_reconcile_idempotent),
    Scn("DUR-07", "durable", "Ref moved to a third commit after a crash", "P0", dur_ref_moved_to_third),
    Scn("DUR-08", "durable", "Journal deleted after a crash", "P2", dur_journal_deleted),
    Scn("DUR-09", "durable", "Crash before intent", "P2", dur_crash_before_intent),
    Scn("DUR-10", "durable", "Valid-JSON non-object journal line", "P3", dur_non_object_journal_line),
    Scn("DUR-11", "durable", "Crafted two intents with two outcomes", "P3", dur_crafted_balance),

    # Axis 5 — user files
    Scn("FIL-01", "files", "Dirty/untracked/read-only/index files preserved", "P0", fil_preserved_files),
    Scn("FIL-02", "files", "Verification reads the index, not the dirty worktree", "P1", fil_index_not_worktree),
    Scn("FIL-03", "files", "Flow state stays out of the verified tree", "P1", fil_flow_state_not_in_tree),
    Scn("FIL-04", "files", "Symlink tree entry checked out as a regular file", "P1", fil_symlink_entry),
    Scn("FIL-05", "files", "Case-colliding tree", "P1", fil_case_collision),
    Scn("FIL-06", "files", "Windows-reserved and `..` tree paths", "P1", fil_reserved_and_dotdot),
    Scn("FIL-07", "files", "Obligation id traversal", "P2", fil_obligation_id_traversal),
    Scn("FIL-08", "files", "EOL conversion: obligation does not see tree bytes", "P1", fil_eol_materialisation),
    Scn("FIL-09", "files", "Journal symlink -> write outside the workspace", "P0", fil_state_journal_symlink),
    Scn("FIL-10", "files", "Receipts symlink -> write outside the workspace", "P0", fil_state_receipts_symlink),
    Scn("FIL-11", "files", "Logs junction -> write outside `.flow` (inside repo)", "P2", fil_logs_symlink_within),
    Scn("FIL-12", "files", "Cloned repo ships a receipts symlink", "P0", fil_clone_receipts_symlink),
    Scn("FIL-13", "files", "Cloned repo ships a journal symlink", "P0", fil_clone_journal_symlink),
    Scn("FIL-14", "files", "Cloned repo ships a journal symlink (default clone)", "P0", fil_clone_default_symlink_refused),
    Scn("FIL-15", "files", "Cloned repo ships `.flow/config.json`", "P1", fil_clone_ships_config),

    # Axis 6 — process
    Scn("PRC-01", "process", "10 MiB stdout + stderr", "P2", prc_massive_output),
    Scn("PRC-02", "process", "Obligation ignores termination", "P2", prc_ignores_termination),
    Scn("PRC-03", "process", "Detached grandchild holds the pipe", "P2", prc_detached_grandchild),
    Scn("PRC-04", "process", "Very short timeout", "P2", prc_short_timeout),
    Scn("PRC-05", "process", "GIT_DIR/GIT_WORK_TREE/GIT_INDEX_FILE hijack", "P0", prc_git_env_hijack),
    Scn("PRC-06", "process", "GIT_CONFIG_GLOBAL/GIT_CONFIG_SYSTEM hijack", "P2", prc_git_config_hijack),
    Scn("PRC-07", "process", "Missing toolchain in an obligation", "P2", prc_missing_tool),
    Scn("PRC-08", "process", "post-checkout hook: what runs today", "P2", prc_post_checkout_hook),
    Scn("PRC-09", "process", "reference-transaction hook: what runs today", "P2", prc_reference_transaction_hook),
    Scn("PRC-10", "process", "FLOW_TEST_CRASH in the shipped binary", "P2", prc_test_crash_env),
    Scn("PRC-11", "process", "Unknown options / missing option values", "P1", prc_unknown_flags),
    Scn("PRC-12", "process", "Host secret canary and log redaction", "P2", prc_env_canary),
    Scn("PRC-13", "process", "Scratch cleanup", "P2", prc_scratch_cleanup),

    # Axis 7 — proof
    Scn("PRF-01", "proof", "Re-sealed forged receipt", "P2", prf_forge_receipt),
    Scn("PRF-02", "proof", "Log files deleted after the run", "P3", prf_deleted_log),
    Scn("PRF-03", "proof", "File changed after run, before publish", "P1", prf_change_after_run),
    Scn("PRF-04", "proof", "Tampered receipt displayed by status/explain", "P2", prf_tampered_status_display),
    Scn("PRF-05", "proof", "Doctor honesty and exit code", "P2", prf_doctor_honesty),
    Scn("PRF-06", "proof", "`explain --tree` path traversal", "P2", prf_explain_traversal),

    # Axis 8 — compatibility
    Scn("CMP-01", "compat", "JSON is the default and parseable", "P3", cmp_json_default),
    Scn("CMP-02", "compat", "`--json` flag is a no-op", "P2", cmp_json_flag),
    Scn("CMP-03", "compat", "Exit-code contract stability", "P2", cmp_exit_codes),
    Scn("CMP-04", "compat", "Two concurrent publishes", "P0", cmp_concurrent_publish),
    Scn("CMP-05", "compat", "Concurrent read-only processes", "P2", cmp_concurrent_readers),
    Scn("CMP-06", "compat", "Two concurrent runs", "P2", cmp_concurrent_runs),
    Scn("CMP-07", "compat", "Unknown configuration keys", "P3", cmp_unknown_config_keys),
]

# The new-command scenarios, kept in one list so they can be enabled later by
# replacing their `fn` here. Each entry names the command/option it needs.
FUTURE_SCENARIOS = [
    Scn("FUT-01", "future", "propose -> proposals -> assemble -> run --candidate -> "
        "publish --candidate lifecycle", "P1", fut_unimplemented, ("propose", "proposals",
                                                                  "assemble")),
    Scn("FUT-02", "future", "`run --candidate` verifies the assembled candidate, not "
        "the index", "P1", fut_unimplemented, ("--candidate",)),
    Scn("FUT-03", "future", "`publish --candidate` publishes the approved candidate "
        "only", "P1", fut_unimplemented, ("--candidate",)),
    Scn("FUT-04", "future", "`--dry-run` has no side effect", "P1", fut_unimplemented,
        ("--dry-run",)),
    Scn("FUT-05", "future", "`approve` records a human approval bound to a candidate",
        "P1", fut_unimplemented, ("approve",)),
    Scn("FUT-06", "future", "`proposals` output is machine-readable and stable", "P2",
        fut_unimplemented, ("proposals",)),
    Scn("FUT-07", "future", "`assemble` is deterministic (same inputs, same tree)",
        "P2", fut_unimplemented, ("assemble",)),
]


def help_text(flow: Path, cwd: Path, env: dict) -> str:
    res = run_process([str(flow), "help"], cwd, env, 30)
    return res.stdout


def command_available(flow: Path, cwd: Path, env: dict, command: str) -> bool:
    if command.startswith("--"):
        return command in help_text(flow, cwd, env)
    res = run_process([str(flow), command], cwd, env, 30)
    text = (res.stdout or "") + (res.stderr or "")
    if res.exit == 2 and "unknown command" in text:
        return False
    return command in help_text(flow, cwd, env)


# --------------------------------------------------------------------------
# Runner
# --------------------------------------------------------------------------

def run_scenario(scn: Scn, run_root: Path, flow: Path, base_sha: str) -> dict:
    started = time.monotonic()
    case = Case(scn, run_root, flow)
    record = {
        "id": scn.id,
        "axis": scn.axis,
        "title": scn.title,
        "severity": scn.severity,
        "verdict": "ERROR",
        "expected": "",
        "observed": "",
        "notes": "",
        "evidence": {},
        "commands": [],
        "duration_ms": 0,
    }
    try:
        if scn.requires:
            missing = [req for req in scn.requires
                       if not command_available(flow, case.root, case.env, req)]
            if missing:
                raise NotRun(
                    "command/option absent at branch " + base_sha[:7] + ": "
                    + ", ".join(missing)
                    + " (the new commands do not exist in this worktree; "
                      "scenario is declared and will be enabled when the binary "
                      "provides it)")
        outcome = scn.fn(case)
        if outcome is None:
            raise HarnessError("scenario returned no outcome")
        record.update({
            "verdict": outcome["verdict"],
            "expected": outcome["expected"],
            "observed": outcome["observed"],
            "notes": outcome.get("notes", ""),
            "evidence": outcome.get("evidence", {}),
        })
    except NotRun as exc:
        record.update({"verdict": "NOT_RUN", "observed": str(exc)})
    except HarnessError as exc:
        record.update({"verdict": "ERROR", "observed": str(exc)})
    except Failed as exc:
        record.update({"verdict": "ERROR", "observed": str(exc)})
    except Exception as exc:  # noqa: BLE001 - a harness bug must be visible
        record.update({
            "verdict": "ERROR",
            "observed": f"harness exception: {exc}",
            "notes": "".join(traceback.format_exception_only(type(exc), exc)).strip(),
        })
    finally:
        record["commands"] = case.commands
        record["duration_ms"] = int((time.monotonic() - started) * 1000)
        record["evidence"] = case.evidence | record.get("evidence", {})
        if not os.environ.get("FLOW_QA_KEEP", ""):
            try:
                safe_remove_tree(case.root)
            except Exception:
                pass
    return record


def summarize(records: list) -> dict:
    by_axis = {}
    by_severity = {}
    for record in records:
        axis = by_axis.setdefault(record["axis"], {"total": 0, "REPRODUCED": 0,
                                                   "REFUTED": 0, "NOT_RUN": 0,
                                                   "ERROR": 0})
        axis["total"] += 1
        axis[record["verdict"]] = axis.get(record["verdict"], 0) + 1
        if record["verdict"] == "REPRODUCED":
            key = record["severity"]
            by_severity[key] = by_severity.get(key, 0) + 1
    total = len(records)
    counts = {v: sum(1 for r in records if r["verdict"] == v)
              for v in ("REPRODUCED", "REFUTED", "NOT_RUN", "ERROR")}
    return {"total": total, **counts, "by_axis": by_axis,
            "reproduced_by_severity": by_severity}


def print_table(records: list) -> None:
    print("")
    header = f"{'ID':8} {'AXIS':9} {'VERDICT':11} {'SEV':4} ATTACK"
    print(header)
    print("-" * len(header))
    for record in records:
        title = record["title"]
        if len(title) > 78:
            title = title[:75] + "..."
        print(f"{record['id']:8} {record['axis']:9} {record['verdict']:11} "
              f"{record['severity']:4} {title}")
    print("-" * len(header))


def print_findings(records: list) -> None:
    reproduced = [r for r in records if r["verdict"] == "REPRODUCED"]
    if not reproduced:
        print("\nNo attack was reproduced.")
        return
    order = {"P0": 0, "P1": 1, "P2": 2, "P3": 3}
    reproduced.sort(key=lambda r: (order.get(r["severity"], 9), r["id"]))
    print(f"\nREPRODUCED findings ({len(reproduced)}):")
    for record in reproduced:
        print(f"  [{record['severity']}] {record['id']}  {record['title']}")
        print(f"        observed: {record['observed'][:400]}")


def write_markdown(records: list, path: Path, meta: dict) -> None:
    axes = {
        "identity": "Identity / receipt scope",
        "scope": "Scope (portee)",
        "git": "Git refs and CAS",
        "durable": "Durable journal and recovery",
        "files": "User files and hostile paths",
        "process": "Process containment",
        "proof": "Proof quality",
        "compat": "Compatibility",
        "future": "Future commands (absent at this commit)",
    }
    lines = ["<!-- generated by experiments/mvp-qa/attacks.py; do not edit by hand -->", ""]
    lines.append(f"Binary: `{meta['flow']['path']}` (sha256 "
                 f"`{meta['flow']['sha256'][:16]}`) — base commit "
                 f"`{meta['git']['base_sha'][:12]}`")
    lines.append("")
    for axis, title in axes.items():
        rows = [r for r in records if r["axis"] == axis]
        if not rows:
            continue
        lines.append(f"### {title}")
        lines.append("")
        lines.append("| ID | Attack | Command (excerpt) | Expected refusal | Observed | Verdict | Severity |")
        lines.append("|---|---|---|---|---|---|---|")
        for record in rows:
            cmd = record["commands"][-1]["argv"] if record["commands"] else "(no command)"
            cmd = cmd.replace("|", "\\|")
            observed = record["observed"].replace("|", "\\|").replace("\n", " ")[:240]
            expected = record["expected"].replace("|", "\\|").replace("\n", " ")[:200]
            lines.append(
                f"| {record['id']} | {record['title']} | `{cmd}` | {expected} | "
                f"{observed} | **{record['verdict']}** | {record['severity']} |")
        lines.append("")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(
        description="Adversarial harness for Ravum Flow MVP 0.1 (stdlib only).")
    parser.add_argument("--flow", required=True, help="path to the flow binary")
    parser.add_argument("--out", default="results.json", help="results JSON path")
    parser.add_argument("--filter", default="", help="only run ids starting with "
                                                     "this prefix / axis")
    parser.add_argument("--keep", action="store_true", help="keep fixtures")
    parser.add_argument("--markdown", default="", help="also write verdict tables "
                                                       "as Markdown")
    parser.add_argument("--base-sha", default="", help="override the recorded base SHA")
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args(argv)

    flow = Path(args.flow).resolve()
    if not flow.is_file():
        print(f"error: flow binary not found: {flow}", file=sys.stderr)
        return 2
    repo_root = Path(__file__).resolve().parents[2]
    base_sha = args.base_sha or git_capture(repo_root, "rev-parse", "HEAD") or "unknown"
    branch = git_capture(repo_root, "rev-parse", "--abbrev-ref", "HEAD")

    scenarios = SCENARIOS + FUTURE_SCENARIOS
    if args.filter:
        scenarios = [s for s in scenarios
                     if s.id.lower().startswith(args.filter.lower())
                     or s.axis.lower().startswith(args.filter.lower())]
        if not scenarios:
            print(f"error: filter {args.filter!r} matched no scenario", file=sys.stderr)
            return 2

    run_root = Path(tempfile.mkdtemp(prefix="flow-mvp-qa-"))
    started = time.time()
    records = []
    try:
        for index, scn in enumerate(scenarios, 1):
            if not args.quiet:
                print(f"[{index:02d}/{len(scenarios)}] {scn.id} {scn.title}",
                      file=sys.stderr)
            records.append(run_scenario(scn, run_root, flow, base_sha))
    finally:
        if not args.keep and not os.environ.get("FLOW_QA_KEEP", ""):
            safe_remove_tree(run_root)
        else:
            print(f"fixtures kept under {run_root}", file=sys.stderr)

    flow_version = run_process([str(flow), "version"], repo_root, dict(BASE_ENV), 30)
    try:
        flow_version_json = json.loads(flow_version.stdout)
    except Exception:
        flow_version_json = None
    meta = {
        "harness": {
            "version": "1.0",
            "python": platform.python_version(),
            "platform": platform.platform(),
            "git": git_capture(Path.cwd(), "--version") or "unknown",
            "started_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(started)),
            "duration_s": round(time.time() - started, 1),
            "run_root": str(run_root) if args.keep else "<removed>",
        },
        "flow": {
            "path": str(flow),
            "sha256": sha256_file(flow),
            "version": flow_version_json,
        },
        "git": {
            "repo": str(repo_root),
            "base_sha": base_sha,
            "branch": branch,
        },
    }
    summary = summarize(records)
    result = {"meta": meta, "summary": summary, "scenarios": records}
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n",
                   encoding="utf-8")

    print_table(records)
    print(f"\nSummary: {summary['total']} scenarios | "
          f"REPRODUCED {summary['REPRODUCED']} | REFUTED {summary['REFUTED']} | "
          f"NOT_RUN {summary['NOT_RUN']} | ERROR {summary['ERROR']}")
    for axis, counts in summary["by_axis"].items():
        print(f"  {axis:9} total {counts['total']:2}  reproduced "
              f"{counts['REPRODUCED']:2}  refuted {counts['REFUTED']:2}  "
              f"not_run {counts['NOT_RUN']:2}  error {counts['ERROR']}")
    print(f"reproduced by severity: {summary['reproduced_by_severity']}")
    print(f"results: {out}")
    print_findings(records)
    if args.markdown:
        write_markdown(records, Path(args.markdown), meta)
        print(f"markdown tables: {args.markdown}")

    p0 = [r for r in records
          if r["verdict"] == "REPRODUCED" and r["severity"] == "P0"]
    errors = [r for r in records if r["verdict"] == "ERROR"]
    if p0:
        print(f"\nFAIL: {len(p0)} reproduced P0: "
              + ", ".join(r["id"] for r in p0), file=sys.stderr)
        return 1
    if errors:
        print(f"\nHARNESS ERROR: {len(errors)} scenario(s) could not be evaluated: "
              + ", ".join(r["id"] for r in errors), file=sys.stderr)
        return 3
    print("\nOK: no reproduced P0.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
