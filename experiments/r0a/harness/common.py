"""R0-a harness — shared primitives.

Rules enforced here (see experiments/r0a/README.md):
  * every write happens under experiments/r0a/work/ ;
  * every child process gets a private git config, never the user's ;
  * cleanup only removes paths this harness created and inventoried.
"""
from __future__ import annotations

import hashlib
import json
import os
import shutil
import stat
import subprocess
import sys
import time
from dataclasses import dataclass, field
from pathlib import Path

HARNESS_DIR = Path(__file__).resolve().parent
R0A_DIR = HARNESS_DIR.parent
REPO_ROOT = R0A_DIR.parents[1]
WORK = R0A_DIR / "work"
FIXTURES = WORK / "fixtures"
LOGS = WORK / "logs"
OUT = R0A_DIR / "out"
PRIVATE_GITCONFIG = WORK / "gitconfig-private"
CANARY = "FLOW_FAKE_SECRET_CANARY_7d13c0ffee"


def ensure_layout() -> None:
    for p in (WORK, FIXTURES, LOGS, OUT):
        p.mkdir(parents=True, exist_ok=True)
    if not PRIVATE_GITCONFIG.exists():
        PRIVATE_GITCONFIG.write_text("", encoding="utf-8")


def _decode(b: bytes) -> str:
    return b.decode("utf-8", errors="replace")


def child_env(extra: dict[str, str] | None = None) -> dict[str, str]:
    """Environment for child git/python processes: no user gitconfig, private HOME."""
    env = {
        k: v for k, v in os.environ.items()
        if not k.startswith("GIT_")  # drop inherited git overrides (ENV-04 hygiene)
    }
    env["GIT_CONFIG_NOSYSTEM"] = "1"
    env["GIT_CONFIG_GLOBAL"] = str(PRIVATE_GITCONFIG)
    env["GIT_CONFIG_SYSTEM"] = str(PRIVATE_GITCONFIG)
    env["GIT_TERMINAL_PROMPT"] = "0"
    env["HOME"] = str(PRIVATE_GITCONFIG.parent)
    env["USERPROFILE"] = str(PRIVATE_GITCONFIG.parent)
    env["LC_ALL"] = "C"
    env["PYTHONIOENCODING"] = "utf-8"
    if extra:
        env.update(extra)
    return env


def run(cmd: list[str] | str, cwd: Path | None = None, env: dict | None = None,
        timeout: float = 120.0, check: bool = True, stdin_text: str | None = None):
    """Run one child with captured bytes. Returns CompletedProcess with str streams.

    Limitation, measured and documented: on timeout only the direct child is
    killed, not a full job object (RESOURCE-14 target of R0-b).
    """
    started = time.perf_counter()
    use_shell = isinstance(cmd, str)
    try:
        cp = subprocess.run(
            cmd, cwd=str(cwd) if cwd else None,
            env=env if env is not None else child_env(),
            input=(stdin_text.encode() if stdin_text else None),
            stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            timeout=timeout, shell=use_shell, check=False,
        )
        timed_out = False
    except subprocess.TimeoutExpired as exc:
        cp = subprocess.CompletedProcess(cmd, 124, exc.stdout or b"", (exc.stderr or b"") + b"\n[TIMEOUT]")
        timed_out = True
    elapsed = time.perf_counter() - started
    cp.stdout = _decode(cp.stdout or b"")
    cp.stderr = _decode(cp.stderr or b"")
    cp.elapsed_s = elapsed
    cp.timed_out = timed_out
    if check and (cp.returncode != 0 or timed_out):
        raise RuntimeError(
            f"command failed ({cp.returncode}): {cmd!r}\nstdout:\n{cp.stdout}\nstderr:\n{cp.stderr}")
    return cp


def git(repo: Path, *args: str, env: dict | None = None, check: bool = True,
        timeout: float = 120.0, stdin_text: str | None = None):
    return run(["git", "-C", str(repo), *args], env=env, check=check,
               timeout=timeout, stdin_text=stdin_text)


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def write_file(path: Path, data: bytes | str) -> dict:
    path.parent.mkdir(parents=True, exist_ok=True)
    raw = data.encode() if isinstance(data, str) else data
    path.write_bytes(raw)
    return {"path": str(path), "bytes": len(raw), "sha256": sha256_bytes(raw)}


def init_repo(path: Path, *, filemode: bool = True) -> Path:
    path.mkdir(parents=True, exist_ok=True)
    git(path, "init", "-q", "-b", "main")
    git(path, "config", "user.name", "Flow R0 Fixture")
    git(path, "config", "user.email", "fixture@example.invalid")
    git(path, "config", "commit.gpgsign", "false")
    git(path, "config", "core.autocrlf", "false")
    git(path, "config", "core.filemode", "true" if filemode else "false")
    git(path, "config", "core.symlinks", "true")
    git(path, "config", "advice.detachedHead", "false")
    return path


def commit_all(repo: Path, message: str, *, allow_empty: bool = False) -> str:
    git(repo, "add", "-A")
    args = ["commit", "-q", "-m", message]
    if allow_empty:
        args.insert(1, "--allow-empty")
    git(repo, *args)
    return git(repo, "rev-parse", "HEAD").stdout.strip()


def head_oid(repo: Path) -> str:
    return git(repo, "rev-parse", "HEAD").stdout.strip()


def tree_oid(repo: Path, rev: str = "HEAD") -> str:
    return git(repo, "rev-parse", f"{rev}^{{tree}}").stdout.strip()


def git_tree_listing(repo: Path, rev: str = "HEAD") -> list[dict]:
    """Canonical content identity: every blob mode/oid/path of a Git tree."""
    out = git(repo, "ls-tree", "-r", "-z", "--full-tree", rev).stdout
    entries = []
    for record in out.split("\0"):
        if not record:
            continue
        meta, path = record.split("\t", 1)
        mode, kind, oid = meta.split(" ")
        entries.append({"mode": mode, "type": kind, "oid": oid, "path": path})
    entries.sort(key=lambda e: e["path"])
    return entries


def tree_digest(repo: Path, rev: str = "HEAD") -> dict:
    entries = git_tree_listing(repo, rev)
    payload = json.dumps(entries, sort_keys=True).encode()
    reproducible = sha256_bytes(payload)
    return {"rev": rev, "oid": tree_oid(repo, rev), "entries": len(entries),
            "digest": reproducible, "listing": entries}


def worktree_digest(root: Path, *, skip_dirs: tuple[str, ...] = (".git",)) -> dict:
    """Byte-level digest of the working tree, including mode and symlink targets."""
    rows, total = [], 0
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(d for d in dirnames if d not in skip_dirs)
        for name in sorted(filenames):
            p = Path(dirpath) / name
            if p.is_symlink():
                target = os.readlink(p)
                rows.append({"path": p.relative_to(root).as_posix(), "kind": "link",
                             "target": target, "sha256": sha256_bytes(target.encode())})
                continue
            try:
                st = p.stat()
                raw = p.read_bytes()
            except OSError as exc:  # locked/unreadable file is data, not a crash
                rows.append({"path": p.relative_to(root).as_posix(), "kind": "unreadable",
                             "error": str(exc)})
                continue
            total += len(raw)
            rows.append({"path": p.relative_to(root).as_posix(),
                         "kind": "exec" if (st.st_mode & 0o111) else "file",
                         "bytes": len(raw), "sha256": sha256_bytes(raw)})
    rows.sort(key=lambda r: r["path"])
    return {"files": len(rows), "bytes": total,
            "digest": sha256_bytes(json.dumps(rows, sort_keys=True).encode()),
            "rows": rows}


def status_porcelain(repo: Path) -> list[str]:
    return [ln for ln in git(repo, "status", "--porcelain=v1", "-uall").stdout.splitlines() if ln]


@dataclass
class Fixture:
    name: str
    path: Path
    description: str
    commit: str = ""
    tree: str = ""
    tree_digest: str = ""
    worktree_digest: str = ""
    status: list[str] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)
    sub_cases: dict = field(default_factory=dict)

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "path": self.path.relative_to(REPO_ROOT).as_posix(),
            "description": self.description,
            "commit": self.commit,
            "tree": self.tree,
            "tree_digest": self.tree_digest,
            "worktree_digest": self.worktree_digest,
            "status": self.status,
            "notes": self.notes,
            "sub_cases": self.sub_cases,
        }


def finalize(fixture: Fixture, repo: Path, notes: list[str] | None = None) -> Fixture:
    td = tree_digest(repo)
    fixture.commit = head_oid(repo)
    fixture.tree = td["oid"]
    fixture.tree_digest = td["digest"]
    fixture.worktree_digest = worktree_digest(repo)["digest"]
    fixture.status = status_porcelain(repo)
    if notes:
        fixture.notes.extend(notes)
    return fixture


def finalize_multi(fixture: Fixture, root: Path, notes: list[str] | None = None) -> Fixture:
    """For fixtures that own several repositories: digest the directory, no single HEAD."""
    rows = worktree_digest(root, skip_dirs=(".git",))["rows"]
    fixture.worktree_digest = sha256_bytes(json.dumps(rows, sort_keys=True).encode())
    if notes:
        fixture.notes.extend(notes)
    return fixture


INVENTORY = WORK / "created-paths.json"


def inventory_add(paths: list[Path]) -> None:
    existing = json.loads(INVENTORY.read_text()) if INVENTORY.exists() else []
    for p in paths:
        rel = p.resolve().relative_to(WORK.resolve()).as_posix()
        if rel not in existing:
            existing.append(rel)
    INVENTORY.write_text(json.dumps(sorted(existing), indent=2))


def _remove_readonly(func, path, exc_info) -> None:
    """Git objects are read-only on Windows; clear the bit then retry once."""
    try:
        os.chmod(path, stat.S_IWRITE | stat.S_IREAD)
        func(path)
    except OSError:
        pass


def _rmtree(path: Path) -> None:
    shutil.rmtree(path, onexc=_remove_readonly)


def cleanup(force_all: bool = False) -> dict:
    """Remove only inventoried paths under WORK. Never touches anything else."""
    ensure_layout()
    removed, skipped = [], []
    targets = json.loads(INVENTORY.read_text()) if INVENTORY.exists() else []
    if force_all:
        targets = [p.relative_to(WORK).as_posix() for p in sorted(FIXTURES.iterdir())] if FIXTURES.exists() else []
    for rel in targets:
        target = (WORK / rel).resolve()
        if WORK.resolve() not in target.parents and target != WORK.resolve():
            skipped.append({"path": rel, "reason": "outside_work_root"})
            continue
        if target.is_dir():
            _rmtree(target)
        elif target.exists():
            target.unlink()
        removed.append(rel)
    INVENTORY.write_text("[]")
    return {"removed": removed, "skipped": skipped}


def manifest_environment() -> dict:
    def ver(cmd: list[str]) -> str:
        try:
            cp = run(cmd, check=False, timeout=20)
            return (cp.stdout or cp.stderr).strip().splitlines()[0] if (cp.stdout or cp.stderr) else "UNKNOWN"
        except Exception as exc:  # toolchain absent is data
            return f"NOT_AVAILABLE: {exc}"
    return {
        "os": sys.platform,
        "platform": os.environ.get("OS", "Windows"),
        "python": sys.version.split()[0],
        "git": ver(["git", "--version"]),
        "cargo": ver(["cargo", "--version"]),
        "rustc": ver(["rustc", "--version"]),
        "node": ver(["node", "--version"]),
        "cpu_count": os.cpu_count(),
        "repo_root": ".",
        "canary_label": CANARY.split("_7")[0],
    }
