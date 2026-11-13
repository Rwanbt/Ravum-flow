"""R0-a fixtures F0–F7: disposable synthetic repositories.

Every fixture is built under experiments/r0a/work/fixtures and inventoried.
Nothing here reads Ravum, the user's repositories, or a real credential.
"""
from __future__ import annotations

import json
import os
import stat
from pathlib import Path

from .common import (
    CANARY, FIXTURES, Fixture, child_env, commit_all, ensure_layout, finalize, finalize_multi, git,
    init_repo, inventory_add, run, sha256_bytes, tree_digest, write_file, WORK,
)

CARGO_TOML = """[package]
name = "fixture_core"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"
"""

LIB_V1 = 'pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n'
LIB_V2 = LIB_V1 + '\npub fn double(a: i32) -> i32 {\n    add(a, a)\n}\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn adds() {\n        assert_eq!(double(3), 6);\n    }\n}\n'


def _abs(path: Path) -> Path:
    return path.resolve()


# ---------------------------------------------------------------- F0 core
def build_f0_core(name: str = "f0-core", root: Path | None = None) -> Fixture:
    ensure_layout()
    repo = (root or FIXTURES) / name
    repo.parent.mkdir(parents=True, exist_ok=True)
    init_repo(repo)
    write_file(repo / ".gitignore", "target/\n*.log\ngen/\n")
    write_file(repo / "Cargo.toml", CARGO_TOML)
    write_file(repo / "src" / "lib.rs", LIB_V1)
    first = commit_all(repo, "fixture: initial library")
    write_file(repo / "src" / "lib.rs", LIB_V2)
    write_file(repo / "README.md", "# synthetic fixture\n\nsecond commit content\n")
    second = commit_all(repo, "fixture: add double() and README")
    # deliberate dirty/untracked/ignored state (never published)
    write_file(repo / "README.md", "# synthetic fixture\n\nlocal uncommitted edit\n")
    write_file(repo / "untracked.txt", "untracked by design\n")
    write_file(repo / "build.log", "ignored log line\n")
    write_file(repo / "gen" / "cache.bin", b"\x00\x01\x02ignored-cache")
    fx = Fixture(name=name, path=repo, description="mono-language repo, 2 commits, dirty+untracked+ignored",
                 sub_cases={"first_commit": first, "second_commit": second})
    return finalize(fx, repo, ["cargo manifest present; `cargo test` is available offline but not run by R0-a"])


# ------------------------------------------------------- F1 concurrency
def build_f1_concurrency() -> Fixture:
    repo = FIXTURES / "f1-concurrency"
    init_repo(repo)
    for path, lines in (("a.txt", ["a1", "a2", "a3"]), ("b.txt", ["b1", "b2", "b3"]),
                        ("c.txt", ["c1", "c2", "c3"])):
        write_file(repo / path, "\n".join(lines) + "\n")
    base = commit_all(repo, "fixture: base files")

    def branch(name: str, edits: dict[str, str], message: str) -> str:
        git(repo, "checkout", "-q", "-b", name, base)
        for path, content in edits.items():
            write_file(repo / path, content)
        oid = commit_all(repo, message)
        return oid

    a = branch("agent-a", {"a.txt": "a1\na2-a\na3\n", "c.txt": "c1\nc2-agent-a\nc3\n"}, "agent a: a.txt + c.txt")
    b_oid = branch("agent-b", {"b.txt": "b1\nb2-agent-b\nb3\n"}, "agent b: b.txt")
    c = branch("agent-c", {"c.txt": "c1\nc2-agent-c\nc3\n"}, "agent c: c.txt conflict")
    git(repo, "checkout", "-q", "main")

    # expected outcomes are recorded as data, then verified by the builder itself
    git(repo, "checkout", "-q", "agent-a")
    merge_disjoint = git(repo, "merge", "--no-ff", "-m", "merge b into a", "agent-b", check=False)
    disjoint_ok = merge_disjoint.returncode == 0
    git(repo, "merge", "--abort", check=False)
    git(repo, "checkout", "-q", "main")

    fx = Fixture(name="f1-concurrency", path=repo,
                 description="disjoint and same-hunk agent branches",
                 sub_cases={"base": base, "agent_a": a, "agent_b": b_oid, "agent_c": c,
                            "disjoint_merge_clean": disjoint_ok})
    fx.notes.append("merge a+b is expected to be clean; a+c is expected to conflict (same hunk)")
    return finalize(fx, repo)


# -------------------------------------------------------------- F2 hooks
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


def _install_hooks(repo: Path, hooks: dict[str, str]) -> None:
    hook_dir = Path(git(repo, "rev-parse", "--git-path", "hooks").stdout.strip())
    if not hook_dir.is_absolute():
        hook_dir = repo / hook_dir
    hook_dir.mkdir(parents=True, exist_ok=True)
    for name, body in hooks.items():
        p = hook_dir / name
        p.write_bytes(body.replace("\r\n", "\n").encode())
        try:
            p.chmod(p.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
        except OSError:
            pass
        # Git for Windows only runs hooks whose shebang it can resolve
        if os.name == "nt":
            (hook_dir / f"{name}.sh").write_bytes(p.read_bytes())


def build_f2_hooks() -> Fixture:
    root = FIXTURES / "f2-hooks"
    variants = {}
    for name, hooks in (
        ("order", {"pre-commit": HOOK_ORDER, "prepare-commit-msg": HOOK_PREPARE,
                   "commit-msg": HOOK_COMMITMSG, "post-commit": HOOK_POST}),
        ("refuse", {"pre-commit": HOOK_REFUSE}),
        ("mutate", {"pre-commit": HOOK_MUTATE}),
    ):
        repo = root / name
        init_repo(repo)
        write_file(repo / "seed.txt", "seed\n")
        seed = commit_all(repo, "fixture: hook variant seed")
        _install_hooks(repo, hooks)
        write_file(repo / "work.txt", "staged by experiment\n")
        git(repo, "add", "work.txt")
        variants[name] = {"path": (Path("f2-hooks") / name).as_posix(), "seed": seed,
                          "hook_order_log": "git-dir/hook-order.log"}
    fx = Fixture(name="f2-hooks", path=root,
                 description="hook variants: order logging, refusal, index mutation",
                 sub_cases=variants)
    fx.notes.append("hooks are installed in .git/hooks only; no hook is committed and none runs on Ravum")
    return finalize_multi(fx, root)


# ---------------------------------------------------------- F3 file types
def _f3_base() -> dict:
    repo = FIXTURES / "f3-filetypes" / "base"
    init_repo(repo)
    write_file(repo / ".gitattributes", "*.txt text eol=crlf\n*.bin -text\n")
    write_file(repo / "a.txt", "alpha\n")
    write_file(repo / "crlf.txt", b"line one\r\nline two\r\n")
    write_file(repo / "b.bin", bytes(range(0, 32)) + b"\x00\xff")
    write_file(repo / "script.sh", "#!/bin/sh\necho fixture\n")
    write_file(repo / "keep.txt", "to be renamed\n")
    write_file(repo / "delete.txt", "to be deleted\n")
    git(repo, "add", "-A")
    # On Windows `git add` from the worktree downgrades the exec bit to 100644;
    # only the index can carry 100755 (recorded as a FILE-09 observation).
    git(repo, "update-index", "--chmod=+x", "script.sh")
    git(repo, "commit", "-q", "-m", "fixture: base file types")
    first = git(repo, "rev-parse", "HEAD").stdout.strip()
    git(repo, "mv", "keep.txt", "renamed.txt")
    git(repo, "rm", "-q", "delete.txt")
    # commit the index directly: `git add -A` would re-stage script.sh as 100644
    git(repo, "commit", "-q", "-m", "fixture: rename and delete")
    second = git(repo, "rev-parse", "HEAD").stdout.strip()
    modes = {}
    for rec in git(repo, "ls-tree", "-r", "HEAD").stdout.splitlines():
        meta, path = rec.split("\t", 1)
        modes[path] = meta.split(" ")[0]
    return {"path": "f3-filetypes/base", "first": first, "second": second, "modes": modes}


def _f3_link() -> dict:
    repo = FIXTURES / "f3-filetypes" / "link"
    init_repo(repo)
    write_file(repo / "target.txt", "target content\n")
    commit_all(repo, "fixture: link target")
    # create a real 120000 blob through plumbing: works even where the OS
    # cannot create symlinks (Windows without Developer Mode).
    # A Git symlink blob contains the target path with NO trailing newline.
    oid = git(repo, "hash-object", "-w", "--stdin", stdin_text="target.txt").stdout.strip()
    git(repo, "update-index", "--add", "--cacheinfo", f"120000,{oid},link.txt")
    # commit the index as-is: `git add -A` would stage the missing worktree file as a deletion
    git(repo, "commit", "-q", "-m", "fixture: symlink entry")
    mode = [ln.split(" ")[0] for ln in git(repo, "ls-tree", "HEAD", "link.txt").stdout.splitlines()][0]
    commit = git(repo, "rev-parse", "HEAD").stdout.strip()
    return {"path": "f3-filetypes/link", "commit": commit, "mode": mode,
            "worktree_is_real_symlink": False}


def _f3_case() -> dict:
    repo = FIXTURES / "f3-filetypes" / "case"
    init_repo(repo, filemode=False)
    write_file(repo / "seed.txt", "seed\n")
    commit_all(repo, "fixture: case base")
    for name, content in (("Case.txt", "upper\n"), ("case.txt", "lower\n")):
        write_file(repo / "tmp-name.txt", content)
        oid = git(repo, "hash-object", "-w", "tmp-name.txt").stdout.strip()
        git(repo, "update-index", "--add", "--cacheinfo", f"100644,{oid},{name}")
        (repo / "tmp-name.txt").unlink()
    # commit the index as-is: `git add -A` would drop entries the worktree cannot materialise
    git(repo, "commit", "-q", "-m", "fixture: case-ambiguous entries")
    commit = git(repo, "rev-parse", "HEAD").stdout.strip()
    entries = [ln for ln in git(repo, "ls-tree", "-r", "--name-only", "HEAD").stdout.splitlines() if ln]
    checkout = git(repo, "checkout", "--", ".", check=False)
    return {"path": "f3-filetypes/case", "commit": commit,
            "index_entries": entries,
            "checkout_rc": checkout.returncode,
            "checkout_stderr": checkout.stderr.strip()[:200]}


def build_f3_filetypes() -> Fixture:
    root = FIXTURES / "f3-filetypes"
    sub = {"base": _f3_base(), "link": _f3_link(), "case": _f3_case()}
    fx = Fixture(name="f3-filetypes", path=root,
                 description="binary, exec bit, symlink entry, rename, delete, CRLF, case collision",
                 sub_cases=sub)
    fx.notes.append("exec bit forced with `git update-index --chmod=+x`; Windows cannot store it in the FS")
    fx.notes.append("symlink is a real 120000 tree entry; the Windows worktree file is not a real symlink")
    fx.notes.append("case collision lives in the index/tree; a case-insensitive worktree cannot hold both")
    return finalize(fx, root / "base")


# --------------------------------------------------------------- F4 E2E
E2E_API = "def area(width: int, height: int) -> int:\n    return width * height\n"
E2E_API_BREAKING = "def area(width: int) -> int:\n    # breaking signature: height is gone\n    return width * width\n"
E2E_CONSUMER = "from api import area\n\n\ndef rectangle(w: int, h: int) -> int:\n    return area(w, h)\n"
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


def build_f4_e2e() -> Fixture:
    repo = FIXTURES / "f4-e2e"
    init_repo(repo)
    write_file(repo / "api.py", E2E_API)
    write_file(repo / "consumer.py", E2E_CONSUMER)
    write_file(repo / "test_e2e.py", E2E_TEST)
    green = commit_all(repo, "fixture: E2E green")
    git(repo, "checkout", "-q", "-b", "breaking", green)
    write_file(repo / "api.py", E2E_API_BREAKING)
    breaking = commit_all(repo, "fixture: API break, consumer untouched")
    git(repo, "checkout", "-q", "main")

    def run_suite(rev: str) -> int:
        target = FIXTURES.parent / "f4-checkouts" / rev[:7]
        git(repo, "worktree", "add", "-q", "--detach", str(target), rev)
        cp = run(["python", "-m", "unittest", "discover", "-q"], cwd=target, timeout=120, check=False)
        return cp.returncode

    rc_green = run_suite(green)
    rc_breaking = run_suite(breaking)
    fx = Fixture(name="f4-e2e", path=repo,
                 description="fake API + unchanged consumer; E2E deliberately red on the breaking branch",
                 sub_cases={"green": green, "breaking": breaking,
                            "e2e_green_rc": rc_green, "e2e_breaking_rc": rc_breaking})
    fx.notes.append("a required E2E that is red must block publication (E2E-12); rc != 0 is the evidence")
    return finalize(fx, repo)


# ------------------------------------------------------------- F5 crash
PUBLISHER = r'''"""Minimal intent -> CAS -> outcome publisher with driven crash points.

Used only inside a disposable fixture repository. It models the journal
protocol under test, not a Flow implementation.
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
    # build the candidate commit through plumbing
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


def build_f5_crash() -> Fixture:
    repo = FIXTURES / "f5-crash"
    init_repo(repo)
    write_file(repo / "publisher.py", PUBLISHER)
    write_file(repo / "state.txt", "v1\n")
    base = commit_all(repo, "fixture: crash publisher seed")
    fx = Fixture(name="f5-crash", path=repo,
                 description="driven crash at journal/ref boundaries (process exit, no cleanup)",
                 sub_cases={"base": base,
                            "crash_points": ["before-intent", "after-intent", "after-ref", "before-outcome"]})
    fx.notes.append("crash uses os._exit(70): no finally block, no flush, closest portable analogue to SIGKILL")
    return finalize(fx, repo)


# ------------------------------------------------------------ F6 canary
PROBE = r'''"""Probe used by R0-b: fake secret read + localhost network attempt.

It never contains a real secret. It reports what the boundary allowed.
"""
import json
import os
import socket
import sys

from pathlib import Path

result = {"read_secret": False, "secret_sha256": None, "network": False, "error": None}
try:
    home = Path(os.environ.get("FLOW_FAKE_HOME", "/nonexistent"))
    key = home / ".ssh" / "id_fixture"
    data = key.read_bytes()
    import hashlib
    result["read_secret"] = True
    result["secret_sha256"] = hashlib.sha256(data).hexdigest()
except Exception as exc:
    result["error"] = f"{type(exc).__name__}: {exc}"
try:
    sock = socket.create_connection(("127.0.0.1", 9), timeout=0.2)
    sock.close()
    result["network"] = True
except OSError as exc:
    result["error"] = (result["error"] or "") + f" | net: {type(exc).__name__}"
print(json.dumps(result, sort_keys=True))
'''


def build_f6_canary() -> Fixture:
    repo = FIXTURES / "f6-canary"
    init_repo(repo)
    write_file(repo / "probe.py", PROBE)
    commit = commit_all(repo, "fixture: canary probe")
    fake_home = WORK / "fake-home"
    write_file(fake_home / ".ssh" / "id_fixture",
               f"-----BEGIN FIXTURE KEY-----\n{CANARY}\n-----END FIXTURE KEY-----\n")
    write_file(fake_home / ".ssh" / "known_hosts", "fixture.invalid ssh-ed25519 AAAAfake\n")
    inventory_add([fake_home])
    fx = Fixture(name="f6-canary", path=repo,
                 description="fictitious canary in a fake ~/.ssh plus a localhost network attempt",
                 sub_cases={"commit": commit, "fake_home": fake_home.relative_to(WORK).as_posix()})
    fx.notes.append("canary is fictitious; the network attempt targets 127.0.0.1:9 only, never the internet")
    return finalize(fx, repo)


# ----------------------------------------------------------- F7 context
CONTEXT_BUILD = r'''"""Tiny build whose declared inputs are explicit; used for CONTEXT-03/CACHE-13.

The artifact also consumes an IGNORED cache file that is NOT part of the declared
context. That is deliberate: it reproduces a cache-poisoning window (a declared
context id that does not move while the produced artifact does).
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
cache_sha = hashlib.sha256(cache.read_bytes()).hexdigest() if cache.exists() else "absent"
artifact = (
    f"artifact context={context_id[:12]} cache={cache_sha[:12]} "
    f"toolchain={os.environ.get('FLOW_TOOLCHAIN', '')}\n"
).encode()
(root / "out").mkdir(exist_ok=True)
(root / "out" / "artifact.txt").write_bytes(artifact)
(root / "out" / "context.json").write_text(
    json.dumps({"context_id": context_id, "inputs": manifest, "cache_sha": cache_sha}, sort_keys=True, indent=2)
)
print(json.dumps({"context_id": context_id, "cache_sha": cache_sha,
                  "artifact_sha": hashlib.sha256(artifact).hexdigest()}))
'''


def build_f7_context() -> Fixture:
    repo = FIXTURES / "f7-context"
    init_repo(repo)
    write_file(repo / ".gitignore", "out/\n.cache/\n")
    write_file(repo / "tools" / "build.py", CONTEXT_BUILD)
    write_file(repo / "inputs" / "flags.txt", "-O1\n")
    write_file(repo / "inputs" / "lock.txt", "dep-a 1.0.0\ndep-b 2.0.0\n")
    base = commit_all(repo, "fixture: declared-input build")
    cp = run(["python", "tools/build.py"], cwd=repo, check=False)
    first = json.loads(cp.stdout or "{}")
    context_id = first.get("context_id")
    write_file(repo / ".cache" / "tool.bin", b"ignored cache v1")
    cp_cache = run(["python", "tools/build.py"], cwd=repo, check=False)
    after_cache = json.loads(cp_cache.stdout or "{}")
    # variant 1: changed undeclared env, declared inputs unchanged
    cp2 = run(["python", "tools/build.py"], cwd=repo,
              env=child_env({"FLOW_TOOLCHAIN": "rust-1.95.0"}), check=False)
    context_id_env = json.loads(cp2.stdout or "{}").get("context_id")
    # variant 2: changed declared input (dirty worktree)
    write_file(repo / "inputs" / "flags.txt", "-O2\n")
    cp3 = run(["python", "tools/build.py"], cwd=repo, check=False)
    context_id_dirty = json.loads(cp3.stdout or "{}").get("context_id")
    fx = Fixture(name="f7-context", path=repo,
                 description="declared vs undeclared build inputs and an ignored cache",
                 sub_cases={"base": base, "context_id_base": context_id,
                            "context_id_changed_env": context_id_env,
                            "context_id_changed_flags": context_id_dirty,
                            "cache_sha_initial": first.get("cache_sha"),
                            "cache_sha_after_write": after_cache.get("cache_sha"),
                            "artifact_sha_initial": first.get("artifact_sha"),
                            "artifact_sha_after_cache": after_cache.get("artifact_sha"),
                            "context_id_after_cache": after_cache.get("context_id")})
    fx.notes.append("the artifact consumes an ignored cache that is NOT in the declared context: same context id, different artifact")
    fx.notes.append("CACHE-13: a cache key must change when a declared input changes; an ignored file is undeclared by construction")
    return finalize(fx, repo)


BUILDERS = {
    "f0-core": build_f0_core,
    "f1-concurrency": build_f1_concurrency,
    "f2-hooks": build_f2_hooks,
    "f3-filetypes": build_f3_filetypes,
    "f4-e2e": build_f4_e2e,
    "f5-crash": build_f5_crash,
    "f6-canary": build_f6_canary,
    "f7-context": build_f7_context,
}


def build_all() -> list[Fixture]:
    ensure_layout()
    built = []
    for name, builder in BUILDERS.items():
        built.append(builder())
    inventory_add([FIXTURES, WORK / "f4-checkouts", WORK / "fake-home"])
    return built
