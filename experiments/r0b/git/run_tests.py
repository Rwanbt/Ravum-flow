#!/usr/bin/env python3
"""R0-b Git mechanics tests (agent A) — native Git only, no network, no Ravum.

Everything runs inside experiments/r0b/git/. The R0-a fixtures were absent at
execution time (experiments/r0a/work/fixtures was empty, manifest stale), so the
needed fixtures are rebuilt here from the documented scenarios of
experiments/r0a/harness/fixtures.py. `python experiments/r0a/run_r0a.py build`
is deliberately NOT invoked (forbidden by the mission).

Each test writes raw evidence to logs/<ID>.log and one record to RESULTS.json.
No claim of security is made: a passing check is not a proof.
"""
from __future__ import annotations

import json
import os
import platform
import re
import shutil
import stat as statmod
import subprocess
import sys
import time
from pathlib import Path

BASE = Path(__file__).resolve().parent            # experiments/r0b/git
ROOT = BASE.parents[2]                            # repo root
FIX = BASE / "fixtures"
SCRATCH = BASE / "scratch"
LOGS = BASE / "logs"
PRIVATE = SCRATCH / "private"
GITCONFIG = PRIVATE / "gitconfig"


def _git_version() -> str:
    try:
        return subprocess.run(["git", "--version"], capture_output=True, text=True).stdout.strip()
    except OSError:
        return "git: unknown"


GIT_VERSION = _git_version()
OS_LABEL = f"{platform.system()} {platform.release()} ({sys.platform}); {GIT_VERSION}"


def child_env(extra: dict[str, str] | None = None) -> dict[str, str]:
    """Isolated child environment: no user/system gitconfig, private HOME."""
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    env["GIT_CONFIG_NOSYSTEM"] = "1"
    env["GIT_CONFIG_GLOBAL"] = str(GITCONFIG)
    env["GIT_CONFIG_SYSTEM"] = str(GITCONFIG)
    env["GIT_TERMINAL_PROMPT"] = "0"
    env["HOME"] = str(PRIVATE)
    env["USERPROFILE"] = str(PRIVATE)
    env["LC_ALL"] = "C"
    env["PYTHONIOENCODING"] = "utf-8"
    if extra:
        env.update(extra)
    return env


ENV = child_env()
PY = sys.executable


def redact(text: str) -> str:
    if not text:
        return ""
    text = text.replace(str(ROOT), "<REPO>")
    text = text.replace(str(ROOT).replace("\\", "/"), "<REPO>")
    home = str(Path.home())
    text = text.replace(home, "<HOME>")
    text = re.sub(r"[A-Za-z]:\\Users\\[^\\\s\"']+", "<HOME>", text)
    text = re.sub(r"[A-Za-z]:/Users/[^/\s\"']+", "<HOME>", text)
    return text


def run(cmd, cwd: Path, stdin_text: str | None = None, timeout: float = 120.0, env=None):
    argv = [str(c) for c in cmd]
    try:
        cp = subprocess.run(
            argv,
            cwd=str(cwd),
            env=env if env is not None else ENV,
            input=(stdin_text.encode() if stdin_text is not None else None),
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout,
            shell=False,
        )
        out = cp.stdout.decode("utf-8", "replace")
        err = cp.stderr.decode("utf-8", "replace")
        rc = cp.returncode
        timed_out = False
    except subprocess.TimeoutExpired as exc:
        out = (exc.stdout or b"").decode("utf-8", "replace")
        err = ((exc.stderr or b"").decode("utf-8", "replace")) + "\n[TIMEOUT]"
        rc, timed_out = 124, True
    return {"rc": rc, "out": out, "err": err, "argv": argv, "cwd": cwd, "timeout": timed_out}


def git(repo: Path, *args: str, stdin_text: str | None = None, timeout: float = 120.0):
    return run(["git", "-C", str(repo), *args], cwd=repo, stdin_text=stdin_text, timeout=timeout)


def git_out(repo: Path, *args: str, stdin_text: str | None = None) -> str:
    cp = git(repo, *args, stdin_text=stdin_text)
    if cp["rc"] != 0:
        raise RuntimeError(f"git {' '.join(args)} failed rc={cp['rc']}: {cp['err']}")
    return cp["out"].strip()


def write_file(path: Path, data) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if isinstance(data, bytes):
        path.write_bytes(data)
    else:
        path.write_text(data, encoding="utf-8", newline="\n")


def read_file(path: Path) -> bytes:
    return path.read_bytes()


def sha256_file(path: Path) -> str:
    import hashlib

    return hashlib.sha256(path.read_bytes()).hexdigest()


def _rmtree_onexc(func, path, _exc):
    try:
        os.chmod(path, statmod.S_IWRITE)
        func(path)
    except OSError:
        pass


def rmtree(path: Path) -> None:
    if path.exists():
        shutil.rmtree(path, onexc=_rmtree_onexc)


def copytree(src: Path, dst: Path) -> Path:
    rmtree(dst)
    shutil.copytree(src, dst)
    return dst


# --------------------------------------------------------------------------
# raw-log case collector
# --------------------------------------------------------------------------


class Case:
    def __init__(self, tid: str, fixture: str, scope: str):
        self.tid = tid
        self.fixture = fixture
        self.scope = scope
        self.commands: list[str] = []
        self.blocks: list[str] = []
        self.extra: dict = {}
        self.started = time.time()

    def run(self, cmd, cwd: Path, stdin_text: str | None = None, timeout: float = 120.0):
        cp = run(cmd, cwd, stdin_text=stdin_text, timeout=timeout)
        shown = redact(" ".join(cp["argv"]))
        self.commands.append(shown)
        self.blocks.append(
            "$ {cmd}\n  cwd={cwd}\n  rc={rc}\n  stdout:\n{out}\n  stderr:\n{err}".format(
                cmd=shown,
                cwd=redact(str(cp["cwd"])),
                rc=cp["rc"],
                out=redact(cp["out"]).rstrip() or "  <empty>",
                err=redact(cp["err"]).rstrip() or "  <empty>",
            )
        )
        return cp

    def note(self, text: str) -> None:
        self.blocks.append(f"# {redact(str(text))}")

    def facts(self, **kwargs) -> None:
        self.extra.update(kwargs)

    def finish(self, verdict: str, observation: str, evidence: str | None = None) -> dict:
        elapsed = round(time.time() - self.started, 3)
        LOGS.mkdir(parents=True, exist_ok=True)
        log_path = LOGS / f"{self.tid}.log"
        header = (
            f"TEST {self.tid}\nverdict={verdict}\nfixture={self.fixture}\nos={OS_LABEL}\n"
            f"scope={self.scope}\nelapsed_s={elapsed}\n" + "=" * 78 + "\n"
        )
        body = "\n\n".join(self.blocks)
        facts = "\n".join(f"{k}={redact(repr(v))}" for k, v in self.extra.items())
        log_path.write_text(header + body + "\n" + "=" * 78 + "\nfacts:\n" + facts + "\n", encoding="utf-8")
        return {
            "id": self.tid,
            "verdict": verdict,
            "fixture": self.fixture,
            "os": OS_LABEL,
            "commands": self.commands,
            "observation": redact(observation),
            "scope": self.scope,
            "evidence": evidence or f"logs/{self.tid}.log",
        }


# --------------------------------------------------------------------------
# fixtures (local replicas of the documented R0-a scenarios)
# --------------------------------------------------------------------------

HOOK_ORDER = """#!/bin/sh
LOG="$(git rev-parse --git-dir)/hook-order.log"
printf '%s\\n' "pre-commit" >> "$LOG"
exit 0
"""
HOOK_PREPARE = """#!/bin/sh
LOG="$(git rev-parse --git-dir)/hook-order.log"
printf '%s\\n' "prepare-commit-msg" >> "$LOG"
printf '%s\\n' "prepared-by-hook" >> "$1"
exit 0
"""
HOOK_COMMITMSG = """#!/bin/sh
LOG="$(git rev-parse --git-dir)/hook-order.log"
printf '%s\\n' "commit-msg" >> "$LOG"
if grep -q 'REJECT-FIXTURE' "$1"; then
  printf '%s\\n' "commit-msg refused the message" >&2
  exit 1
fi
exit 0
"""
HOOK_POST = """#!/bin/sh
LOG="$(git rev-parse --git-dir)/hook-order.log"
printf '%s\\n' "post-commit" >> "$LOG"
exit 0
"""
HOOK_REFUSE = """#!/bin/sh
printf '%s\\n' "pre-commit deliberately refused" >&2
exit 1
"""
HOOK_MUTATE = """#!/bin/sh
printf '%s\\n' "hook-mutated" >> extra.txt
git add extra.txt
printf '%s\\n' "pre-commit mutated the index and the worktree"
exit 0
"""
HOOK_ISOLATION = """#!/bin/sh
DIR="$(git rev-parse --git-dir)"
git rev-parse --show-toplevel >> "$DIR/hook-cwd.log"
printf '%s\\n' "hooked" >> dirty.txt
exit 0
"""

PUBLISHER = r'''"""Minimal intent -> CAS -> outcome publisher with driven crash points.

Copied verbatim from experiments/r0a/harness/fixtures.py (PUBLISHER). Used only
inside a disposable fixture repository; it models the journal protocol under
test, not a Flow implementation.
"""
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
    parent = old
    commit = git(repo, "commit-tree", tree, "-p", parent, "-m", payload).stdout.strip()

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


def init_repo(path: Path, filemode: bool = True) -> Path:
    path.mkdir(parents=True, exist_ok=True)
    git(path, "init", "-q", "-b", "main")
    git(path, "config", "user.name", "Flow R0 Fixture")
    git(path, "config", "user.email", "fixture@example.invalid")
    git(path, "config", "commit.gpgsign", "false")
    git(path, "config", "core.autocrlf", "false")
    git(path, "config", "core.filemode", "true" if filemode else "false")
    git(path, "config", "core.symlinks", "true")
    return path


def commit_all(repo: Path, message: str, allow_empty: bool = False) -> str:
    git(repo, "add", "-A")
    args = ["commit", "-q", "-m", message]
    if allow_empty:
        args.insert(1, "--allow-empty")
    cp = git(repo, *args)
    if cp["rc"] != 0:
        raise RuntimeError(f"commit failed: {cp['err']}")
    return git_out(repo, "rev-parse", "HEAD")


def install_hooks(repo: Path, hooks: dict[str, str]) -> Path:
    raw = git_out(repo, "rev-parse", "--git-path", "hooks")
    hook_dir = Path(raw)
    if not hook_dir.is_absolute():
        hook_dir = (repo / hook_dir).resolve()
    hook_dir.mkdir(parents=True, exist_ok=True)
    for name, body in hooks.items():
        p = hook_dir / name
        p.write_bytes(body.replace("\r\n", "\n").encode())
        try:
            p.chmod(0o755)
        except OSError:
            pass
        # Git for Windows resolves a POSIX shebang through its own sh; keep a
        # .sh twin exactly like the R0-a fixture did.
        (hook_dir / f"{name}.sh").write_bytes(p.read_bytes())
    return hook_dir


def tree_of(repo: Path, rev: str = "HEAD") -> str:
    return git_out(repo, "rev-parse", f"{rev}^{{tree}}")


def head_of(repo: Path) -> str:
    return git_out(repo, "rev-parse", "HEAD")


def ref_of(repo: Path, ref: str) -> str | None:
    cp = git(repo, "rev-parse", "--verify", ref)
    return cp["out"].strip() if cp["rc"] == 0 else None


def commit_count(repo: Path) -> int:
    return int(git_out(repo, "rev-list", "--all", "--count"))


def build_tree_imports() -> None:
    repo = FIX / "tree-imports"
    init_repo(repo)
    write_file(repo / ".gitignore", "gen/\n*.log\n")
    write_file(repo / "test_app.py", (
        "import unittest\n\n"
        "import local_config\n"
        "from gen.values import FLAG\n\n\n"
        "class TestApp(unittest.TestCase):\n"
        "    def test_value(self):\n"
        "        self.assertEqual(local_config.VALUE, 42)\n"
        "        self.assertEqual(FLAG, 'ok')\n\n\n"
        "if __name__ == '__main__':\n"
        "    unittest.main()\n"
    ))
    write_file(repo / "README.md", "# tree-imports fixture\n")
    commit_all(repo, "fixture: app + test, dependency absent from the tree")
    # dependency the test needs, deliberately untracked (TREE-01a)
    write_file(repo / "local_config.py", "VALUE = 42\n")
    # dependency the test needs, deliberately ignored (TREE-01b)
    write_file(repo / "gen" / "values.py", "FLAG = 'ok'\n")
    write_file(repo / "build.log", "ignored log\n")


def build_tree_mutation() -> None:
    repo = FIX / "tree-mutation"
    init_repo(repo)
    write_file(repo / "calc.py", "def value():\n    return 2\n")
    write_file(repo / "test_calc.py", (
        "import unittest\n\n"
        "import calc\n\n\n"
        "class TestCalc(unittest.TestCase):\n"
        "    def test_value(self):\n"
        "        self.assertEqual(calc.value(), 3)\n\n\n"
        "if __name__ == '__main__':\n"
        "    unittest.main()\n"
    ))
    commit_all(repo, "fixture: calc returns 2, test expects 3 (red as committed)")


def build_hooks() -> None:
    root = FIX / "f2-hooks"
    for name, hooks in (
        ("order", {"pre-commit": HOOK_ORDER, "prepare-commit-msg": HOOK_PREPARE,
                   "commit-msg": HOOK_COMMITMSG, "post-commit": HOOK_POST}),
        ("refuse", {"pre-commit": HOOK_REFUSE}),
        ("mutate", {"pre-commit": HOOK_MUTATE}),
    ):
        repo = root / name
        init_repo(repo)
        write_file(repo / "seed.txt", "seed\n")
        commit_all(repo, "fixture: hook variant seed")
        install_hooks(repo, hooks)
        write_file(repo / "work.txt", "staged by experiment\n")
        git(repo, "add", "work.txt")


def build_hook_isolation() -> None:
    repo = FIX / "hook-isolate"
    init_repo(repo)
    write_file(repo / "seed.txt", "seed\n")
    write_file(repo / "dirty.txt", "clean committed content\n")
    commit_all(repo, "fixture: isolation seed")
    install_hooks(repo, {"pre-commit": HOOK_ISOLATION})
    # user's dirty (unstaged) file, plus one staged file so a commit happens
    write_file(repo / "dirty.txt", "dirty user edit\n")
    write_file(repo / "work.txt", "staged by experiment\n")
    git(repo, "add", "work.txt")


def build_filetypes() -> None:
    root = FIX / "f3-filetypes"
    # base
    base = root / "base"
    init_repo(base)
    write_file(base / ".gitattributes", "*.txt text eol=crlf\n*.bin -text\n")
    write_file(base / "a.txt", "alpha\n")
    write_file(base / "crlf.txt", b"line one\r\nline two\r\n")
    write_file(base / "b.bin", bytes(range(0, 32)) + b"\x00\xff")
    write_file(base / "script.sh", "#!/bin/sh\necho fixture\n")
    write_file(base / "keep.txt", "to be renamed\n")
    write_file(base / "delete.txt", "to be deleted\n")
    git(base, "add", "-A")
    git(base, "update-index", "--chmod=+x", "script.sh")
    git(base, "commit", "-q", "-m", "fixture: base file types")
    git(base, "mv", "keep.txt", "renamed.txt")
    git(base, "rm", "-q", "delete.txt")
    git(base, "commit", "-q", "-m", "fixture: rename and delete")
    # link (120000 entry, created through plumbing)
    link = root / "link"
    init_repo(link)
    write_file(link / "target.txt", "target content\n")
    commit_all(link, "fixture: link target")
    oid = git(link, "hash-object", "-w", "--stdin", stdin_text="target.txt\n")["out"].strip()
    git(link, "update-index", "--add", "--cacheinfo", f"120000,{oid},link.txt")
    git(link, "commit", "-q", "-m", "fixture: symlink entry")
    # case collision (index only)
    case = root / "case"
    init_repo(case, filemode=False)
    write_file(case / "seed.txt", "seed\n")
    commit_all(case, "fixture: case base")
    for name, content in (("Case.txt", "upper\n"), ("case.txt", "lower\n")):
        write_file(case / "tmp-name.txt", content)
        oid = git(case, "hash-object", "-w", "tmp-name.txt")["out"].strip()
        git(case, "update-index", "--add", "--cacheinfo", f"100644,{oid},{name}")
        (case / "tmp-name.txt").unlink()
    git(case, "commit", "-q", "-m", "fixture: case-ambiguous entries")


def build_crash() -> None:
    repo = FIX / "f5-crash"
    init_repo(repo)
    write_file(repo / "publisher.py", PUBLISHER)
    write_file(repo / "state.txt", "v1\n")
    commit_all(repo, "fixture: crash publisher seed")
    # a real staged change so `write-tree` differs from HEAD^{tree}
    write_file(repo / "state.txt", "v2\n")
    git(repo, "add", "state.txt")


def build_generic(name: str) -> Path:
    repo = FIX / name
    init_repo(repo)
    write_file(repo / "seed.txt", "seed\n")
    commit_all(repo, "fixture: seed")
    return repo


def build_all_fixtures() -> None:
    rmtree(FIX)
    FIX.mkdir(parents=True, exist_ok=True)
    build_tree_imports()
    build_tree_mutation()
    build_hooks()
    build_hook_isolation()
    build_filetypes()
    build_crash()
    build_generic("gitdir-main")
    build_generic("refcas")
    build_generic("fence-a")
    build_generic("fence-b")


# --------------------------------------------------------------------------
# journal recovery helper (models a disposable writer's recovery, not Flow)
# --------------------------------------------------------------------------


def git_dir(repo: Path) -> Path:
    raw = git_out(repo, "rev-parse", "--git-dir")
    p = Path(raw)
    return p if p.is_absolute() else (repo / p).resolve()


def journal_path(repo: Path) -> Path:
    return git_dir(repo) / "flow-journal.jsonl"


def read_journal(repo: Path) -> list[dict]:
    p = journal_path(repo)
    if not p.exists():
        return []
    return [json.loads(line) for line in p.read_text(encoding="utf-8").splitlines() if line.strip()]


def append_journal(repo: Path, record: dict) -> None:
    p = journal_path(repo)
    with open(p, "a", encoding="utf-8") as fh:
        fh.write(json.dumps(record, sort_keys=True) + "\n")
        fh.flush()
        os.fsync(fh.fileno())


def recover_guard(repo: Path) -> dict:
    """Idempotent recovery: classify from the journal + ref, never publish.

    Returns {state, ref, intent, outcome_written, ref_after}. The guard must not
    call update-ref; the caller asserts the ref is unchanged.
    """
    records = read_journal(repo)
    intents = [r for r in records if r.get("event") == "intent"]
    outcomes = [r for r in records if r.get("event") == "outcome"]
    ref = ref_of(repo, "refs/heads/main")
    ref_before = ref
    if not intents:
        return {"state": "NOT_STARTED", "ref": ref, "intent": None,
                "outcome_written": False, "ref_after": ref}
    intent = intents[-1]
    published_outcomes = [o for o in outcomes if o.get("status") == "PUBLISHED"]
    outcome_written = False
    if published_outcomes:
        state = "PUBLISHED"
    elif ref == intent["new"]:
        state = "PUBLISHED"
        append_journal(repo, {"event": "outcome", "status": "RECOVERED_PUBLISHED",
                              "new": intent["new"], "old": intent["old"]})
        outcome_written = True
    elif ref == intent["old"]:
        state = "NOT_PUBLISHED"
    else:
        state = "UNKNOWN_OUTCOME"
    return {"state": state, "ref": ref, "intent": intent,
            "outcome_written": outcome_written, "ref_after": ref_of(repo, "refs/heads/main"),
            "ref_before": ref_before}


# --------------------------------------------------------------------------
# tests
# --------------------------------------------------------------------------


def test_tree_01() -> dict:
    repo = FIX / "tree-imports"
    c = Case(
        "TREE-01",
        "r0b-local:tree-imports (R0-a fixtures absent)",
        "Git 2.51.1 for Windows, local NTFS, native Python unittest. Does not cover Linux, "
        "other languages, or Ravum's own verification path.",
    )
    status = c.run(["git", "-C", str(repo), "status", "--porcelain"], cwd=repo)
    ignored = c.run(["git", "-C", str(repo), "check-ignore", "-v", "gen/values.py"], cwd=repo)
    dirty = c.run([PY, "-m", "unittest", "-q", "test_app"], cwd=repo)
    clean = SCRATCH / "tree01-clean"
    rmtree(clean)
    add = c.run(["git", "-C", str(repo), "worktree", "add", "--detach", str(clean), "HEAD"], cwd=repo)
    clean_rc = None
    if add["rc"] == 0:
        clean_rc = c.run([PY, "-m", "unittest", "-q", "test_app"], cwd=clean)
    c.facts(
        dirty_rc=dirty["rc"],
        clean_rc=None if clean_rc is None else clean_rc["rc"],
        untracked_present=(repo / "local_config.py").exists(),
        ignored_present=(repo / "gen" / "values.py").exists(),
        clean_has_config=(clean / "local_config.py").exists() if clean.exists() else None,
        clean_has_gen=(clean / "gen" / "values.py").exists() if clean.exists() else None,
        tree_entries=git_out(repo, "ls-tree", "-r", "--name-only", "HEAD").splitlines(),
    )
    reproduced = (
        dirty["rc"] == 0
        and clean_rc is not None
        and clean_rc["rc"] != 0
        and "?? local_config.py" in status["out"]
        and ignored["rc"] == 0
    )
    obs = (
        "Le test passe dans le worktree (rc=0) parce qu'il importe `local_config.py` non suivi et "
        "`gen/values.py` ignoré ; le même test échoue (rc!=0) dans un export propre du tree via "
        "`git worktree add --detach HEAD`. Le tree OID ne contient aucune de ces deux entrées."
        if reproduced else
        "Comportement inattendu : voir le log (le test ne dépend pas du fichier dirty/ignoré comme attendu)."
    )
    return c.finish("REPRODUCED" if reproduced else "NOT_REPRODUCED", obs)


def test_tree_02() -> dict:
    src = FIX / "tree-mutation"
    repo = copytree(src, SCRATCH / "tree02")
    c = Case(
        "TREE-02",
        "r0b-local:tree-mutation (R0-a fixtures absent)",
        "Git 2.51.1 for Windows, local NTFS. Demonstrates the local false-receipt window only; "
        "does not model Flow's receipt format.",
    )
    t_before = tree_of(repo)
    hash_before = sha256_file(repo / "calc.py")
    c.note(f"tree OID before mutation: {t_before}")
    c.note(f"calc.py sha256 before mutation: {hash_before}")
    # mutate the worktree only: calc.value() now returns 3 (the test expectation)
    write_file(repo / "calc.py", "def value():\n    return 3\n")
    t_after = tree_of(repo)
    hash_after = sha256_file(repo / "calc.py")
    status = c.run(["git", "-C", str(repo), "status", "--porcelain"], cwd=repo)
    diff = c.run(["git", "-C", str(repo), "diff", "--stat"], cwd=repo)
    dirty = c.run([PY, "-m", "unittest", "-q", "test_calc"], cwd=repo)
    clean = SCRATCH / "tree02-clean"
    rmtree(clean)
    add = c.run(["git", "-C", str(repo), "worktree", "add", "--detach", str(clean), "HEAD"], cwd=repo)
    clean_rc = c.run([PY, "-m", "unittest", "-q", "test_calc"], cwd=clean) if add["rc"] == 0 else None
    c.facts(tree_before=t_before, tree_after=t_after, calc_sha_before=hash_before,
            calc_sha_after=hash_after, dirty_rc=dirty["rc"],
            clean_rc=None if clean_rc is None else clean_rc["rc"],
            status=status["out"].strip(),
            tree_calc_blob=git_out(repo, "cat-file", "blob", "HEAD:calc.py"))
    reproduced = (
        t_before == t_after
        and hash_before != hash_after
        and dirty["rc"] == 0
        and clean_rc is not None
        and clean_rc["rc"] != 0
        and " M calc.py" in status["out"]
    )
    obs = (
        f"Tree OID inchangé ({t_before}) alors que `calc.py` est muté dans le worktree "
        f"(sha256 {hash_before[:12]}→{hash_after[:12]}) : le test passe sur les octets mutés (rc=0) "
        "mais échoue sur le tree exporté (rc!=0). Un reçu portant ce tree OID serait donc faux."
        if reproduced else
        "Écart non démontré dans les conditions du test ; voir le log."
    )
    return c.finish("REPRODUCED" if reproduced else "NOT_REPRODUCED", obs)


def test_hook_02a() -> dict:
    repo = copytree(FIX / "f2-hooks" / "mutate", SCRATCH / "hook02a")
    c = Case(
        "HOOK-02a",
        "r0b-local:f2-hooks/mutate (R0-a fixtures absent)",
        "Git 2.51.1 native `git commit` only. Ravum's inverted order is CODE_READ from the "
        "documentary phase and was NOT executed here.",
    )
    head_before = head_of(repo)
    c.note(f"HEAD before: {head_before}")
    c.note(f"index before: {git_out(repo, 'diff', '--cached', '--name-only')!r}")
    commit = c.run(["git", "-C", str(repo), "commit", "-q", "-m", "hook mutate"], cwd=repo)
    head_after = head_of(repo) if commit["rc"] == 0 else None
    tree_list = c.run(["git", "-C", str(repo), "ls-tree", "-r", "--name-only", "HEAD"], cwd=repo)
    show = c.run(["git", "-C", str(repo), "show", "--stat", "--oneline", "--format=%H %s", "HEAD"], cwd=repo)
    extra_blob = None
    if head_after:
        cp = c.run(["git", "-C", str(repo), "cat-file", "blob", "HEAD:extra.txt"], cwd=repo)
        extra_blob = cp["out"] if cp["rc"] == 0 else None
    c.facts(head_before=head_before, head_after=head_after,
            extra_in_tree="extra.txt" in tree_list["out"].split(),
            extra_blob=extra_blob, commit_rc=commit["rc"])
    reproduced = (
        commit["rc"] == 0
        and "extra.txt" in tree_list["out"].split()
        and "hook-mutated" in (extra_blob or "")
    )
    obs = (
        "Avec `git commit` natif, le fichier ajouté à l'index par `pre-commit` (`git add extra.txt`) "
        "EST dans le tree committé : l'arbre est écrit après les hooks. C'est exactement ce que le "
        "plumbing de Ravum (write-tree avant hooks) ne peut pas produire."
        if reproduced else
        "Le staging du hook n'apparaît pas dans le commit ; voir le log."
    )
    return c.finish("REPRODUCED" if reproduced else "DISPROVED", obs)


def test_hook_02b() -> dict:
    repo = copytree(FIX / "hook-isolate", SCRATCH / "hook02b")
    c = Case(
        "HOOK-02b",
        "r0b-local:hook-isolate (R0-a fixtures absent)",
        "Git 2.51.1 native `git commit` on Windows. Shows absence of hook isolation; does not "
        "cover a sandboxed runner or a disposable worktree copy.",
    )
    dirty_before = read_file(repo / "dirty.txt")
    c.note(f"dirty.txt before: {dirty_before!r}")
    commit = c.run(["git", "-C", str(repo), "commit", "-q", "-m", "hook isolation"], cwd=repo)
    dirty_after = read_file(repo / "dirty.txt")
    cwd_log = git_dir(repo) / "hook-cwd.log"
    hook_cwd = cwd_log.read_text(encoding="utf-8").strip() if cwd_log.exists() else ""
    status = c.run(["git", "-C", str(repo), "status", "--porcelain"], cwd=repo)
    committed = c.run(["git", "-C", str(repo), "show", "--name-only", "--format=", "HEAD"], cwd=repo)
    c.facts(commit_rc=commit["rc"], dirty_before=dirty_before, dirty_after=dirty_after,
            hook_cwd=hook_cwd, status=status["out"].strip(),
            committed=committed["out"].split())
    mutated = dirty_after != dirty_before
    try:
        hook_in_repo_root = Path(hook_cwd).resolve() == repo.resolve()
    except OSError:
        hook_in_repo_root = False
    reproduced = commit["rc"] == 0 and mutated and hook_in_repo_root and "dirty.txt" not in committed["out"]
    obs = (
        f"Le `pre-commit` s'exécute avec cwd = racine du worktree utilisateur et réécrit un fichier "
        f"sale non stagé (`dirty.txt` devient {dirty_after!r}) : aucun isolat. Le commit ne contient "
        "que le fichier stagé ; la modification du hook reste comme salissure utilisateur."
        if reproduced else
        "Le hook n'a pas modifié le fichier utilisateur comme attendu ; voir le log."
    )
    return c.finish("REPRODUCED" if reproduced else "NOT_REPRODUCED", obs)


def test_hook_02c() -> dict:
    repo = copytree(FIX / "f2-hooks" / "order", SCRATCH / "hook02c")
    c = Case(
        "HOOK-02c",
        "r0b-local:f2-hooks/order (R0-a fixtures absent)",
        "Git 2.51.1 native `git commit` on Windows, POSIX sh hooks. Git for Windows only; "
        "no other OS or hook runner tested.",
    )
    commit = c.run(["git", "-C", str(repo), "commit", "-q", "-m", "hook order"], cwd=repo)
    log = git_dir(repo) / "hook-order.log"
    order = log.read_text(encoding="utf-8").split() if log.exists() else []
    message = git_out(repo, "log", "-1", "--format=%B") if commit["rc"] == 0 else ""
    expected = ["pre-commit", "prepare-commit-msg", "commit-msg", "post-commit"]
    c.facts(commit_rc=commit["rc"], order=order, expected=expected,
            message_has_prepared="prepared-by-hook" in message)
    reproduced = order == expected
    obs = (
        f"Ordre observé : {' → '.join(order)} (attendu {' → '.join(expected)}). "
        "Le message contient aussi `prepared-by-hook`, donc `prepare-commit-msg` peut éditer le "
        "message — mais il tourne APRÈS `pre-commit`, contrairement au prototype Ravum."
        if reproduced else
        f"Ordre observé {order} ≠ attendu {expected} ; voir le log."
    )
    return c.finish("REPRODUCED" if reproduced else "DISPROVED", obs)


def test_hook_02d() -> dict:
    repo = copytree(FIX / "f2-hooks" / "refuse", SCRATCH / "hook02d")
    c = Case(
        "HOOK-02d",
        "r0b-local:f2-hooks/refuse (R0-a fixtures absent)",
        "Git 2.51.1 native `git commit`. Checks reachable commits and staged index only; "
        "unreachable object inspection is limited to `git fsck --unreachable`.",
    )
    head_before = head_of(repo)
    count_before = commit_count(repo)
    staged_before = git_out(repo, "diff", "--cached", "--name-only")
    commit = c.run(["git", "-C", str(repo), "commit", "-q", "-m", "must be refused"], cwd=repo)
    head_after = head_of(repo)
    count_after = commit_count(repo)
    staged_after = git_out(repo, "diff", "--cached", "--name-only")
    fsck = c.run(["git", "-C", str(repo), "fsck", "--unreachable", "--no-reflogs"], cwd=repo)
    unreachable_commits = [ln for ln in fsck["out"].splitlines() if "unreachable commit" in ln]
    unreachable_trees = [ln for ln in fsck["out"].splitlines() if "unreachable tree" in ln]
    c.facts(commit_rc=commit["rc"], head_before=head_before, head_after=head_after,
            count_before=count_before, count_after=count_after,
            staged_before=staged_before, staged_after=staged_after,
            stderr=commit["err"].strip(), unreachable_commits=unreachable_commits,
            unreachable_trees=unreachable_trees)
    reproduced = (
        commit["rc"] != 0
        and head_after == head_before
        and count_after == count_before
        and staged_after == staged_before
        and "pre-commit deliberately refused" in commit["err"]
        and not unreachable_commits
    )
    obs = (
        "Refus traçable (stderr du hook, rc!=0), HEAD et le nombre de commits inchangés, index "
        "toujours stagé, et `git fsck --unreachable` ne montre ni commit ni tree orphelin : le refus "
        "ne laisse aucun objet partiel."
        if reproduced else
        "Le refus n'est pas propre dans les conditions du test ; voir le log."
    )
    return c.finish("REPRODUCED" if reproduced else "NOT_REPRODUCED", obs)


def test_hook_gitdir_19() -> dict:
    repo = FIX / "gitdir-main"
    wt = SCRATCH / "gitdir-linked"
    rmtree(wt)
    c = Case(
        "HOOK-GITDIR-19",
        "r0b-local:gitdir-main (R0-a fixtures absent)",
        "Git 2.51.1 for Windows, local NTFS. Shows shared control files of a linked worktree; "
        "does not claim a sandbox escape or a privilege boundary bypass.",
    )
    c.run(["git", "-C", str(repo), "worktree", "add", "--detach", str(wt), "HEAD"], cwd=repo)
    layout = c.run(["git", "-C", str(wt), "rev-parse", "--git-dir", "--git-common-dir"], cwd=wt)
    cfg = c.run(["git", "-C", str(wt), "config", "r0b.shared", "written-from-linked"], cwd=wt)
    cfg_read = c.run(["git", "-C", str(repo), "config", "--get", "r0b.shared"], cwd=repo)
    main_config = read_file(repo / ".git" / "config")
    commit_linked = c.run(["git", "-C", str(wt), "commit", "--allow-empty", "-q", "-m", "linked-commit"], cwd=wt)
    b = git_out(wt, "rev-parse", "HEAD") if commit_linked["rc"] == 0 else None
    moved = c.run(["git", "-C", str(wt), "update-ref", "refs/heads/moved", b or "HEAD"], cwd=wt)
    seen_from_main = ref_of(repo, "refs/heads/moved")
    # private full copy comparison
    private = SCRATCH / "gitdir-private-copy"
    copytree(repo, private)
    c.run(["git", "-C", str(private), "config", "r0b.shared", "only-in-copy"], cwd=private)
    c.run(["git", "-C", str(private), "update-ref", "refs/heads/copyref", head_of(private)], cwd=private)
    origin_after_copy = c.run(["git", "-C", str(repo), "config", "--get", "r0b.shared"], cwd=repo)
    origin_copyref = ref_of(repo, "refs/heads/copyref")
    c.facts(layout=layout["out"].strip().splitlines(),
            config_written=cfg["rc"] == 0, config_seen_in_main=cfg_read["out"].strip(),
            config_section_in_main=(b"[r0b]" in main_config and b"written-from-linked" in main_config),
            moved_ref=b, seen_from_main=seen_from_main,
            origin_after_copy=origin_after_copy["out"].strip(), origin_copyref=origin_copyref)
    reproduced = (
        cfg_read["out"].strip() == "written-from-linked"
        and b"[r0b]" in main_config and b"written-from-linked" in main_config
        and moved["rc"] == 0
        and seen_from_main == b
        and origin_after_copy["out"].strip() == "written-from-linked"
        and origin_copyref is None
    )
    obs = (
        f"Depuis le worktree lié ({layout['out'].splitlines()[0].strip()}), "
        "`git config` écrit dans le `.git/config` commun (visible depuis le dépôt principal) et "
        f"`git update-ref` déplace une ref du dépôt principal ({b[:12]}…). Une copie privée complète "
        "n'a aucun de ces effets sur l'original : seule la copie est isolée."
        if reproduced else
        "Le partage de fichiers de contrôle n'est pas démontré comme attendu ; voir le log."
    )
    return c.finish("REPRODUCED" if reproduced else "NOT_REPRODUCED", obs)


def test_ref_10() -> dict:
    repo = FIX / "refcas"
    c = Case(
        "REF-10",
        "r0b-local:refcas (R0-a fixtures absent)",
        "Git 2.51.1 `update-ref` CAS semantics on local refs. Does not cover packed-refs races, "
        "reflog-less setups, or network filesystems.",
    )
    a = head_of(repo)
    git(repo, "commit", "-q", "--allow-empty", "-m", "B")
    b = head_of(repo)
    git(repo, "commit", "-q", "--allow-empty", "-m", "C")
    c_oid = head_of(repo)
    create = c.run(["git", "-C", str(repo), "update-ref", "refs/heads/x", a], cwd=repo)
    move_ok = c.run(["git", "-C", str(repo), "update-ref", "refs/heads/x", b, a], cwd=repo)
    stale = c.run(["git", "-C", str(repo), "update-ref", "refs/heads/x", c_oid, a], cwd=repo)
    ref_after_stale = ref_of(repo, "refs/heads/x")
    correct = c.run(["git", "-C", str(repo), "update-ref", "refs/heads/x", c_oid, b], cwd=repo)
    ref_after_correct = ref_of(repo, "refs/heads/x")
    c.facts(a=a, b=b, c=c_oid, create_rc=create["rc"], move_ok_rc=move_ok["rc"],
            stale_rc=stale["rc"], stale_stderr=stale["err"].strip(),
            ref_after_stale=ref_after_stale, correct_rc=correct["rc"],
            ref_after_correct=ref_after_correct)
    reproduced = (
        move_ok["rc"] == 0 and stale["rc"] != 0 and ref_after_stale == b
        and correct["rc"] == 0 and ref_after_correct == c_oid
        and ("cannot lock ref" in stale["err"] or "is at" in stale["err"])
    )
    obs = (
        "CAS à trois arguments : l'ancien OID périmé est refusé sans déplacer la ref ; le CAS avec "
        "l'ancien OID courant réussit. La comparaison porte sur la valeur courante de la ref."
        if reproduced else
        "Le CAS ne s'est pas comporté comme attendu ; voir le log."
    )
    return c.finish("REPRODUCED" if reproduced else "NOT_REPRODUCED", obs)


def test_fence_07a() -> dict:
    repo = copytree(FIX / "fence-a", SCRATCH / "fence07a")
    c = Case(
        "FENCE-07a",
        "r0b-local:fence-a (R0-a fixtures absent)",
        "Two cooperating writers using `git update-ref` CAS. Shows only that the stale writer is "
        "refused; does not implement an epoch or a lease.",
    )
    old = head_of(repo)
    c1 = git_out(repo, "commit-tree", f"{old}^{{tree}}", "-p", old, "-m", "writer-1")
    c2 = git_out(repo, "commit-tree", f"{old}^{{tree}}", "-p", old, "-m", "writer-2")
    w2 = c.run(["git", "-C", str(repo), "update-ref", "refs/heads/main", c2, old], cwd=repo)
    w1 = c.run(["git", "-C", str(repo), "update-ref", "refs/heads/main", c1, old], cwd=repo)
    ref = ref_of(repo, "refs/heads/main")
    c.facts(old=old, c1=c1, c2=c2, w2_rc=w2["rc"], w1_rc=w1["rc"],
            w1_stderr=w1["err"].strip(), ref=ref)
    reproduced = w2["rc"] == 0 and w1["rc"] != 0 and ref == c2
    obs = (
        "L'écrivain périmé (même `old`) est refusé après qu'un autre écrivain a publié : le CAS de "
        "ref suffit entre écrivains coopérants. Aucun epoch n'est nécessaire pour ce cas."
        if reproduced else
        "Le refus de l'écrivain périmé n'est pas démontré ; voir le log."
    )
    return c.finish("REPRODUCED" if reproduced else "NOT_REPRODUCED", obs)


def test_fence_07b() -> dict:
    repo = copytree(FIX / "fence-b", SCRATCH / "fence07b")
    c = Case(
        "FENCE-07b",
        "r0b-local:fence-b (R0-a fixtures absent)",
        "Shows that an external `git` writer bypasses any Flow-side lock/epoch. The external write "
        "is deliberately successful; no prevention mechanism is tested because none exists.",
    )
    old = head_of(repo)
    candidate = git_out(repo, "commit-tree", f"{old}^{{tree}}", "-p", old, "-m", "our-candidate")
    external = c.run(["git", "-C", str(repo), "commit", "-q", "--allow-empty", "-m", "external-writer"], cwd=repo)
    e = head_of(repo)
    ours = c.run(["git", "-C", str(repo), "update-ref", "refs/heads/main", candidate, old], cwd=repo)
    ref = ref_of(repo, "refs/heads/main")
    reachable = c.run(["git", "-C", str(repo), "merge-base", "--is-ancestor", candidate, "refs/heads/main"], cwd=repo)
    c.facts(old=old, candidate=candidate, external_rc=external["rc"], external_head=e,
            our_rc=ours["rc"], our_stderr=ours["err"].strip(), ref=ref,
            our_candidate_reachable=reachable["rc"] == 0)
    reproduced = external["rc"] == 0 and ours["rc"] != 0 and ref == e and reachable["rc"] != 0
    obs = (
        "Un `git commit` externe réussit entre la lecture de l'ancien OID et notre CAS ; notre CAS "
        "refuse ensuite et notre candidat devient inatteignable. Le contournement n'est pas empêché : "
        "il est seulement détecté (trop tard)."
        if reproduced else
        "Le contournement externe ne s'est pas produit comme attendu ; voir le log."
    )
    return c.finish("REPRODUCED" if reproduced else "NOT_REPRODUCED", obs)


def _run_publisher(repo: Path, crash_at: str, payload: str, c: Case):
    return c.run([PY, str(repo / "publisher.py"), str(repo), crash_at, payload], cwd=repo)


def test_fence_07c() -> dict:
    repo = copytree(FIX / "f5-crash", SCRATCH / "fence07c")
    c = Case(
        "FENCE-07c",
        "r0b-local:f5-crash (R0-a fixtures absent)",
        "Single writer, local FS: the crash window after `update-ref` and before the outcome record. "
        "Recovery classifies from the journal + ref; it does not prove crash consistency on other filesystems.",
    )
    pub = _run_publisher(repo, "after-ref", "fence07c", c)
    journal = read_journal(repo)
    ref = ref_of(repo, "refs/heads/main")
    intent = journal[-1] if journal and journal[-1].get("event") == "intent" else None
    outcome_lines = [r for r in journal if r.get("event") == "outcome"]
    guard = recover_guard(repo)
    after = read_journal(repo)
    c.facts(publisher_rc=pub["rc"], journal=journal, ref=ref,
            intent_new=intent["new"] if intent else None,
            outcomes_before=outcome_lines, recover=guard,
            outcomes_after=[r for r in after if r.get("event") == "outcome"])
    reproduced = (
        pub["rc"] == 70
        and intent is not None
        and ref == intent["new"]
        and len(outcome_lines) == 0
        and guard["state"] == "PUBLISHED"
        and guard["ref_after"] == guard["ref_before"] == ref
        and len([r for r in after if r.get("event") == "outcome"]) == 1
    )
    obs = (
        "Après crash post-CAS : la ref a bougé, le journal n'a qu'un INTENT. La relecture de la ref "
        "classe l'issue comme PUBLISHED (état publié identifiable) ; le guard n'écrit qu'un OUTCOME "
        "de récupération et ne republie pas. L'ambiguïté est résolue par la ref, pas par le journal."
        if reproduced else
        "Le scénario crash post-CAS ne s'est pas comporté comme attendu ; voir le log."
    )
    return c.finish("REPRODUCED" if reproduced else "NOT_REPRODUCED", obs)


def test_crash_11() -> dict:
    c = Case(
        "CRASH-11",
        "r0b-local:f5-crash (R0-a fixtures absent)",
        "Four driven crash points on a single local writer, Windows/NTFS. os._exit(70) approximates "
        "a kill; it does not cover power loss, fsync semantics of the OS, or a second process.",
    )
    results = {}
    for crash_at in ("before-intent", "after-intent", "after-ref", "before-outcome"):
        repo = copytree(FIX / "f5-crash", SCRATCH / f"crash11-{crash_at}")
        pub = _run_publisher(repo, crash_at, f"crash-{crash_at}", c)
        journal = read_journal(repo)
        ref = ref_of(repo, "refs/heads/main")
        guard = recover_guard(repo)
        # naive replay in a separate copy: re-run the publisher with no idempotency check
        naive = copytree(repo, SCRATCH / f"crash11-{crash_at}-naive")
        replay = _run_publisher(naive, "none", "naive-replay", c)
        results[crash_at] = {
            "publisher_rc": pub["rc"],
            "journal_events": journal,
            "ref": ref,
            "recover": guard,
            "naive_replay_rc": replay["rc"],
            "naive_replay_ref": ref_of(naive, "refs/heads/main"),
            "naive_journal": read_journal(naive),
        }
    # naive replay evidence: no idempotency key, so a retry republishes
    naive = {
        "after-intent_two_intents": len([r for r in results["after-intent"]["naive_journal"]
                                         if r.get("event") == "intent"]) == 2,
        "after-ref_second_publication": results["after-ref"]["naive_replay_ref"] != results["after-ref"]["ref"],
        "before-intent_single_publication": results["before-intent"]["naive_replay_rc"] == 0,
    }
    c.facts(naive_replay=naive, **{f"case_{k}": v for k, v in results.items()})

    # expectations
    def events(name):
        return [r.get("event") for r in results[name]["journal_events"]]

    ok = {
        "before-intent": results["before-intent"]["journal_events"] == []
        and results["before-intent"]["recover"]["state"] == "NOT_STARTED",
        "after-intent": events("after-intent") == ["intent"]
        and results["after-intent"]["recover"]["state"] == "NOT_PUBLISHED",
        "after-ref": events("after-ref") == ["intent"]
        and results["after-ref"]["recover"]["state"] == "PUBLISHED",
        "before-outcome": events("before-outcome") == ["intent", "outcome"]
        and results["before-outcome"]["recover"]["state"] == "PUBLISHED",
    }
    # the 'before-outcome' crash point actually crashes AFTER the outcome append
    misnamed = (
        results["before-outcome"]["journal_events"][-1].get("status") == "PUBLISHED"
        and results["before-outcome"]["publisher_rc"] == 70
    )
    reproduced = all(ok.values()) and misnamed and naive["after-intent_two_intents"] and naive["after-ref_second_publication"]
    obs = (
        "Matrice : before-intent → journal vide ; after-intent → INTENT sans outcome, ref=old "
        "(NOT_PUBLISHED) ; after-ref → ref bougée sans outcome, récupérée en PUBLISHED (l'ambiguïté est "
        "résolue par la relecture de la ref, pas par le journal) ; before-outcome → l'outcome est déjà "
        "écrit avant le crash (point mal nommé, récupération triviale). Rejeu naïf sans clé d'idempotence : "
        "after-intent produit deux INTENT et deux commits distincts pour le même tree ; after-ref publie "
        "un second commit par-dessus le commit déjà publié. La garde de récupération est donc nécessaire."
        if reproduced else
        "La matrice de crash ne correspond pas aux attentes ; voir le log."
    )
    return c.finish("REPRODUCED" if reproduced else "NOT_REPRODUCED", obs)


def test_file_09() -> dict:
    base = FIX / "f3-filetypes" / "base"
    link = FIX / "f3-filetypes" / "link"
    case = FIX / "f3-filetypes" / "case"
    c = Case(
        "FILE-09",
        "r0b-local:f3-filetypes/{base,link,case} (R0-a fixtures absent)",
        "Git 2.51.1 for Windows on NTFS (case-insensitive, core.filemode=false, core.symlinks "
        "effectively false). Mode/bytes in the tree are exact; worktree restrictions are documented, not fixed.",
    )
    first = git_out(base, "rev-list", "--max-parents=0", "HEAD")
    second = head_of(base)
    modes = {}
    for rec in git_out(base, "ls-tree", "-r", "HEAD").splitlines():
        meta, path = rec.split("\t", 1)
        modes[path] = meta.split(" ")[0]
    bin_hash = git(base, "hash-object", "b.bin")["out"].strip()
    tree_bin = [ln.split()[2] for ln in git_out(base, "ls-tree", "-r", "HEAD").splitlines() if ln.endswith("\tb.bin")][0]
    rename = c.run(["git", "-C", str(base), "diff", "-M", "--summary", first, second], cwd=base)
    crlf_out = git(base, "cat-file", "blob", "HEAD:crlf.txt")["out"]
    crlf_has_crlf_in_blob = "\r\n" in crlf_out
    # link checkout: fresh worktree
    link_wt = SCRATCH / "file09-link"
    rmtree(link_wt)
    link_add = c.run(["git", "-C", str(link), "worktree", "add", "--detach", str(link_wt), "HEAD"], cwd=link)
    link_entry = git_out(link, "ls-tree", "HEAD", "link.txt").split()[0]
    link_is_real_symlink = os.path.islink(link_wt / "link.txt")
    link_content = (link_wt / "link.txt").read_bytes() if (link_wt / "link.txt").exists() else b""
    # case collision checkout: fresh worktree
    case_wt = SCRATCH / "file09-case"
    rmtree(case_wt)
    case_add = c.run(["git", "-C", str(case), "worktree", "add", "--detach", str(case_wt), "HEAD"], cwd=case)
    case_entries_tree = [ln for ln in git_out(case, "ls-tree", "-r", "--name-only", "HEAD").splitlines() if ln]
    listing = sorted(os.listdir(case_wt)) if case_wt.exists() else []
    case_status = (
        c.run(["git", "-C", str(case_wt), "status", "--porcelain"], cwd=case_wt)["out"].strip()
        if case_wt.exists() else "<worktree absent>"
    )
    c.facts(modes=modes, tree_bin=tree_bin, bin_hash=bin_hash,
            rename_summary=rename["out"].strip(), delete_present="delete.txt" in modes,
            crlf_blob_has_crlf=crlf_has_crlf_in_blob,
            link_add_rc=link_add["rc"], link_mode=link_entry,
            link_is_real_symlink=link_is_real_symlink,
            link_content=link_content, case_add_rc=case_add["rc"],
            case_entries_tree=case_entries_tree,
            case_worktree_listing=listing, case_status=case_status)
    reproduced = (
        modes.get("script.sh") == "100755"
        and modes.get("b.bin") == "100644"
        and tree_bin == bin_hash
        and "rename" in rename["out"] and "renamed.txt" in rename["out"]
        and "delete.txt" not in modes
        and link_entry == "120000"
        and set(case_entries_tree) >= {"Case.txt", "case.txt"}
    )
    obs = (
        f"Modes/tree : script.sh={modes.get('script.sh')}, b.bin={modes.get('b.bin')} (blob identique "
        f"aux octets du worktree), rename détecté ({rename['out'].strip().splitlines()[-1].strip() if rename['out'].strip() else 'n/a'}), "
        f"delete.txt absent. Le blob de crlf.txt contient des CRLF : {crlf_has_crlf_in_blob} (attribut `text` → octets du tree "
        f"normalisés LF, différents du disque). link.txt={link_entry} : checkout rc={link_add['rc']}, "
        f"symlink réel sur cet hôte={link_is_real_symlink}, lecture={link_content!r} — la cible du blob inclut "
        f"un saut de ligne, donc le lien est pendant. Casse : {case_entries_tree} dans le tree mais {listing} "
        f"sur disque, `git status`={case_status!r} : une entrée est écrasée et le worktree sort modifié."
        if reproduced else
        "Un des invariants de mode/octet/rename n'est pas vérifié ; voir le log."
    )
    return c.finish("REPRODUCED" if reproduced else "NOT_REPRODUCED", obs)


TESTS = [
    test_tree_01,
    test_tree_02,
    test_hook_02a,
    test_hook_02b,
    test_hook_02c,
    test_hook_02d,
    test_hook_gitdir_19,
    test_ref_10,
    test_fence_07a,
    test_fence_07b,
    test_fence_07c,
    test_crash_11,
    test_file_09,
]

TID = {
    "test_tree_01": "TREE-01",
    "test_tree_02": "TREE-02",
    "test_hook_02a": "HOOK-02a",
    "test_hook_02b": "HOOK-02b",
    "test_hook_02c": "HOOK-02c",
    "test_hook_02d": "HOOK-02d",
    "test_hook_gitdir_19": "HOOK-GITDIR-19",
    "test_ref_10": "REF-10",
    "test_fence_07a": "FENCE-07a",
    "test_fence_07b": "FENCE-07b",
    "test_fence_07c": "FENCE-07c",
    "test_crash_11": "CRASH-11",
    "test_file_09": "FILE-09",
}


# --------------------------------------------------------------------------
# main
# --------------------------------------------------------------------------


def main(argv: list[str]) -> int:
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass
    only = set()
    if "--only" in argv:
        only = set(argv[argv.index("--only") + 1].split(","))
    for d in (BASE, FIX, SCRATCH, LOGS, PRIVATE):
        d.mkdir(parents=True, exist_ok=True)
    if not GITCONFIG.exists():
        GITCONFIG.write_text("", encoding="utf-8")
    if ("fixtures" not in only) and ("nofixtures" not in only):
        print("building local fixtures under", redact(str(FIX)))
        build_all_fixtures()
    print(OS_LABEL)

    results = []
    for fn in TESTS:
        tid_guess = TID.get(fn.__name__, fn.__name__.upper().replace("TEST_", "").replace("_", "-"))
        if only and tid_guess not in only and fn.__name__ not in only:
            continue
        try:
            rec = fn()
        except Exception as exc:  # noqa: BLE001 - a harness failure is a result, not a crash
            LOGS.mkdir(parents=True, exist_ok=True)
            log = LOGS / f"ERROR-{fn.__name__}.log"
            log.write_text(redact(f"{type(exc).__name__}: {exc}\n"), encoding="utf-8")
            rec = {
                "id": tid_guess,
                "verdict": "BLOCKED",
                "fixture": "n/a",
                "os": OS_LABEL,
                "commands": [],
                "observation": redact(f"harness error: {type(exc).__name__}: {exc}"),
                "scope": "Test did not execute to completion; no conclusion.",
                "evidence": f"logs/ERROR-{fn.__name__}.log",
            }
        results.append(rec)
        print(f"[{rec['verdict']:>14}] {rec['id']}: {rec['observation'][:110]}")

    out = BASE / "RESULTS.json"
    out.write_text(json.dumps(results, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    counts: dict[str, int] = {}
    for rec in results:
        counts[rec["verdict"]] = counts.get(rec["verdict"], 0) + 1
    summary = (
        f"R0-b git summary {time.strftime('%Y-%m-%dT%H:%M:%S')}\n"
        f"{OS_LABEL}\n"
        f"tests={len(results)} counts={json.dumps(counts, sort_keys=True)}\n"
        + "\n".join(f"{rec['verdict']:>14}  {rec['id']}" for rec in results)
        + "\n"
    )
    (LOGS / "summary.txt").write_text(summary, encoding="utf-8")
    print(summary)
    return 0 if results else 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
