#!/usr/bin/env python3
"""Attempts to defeat the 7a034ff post-run re-check (wave-4 follow-up).

Cases required by the Lead:
  (a) rewrite a declared file and restore it before the re-check
      -> expected UNDETECTED (PASSED + receipt); documented limit;
  (b) an obligation creates an extra directory and deletes it (and a second
      extra directory that is left behind): extra entries are allowed at the
      re-check by design -> expected PASSED;
  (c) a hook creates a directory that collides with a declared directory path
      component (`nested` from `nested/file.txt`) -> expected PASSED, no false
      refusal; and a hook-created child directory `nested/extra` -> expected
      REFUSED.

Uses the helpers of p0_attacks.py; nothing here is product code.
"""

import json
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import p0_attacks as P  # noqa: E402


def brief_run(ctx, post, repo, tree):
    rec = {
        "run": None,
        "receipt": ctx.receipt_path(repo, tree).exists(),
        "checkout_recheck": None,
    }
    return rec


def main(argv):
    post = Path(argv[1]).resolve()
    root = Path(tempfile.mkdtemp(prefix="flow-recheck-defeat-"))
    ctx = P.Ctx(post, None, root)
    result = {"meta": {"post": str(post), "sha256": P.sha256(post)}, "probes": {}}

    # (a) write-and-revert during the obligations ----------------------
    repo = ctx.new_repo("write-revert")
    ctx.policy(repo, [{
        "id": "tamper",
        "program": "python",
        "args": ["-c",
                 "import pathlib; p=pathlib.Path('a.txt'); orig=p.read_bytes(); "
                 "p.write_bytes(b'ALPHA\\n'); seen=p.read_bytes(); "
                 "print('seen_hex='+seen.hex()); p.write_bytes(orig); "
                 "raise SystemExit(0 if seen==b'ALPHA\\n' else 9)"],
        "required": True, "timeout_s": 30,
    }])
    tree = ctx.tree(repo)
    run = ctx.run_on(post, repo, "run")
    log = repo / ".flow" / "logs" / tree / "tamper.stdout.log"
    rpath = ctx.receipt_path(repo, tree)
    receipt = json.loads(rpath.read_text(encoding="utf-8")) if rpath.exists() else {}
    result["probes"]["a_write_and_revert"] = {
        "run": P.brief(run),
        "receipt": rpath.exists(),
        "checkout_recheck": receipt.get("checkout_recheck"),
        "obligation_log": log.read_text(encoding="utf-8", errors="replace").strip()
        if log.exists() else "",
        "expected": "PASSED: the transient rewrite is reverted before the re-check",
    }

    # (b) extra directory created then deleted; and one left behind -----
    repo = ctx.new_repo("extra-dirs")
    ctx.policy(repo, [
        {"id": "create", "program": "python",
         "args": ["-c",
                  "import pathlib; pathlib.Path('.tmp-cache').mkdir(); "
                  "pathlib.Path('.leftover').mkdir(); print('created')"],
         "required": True, "timeout_s": 30},
        {"id": "delete", "program": "python",
         "args": ["-c",
                  "import os; os.rmdir('.tmp-cache'); print('removed')"],
         "required": True, "timeout_s": 30},
    ])
    tree = ctx.tree(repo)
    run = ctx.run_on(post, repo, "run")
    rpath = ctx.receipt_path(repo, tree)
    receipt = json.loads(rpath.read_text(encoding="utf-8")) if rpath.exists() else {}
    result["probes"]["b_extra_directories"] = {
        "run": P.brief(run),
        "receipt": rpath.exists(),
        "checkout_recheck": receipt.get("checkout_recheck"),
        "expected": "PASSED: extra entries are allowed at the re-check (build "
                    "outputs); the deleted one is gone, the leftover is ignored",
    }

    # (c1) hook re-creates a declared directory component ----------------
    repo = ctx.new_repo("dir-collision-ok")
    (repo / "nested").mkdir()
    (repo / "nested" / "file.txt").write_text("nested\n", encoding="utf-8")
    ctx.git(repo, "add", "-A")
    ctx.git(repo, "commit", "-q", "-m", "nested")
    ctx.hook(repo, "mkdir -p nested")
    ctx.policy(repo, [P.GREEN])
    tree = ctx.tree(repo)
    run = ctx.run_on(post, repo, "run")
    rpath = ctx.receipt_path(repo, tree)
    receipt = json.loads(rpath.read_text(encoding="utf-8")) if rpath.exists() else {}
    result["probes"]["c1_declared_dir_component"] = {
        "run": P.brief(run),
        "receipt": rpath.exists(),
        "checkout_recheck": receipt.get("checkout_recheck"),
        "expected": "PASSED: `nested` is declared (implied by nested/file.txt); "
                    "re-creating it must not be a false positive",
    }

    # (c2) hook creates a child directory under a declared component -----
    repo = ctx.new_repo("dir-collision-extra")
    (repo / "nested").mkdir()
    (repo / "nested" / "file.txt").write_text("nested\n", encoding="utf-8")
    ctx.git(repo, "add", "-A")
    ctx.git(repo, "commit", "-q", "-m", "nested")
    ctx.hook(repo, "mkdir -p nested/extra")
    ctx.policy(repo, [P.GREEN])
    tree = ctx.tree(repo)
    run = ctx.run_on(post, repo, "run")
    result["probes"]["c2_undeclared_child_dir"] = {
        "run": P.brief(run),
        "receipt": ctx.receipt_path(repo, tree).exists(),
        "expected": "REFUSED/UNSUPPORTED_CHECKOUT: nested/extra is not declared",
    }

    # (e) hook races the .git unlink and recreates a `.git` directory -----
    # `materialise` removes the worktree `.git` link and then writes the scratch
    # ownership marker; the hook's background process waits for that marker (a
    # deterministic "after the unlink, before the check" point) and creates a
    # `.git` directory. A `.git` entry present at check time must be refused.
    repo = ctx.new_repo("dot-git-race")
    ctx.hook(repo,
             "( while [ ! -f .flow-scratch-owned ]; do :; done; mkdir -p .git; "
             "printf 'ref: refs/heads/main\\n' > .git/HEAD ) >/dev/null 2>&1 &")
    ctx.policy(repo, [{
        "id": "probe", "program": "python",
        "args": ["-c",
                 "import pathlib,sys; p=pathlib.Path('.git'); "
                 "print('git_dir_seen=%s' % p.is_dir()); "
                 "raise SystemExit(0 if p.is_dir() else 7)"],
        "required": True, "timeout_s": 30,
    }])
    tree = ctx.tree(repo)
    run = ctx.run_on(post, repo, "run", timeout=120)
    log = repo / ".flow" / "logs" / tree / "probe.stdout.log"
    result["probes"]["e_dot_git_race"] = {
        "run": P.brief(run),
        "receipt": ctx.receipt_path(repo, tree).exists(),
        "obligation_log": log.read_text(encoding="utf-8", errors="replace").strip()
        if log.exists() else "",
        "expected": "REFUSED/UNSUPPORTED_CHECKOUT: `materialise` removes the "
                    "`.git` link before the check, so a `.git` entry present at "
                    "check time is an extra entry (was skipped at 7a034ff)",
    }

    # (e2) `.git` created *after* the initial check, by an obligation -------
    # Extras created during the obligations are allowed at the re-check
    # (build outputs); this is the documented "after the check" limit.
    repo = ctx.new_repo("dot-git-after")
    ctx.policy(repo, [{
        "id": "make-git", "program": "python",
        "args": ["-c",
                 "import pathlib; g=pathlib.Path('.git'); g.mkdir(exist_ok=True); "
                 "(g/'HEAD').write_text('ref: refs/heads/main\\n'); "
                 "print('git_dir_seen=%s' % g.is_dir())"],
        "required": True, "timeout_s": 30,
    }])
    tree = ctx.tree(repo)
    run = ctx.run_on(post, repo, "run")
    log = repo / ".flow" / "logs" / tree / "make-git.stdout.log"
    rpath = ctx.receipt_path(repo, tree)
    receipt = json.loads(rpath.read_text(encoding="utf-8")) if rpath.exists() else {}
    result["probes"]["e2_dot_git_after_check"] = {
        "run": P.brief(run),
        "receipt": rpath.exists(),
        "checkout_recheck": receipt.get("checkout_recheck"),
        "obligation_log": log.read_text(encoding="utf-8", errors="replace").strip()
        if log.exists() else "",
        "expected": "PASSED: the re-check proves only declared files; extras "
                    "created while the obligations run are allowed (documented)",
    }

    out = Path(argv[2]) if len(argv) > 2 else Path(
        "experiments/verify-p0/results-recheck-defeat.json")
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n",
                   encoding="utf-8")
    print(json.dumps(result, indent=2, ensure_ascii=False)[:5000])
    print(f"\nwritten: {out}")
    if not P.os.environ.get("FLOW_P0_KEEP"):
        P.safe_remove_tree(root)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
