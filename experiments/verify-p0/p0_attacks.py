#!/usr/bin/env python3
"""Independent adversarial review of the P0 checkout-fidelity fix (wave 3).

Attacks `flow run` / `flow publish` from the outside, on disposable fixtures,
with the real binary.  Two binaries are used:

  post  the reviewed head (dfb941d: "fix: prove the verification checkout
        equals the verified tree"), built by this session;
  pre   its parent 0de8c21, built from `git archive` so the comparison of a
        scenario before/after the patch is a measurement, not a claim.

Stdlib only.  Every scenario records setup / command / expected / observed /
verdict, and never counts NOT_RUN as a pass.

Usage:
  python experiments/verify-p0/p0_attacks.py \
      --post wt-verify-target/debug/flow.exe \
      --pre  %TEMP%/flow-prefix-target/debug/flow.exe \
      --out  experiments/verify-p0/results-p0.json
"""

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

GREEN = {"id": "green", "program": "git", "args": ["--version"],
         "required": True, "timeout_s": 30}
CONFIG_TEMPLATE = {
    "schema": "flow-config/1",
    "policy_version": "FLOW-POLICY-1",
    "execution_boundary": "local-trusted",
    "disabled": False,
}


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def now_ms() -> float:
    return time.monotonic() * 1000.0


def safe_remove_tree(path: Path) -> None:
    def clear(p: Path) -> None:
        try:
            meta = p.lstat()
        except OSError:
            return
        if not p.is_symlink():
            try:
                perms = meta.st_mode
                if not perms & 0o200:
                    os.chmod(p, perms | 0o200)
            except OSError:
                pass
        if p.is_dir() and not p.is_symlink():
            try:
                for entry in p.iterdir():
                    clear(entry)
            except OSError:
                pass
    clear(path)
    shutil.rmtree(path, ignore_errors=True)


class Ctx:
    def __init__(self, post: Path, pre: Path | None, root: Path):
        self.post = post
        self.pre = pre
        self.root = root
        self.commands = []

    # -- process helpers -------------------------------------------------
    def run(self, argv, cwd, timeout=180, stdin=None):
        started = now_ms()
        try:
            proc = subprocess.run([str(a) for a in argv], cwd=str(cwd),
                                  input=stdin, stdout=subprocess.PIPE,
                                  stderr=subprocess.PIPE, timeout=timeout)
            rec = {"argv": [str(a) for a in argv], "exit": proc.returncode,
                   "stdout": proc.stdout.decode("utf-8", "replace"),
                   "stderr": proc.stderr.decode("utf-8", "replace")}
        except subprocess.TimeoutExpired as exc:
            rec = {"argv": [str(a) for a in argv], "exit": None,
                   "stdout": (exc.stdout or b"").decode("utf-8", "replace"),
                   "stderr": (exc.stderr or b"").decode("utf-8", "replace"),
                   "timeout": True}
        rec["duration_ms"] = round(now_ms() - started)
        self.commands.append(rec)
        return rec

    def git(self, repo: Path, *args, stdin=None, allow_fail=False):
        rec = self.run(["git", "-C", str(repo), *args], repo, stdin=stdin)
        if rec["exit"] != 0 and not allow_fail:
            raise RuntimeError(f"git {args} failed: {rec['stderr'][:400]}")
        return rec

    def flow(self, binary: Path, repo: Path, *args, timeout=180):
        rec = self.run([binary, *args], repo, timeout=timeout)
        parsed = None
        text = rec["stdout"].strip()
        if text:
            try:
                parsed = json.loads(text)
            except json.JSONDecodeError:
                parsed = None
        rec["json"] = parsed
        rec["status"] = (parsed or {}).get("status")
        rec["code"] = (parsed or {}).get("code")
        rec["reason"] = (parsed or {}).get("reason") or ""
        return rec

    # -- fixtures --------------------------------------------------------
    def new_repo(self, tag: str, seed: bytes = b"alpha\n") -> Path:
        repo = self.root / f"repo-{tag}"
        repo.mkdir(parents=True)
        self.git(repo, "init", "-q", "-b", "main")
        self.git(repo, "config", "user.name", "Flow P0 Review")
        self.git(repo, "config", "user.email", "p0@example.invalid")
        self.git(repo, "config", "commit.gpgsign", "false")
        self.git(repo, "config", "core.autocrlf", "false")
        (repo / "a.txt").write_bytes(seed)
        self.git(repo, "add", "-A")
        self.git(repo, "commit", "-q", "-m", "seed")
        return repo

    def policy(self, repo: Path, obligations) -> None:
        flow_dir = repo / ".flow"
        flow_dir.mkdir(exist_ok=True)
        config = dict(CONFIG_TEMPLATE, obligations=obligations)
        (flow_dir / "config.json").write_text(json.dumps(config), encoding="utf-8")
        rec = self.flow(self.post, repo, "init", "--adopt-policy")
        if rec["exit"] != 0:
            raise RuntimeError(f"fixture policy not adoptable: {rec['stdout'][:400]}")

    def hook(self, repo: Path, body: str, tracked: bool = True) -> None:
        hooks = repo / ".hooks"
        hooks.mkdir(exist_ok=True)
        (hooks / "post-checkout").write_text("#!/bin/sh\n" + body + "\n",
                                             encoding="utf-8", newline="\n")
        if tracked:
            self.git(repo, "add", "-A")
            self.git(repo, "update-index", "--chmod=+x", "--", ".hooks/post-checkout")
            self.git(repo, "commit", "-q", "-m", "fixture hook")
            self.git(repo, "config", "core.hooksPath", ".hooks")

    def tree(self, repo: Path) -> str:
        return self.git(repo, "write-tree")["stdout"].strip()

    def receipt_path(self, repo: Path, tree: str) -> Path:
        return repo / ".flow" / "receipts" / f"{tree}.json"

    def run_on(self, binary: Path, repo: Path, *args, timeout=180):
        return self.flow(binary, repo, *args, timeout=timeout)


RESULTS = []


def record(scn_id, title, severity, setup, expected, observed, verdict,
           evidence=None, gate="", notes=""):
    RESULTS.append({
        "id": scn_id,
        "title": title,
        "severity": severity,
        "setup": setup,
        "expected": expected,
        "observed": observed,
        "verdict": verdict,
        "gate": gate,
        "notes": notes,
        "evidence": evidence or {},
    })


def brief(rec, limit=420):
    if rec is None:
        return None
    out = {"exit": rec.get("exit"), "status": rec.get("status"),
           "code": rec.get("code"), "reason": rec.get("reason", "")[:limit],
           "duration_ms": rec.get("duration_ms")}
    if rec.get("timeout"):
        out["timeout"] = True
    return out


def reason_has(rec, needle: str) -> bool:
    return needle.lower() in (rec.get("reason") or "").lower()


def evil_filter(ctx: "Ctx", repo: Path) -> None:
    """A required same-length smudge driver with a pass-through clean side.

    The clean side is needed because `git write-tree`/`status` refresh the index
    and would otherwise fail on a required driver whose clean command is unset --
    that failure happens *before* the checkout and would mask the gate under test.
    """
    ctx.git(repo, "config", "filter.evil.smudge", str(SMUDGE).replace("\\", "/"))
    ctx.git(repo, "config", "filter.evil.clean", str(PASSTHRU).replace("\\", "/"))
    ctx.git(repo, "config", "filter.evil.required", "true")


def verdict_from(post: dict, pre: dict, gate: str, success_note: str,
                 pre_bug_note: str):
    """Shared verdict shaping for the pre/post comparison scenarios."""
    post_refused = (post["exit"] == 1 and post["code"] == gate)
    post_passed = post["exit"] == 0 and post["status"] == "PASSED"
    pre_passed = pre is not None and pre["exit"] == 0 and pre["status"] == "PASSED"
    evidence = {"post": brief(post), "pre": brief(pre)}
    if post_refused and pre_passed:
        return ("REFUTED",
                f"post-fix refuses with {gate} (attack not certified); "
                f"the pre-fix build reports PASSED, which is the regression the "
                f"patch closes",
                evidence, pre_bug_note)
    if post_refused:
        return ("REFUTED", f"post-fix refuses with {gate}; pre-fix build also "
                f"refused (gate pre-existed), so only the refusal is evidenced",
                evidence, pre_bug_note)
    if post_passed:
        return ("DEFECT_REPRODUCED",
                f"post-fix reports PASSED ({success_note})", evidence,
                pre_bug_note)
    return ("NOT_RUN", "neither a typed refusal nor PASSED was observed: "
            f"exit={post['exit']} status={post['status']} code={post['code']}",
            evidence, pre_bug_note)


# ======================================================================
# Axis A1 - local attribute sources and filters
# ======================================================================

def a01_local_info_attributes_filter(ctx: Ctx):
    repo = ctx.new_repo("info-filter")
    ctx.policy(repo, [GREEN])
    evil_filter(ctx, repo)
    info = repo / ".git" / "info"
    info.mkdir(parents=True, exist_ok=True)
    (info / "attributes").write_text("a.txt filter=evil\n", encoding="utf-8")
    tree = ctx.tree(repo)
    post = ctx.run_on(ctx.post, repo, "run")
    post_receipt = ctx.receipt_path(repo, tree).exists()
    pre = ctx.run_on(ctx.pre, repo, "run") if ctx.pre else None
    v, msg, ev, note = verdict_from(post, pre, "UNSUPPORTED_CHECKOUT",
                                    "smudge filter rewrote alpha->ALPHA",
                                    "pre-fix heuristic saw no core.autocrlf / "
                                    "core.eol / in-tree .gitattributes, skipped "
                                    "the comparison, and certified content the "
                                    "obligations never saw")
    ev["post_receipt_present"] = post_receipt
    ev["disk_bytes_after_checkout"] = "ALPHA\\n (6 bytes, same length as blob)"
    record("A01", "Local .git/info/attributes names a smudge filter that "
           "rewrites bytes at equal length", "P0",
           ".git/info/attributes: `a.txt filter=evil`; "
           "filter.evil.smudge=<same-length upper-caser>; required=true; "
           "core.autocrlf=false",
           "run must refuse the checkout (byte comparison), write no receipt",
           msg, v, ev, "UNSUPPORTED_CHECKOUT" if post["code"] ==
           "UNSUPPORTED_CHECKOUT" else "", note)


def a02_core_attributes_file_filter(ctx: Ctx):
    repo = ctx.new_repo("core-attrs-filter")
    ctx.policy(repo, [GREEN])
    attrs = ctx.root / "host-attributes"
    attrs.write_text("a.txt filter=evil\n", encoding="utf-8")
    evil_filter(ctx, repo)
    ctx.git(repo, "config", "core.attributesFile",
            str(attrs).replace("\\", "/"))
    tree = ctx.tree(repo)
    post = ctx.run_on(ctx.post, repo, "run")
    post_receipt = ctx.receipt_path(repo, tree).exists()
    pre = ctx.run_on(ctx.pre, repo, "run") if ctx.pre else None
    v, msg, ev, note = verdict_from(post, pre, "UNSUPPORTED_CHECKOUT",
                                    "core.attributesFile smudge rewrote the file",
                                    "pre-fix skipped the comparison for a "
                                    "host-local attribute file outside the tree")
    ev["attributes_file_outside_repo"] = str(attrs)
    ev["post_receipt_present"] = post_receipt
    record("A02", "core.attributesFile outside the repository drives a "
           "same-length smudge filter", "P0",
           "core.attributesFile=<temp>/host-attributes (outside repo) with "
           "`a.txt filter=evil`; filter.evil.smudge same-length rewriter",
           "run must refuse the checkout, no receipt", msg, v, ev,
           "UNSUPPORTED_CHECKOUT" if post["code"] == "UNSUPPORTED_CHECKOUT" else "",
           note)


def a03_local_info_attributes_eol(ctx: Ctx):
    repo = ctx.new_repo("info-eol")
    ctx.policy(repo, [GREEN])
    (repo / ".git" / "info").mkdir(exist_ok=True)
    (repo / ".git" / "info" / "attributes").write_text(
        "a.txt text eol=crlf\n", encoding="utf-8")
    tree = ctx.tree(repo)
    post = ctx.run_on(ctx.post, repo, "run")
    pre = ctx.run_on(ctx.pre, repo, "run") if ctx.pre else None
    v, msg, ev, note = verdict_from(post, pre, "UNSUPPORTED_CHECKOUT",
                                    "eol conversion",
                                    "pre-fix certified an LF blob checked out "
                                    "with CRLF")
    record("A03", "Local .git/info/attributes eol conversion", "P0",
           ".git/info/attributes: `a.txt text eol=crlf`; core.autocrlf=false",
           "run must refuse the checkout, no receipt", msg, v, ev,
           "UNSUPPORTED_CHECKOUT" if post["code"] == "UNSUPPORTED_CHECKOUT" else "",
           note)


def a04_committed_working_tree_encoding(ctx: Ctx):
    repo = ctx.root / "repo-wte"
    repo.mkdir()
    ctx.git(repo, "init", "-q", "-b", "main")
    ctx.git(repo, "config", "user.name", "Flow P0 Review")
    ctx.git(repo, "config", "user.email", "p0@example.invalid")
    ctx.git(repo, "config", "core.autocrlf", "false")
    (repo / "a.txt").write_bytes(b"\xff\xfe" + "alpha\n".encode("utf-16-le"))
    (repo / ".gitattributes").write_text("*.txt working-tree-encoding=UTF-16\n",
                                         encoding="utf-8")
    ctx.git(repo, "add", "-A")
    ctx.git(repo, "commit", "-q", "-m", "seed")
    ctx.policy(repo, [GREEN])
    tree = ctx.tree(repo)
    post = ctx.run_on(ctx.post, repo, "run")
    # Pre-fix heuristic fires on an in-tree .gitattributes, so no pre run needed
    # for the security claim; skip it to keep the fixture honest.
    pre = None
    v = "REFUTED" if (post["exit"] == 1 and post["code"] == "UNSUPPORTED_CHECKOUT") \
        else ("DEFECT_REPRODUCED" if post["status"] == "PASSED" else "NOT_RUN")
    observed = (f"exit={post['exit']} code={post['code']} reason="
                f"{(post['reason'] or '')[:240]}")
    if post["status"] == "PASSED":
        observed = ("PASSED although the tree is UTF-8 and the checkout is "
                    "UTF-16 (git may have ignored working-tree-encoding on this "
                    "host: check `git check-attr`)")
    record("A04", "Committed .gitattributes working-tree-encoding re-encodes "
           "the checkout", "P1",
           "`.gitattributes`: `*.txt working-tree-encoding=UTF-16`",
           "run must refuse any checkout whose bytes differ from the blob",
           observed, v, {"post": brief(post),
                         "check_attr": ctx.git(repo, "check-attr",
                                               "working-tree-encoding",
                                               "--", "a.txt")["stdout"].strip()},
           "UNSUPPORTED_CHECKOUT" if post["code"] == "UNSUPPORTED_CHECKOUT" else "")


# ======================================================================
# Axis A2 - post-checkout hook side effects at check time
# ======================================================================

def a05_hook_adds_file(ctx: Ctx):
    repo = ctx.new_repo("hook-add")
    ctx.hook(repo, "printf 'injected\\n' > injected.txt")
    ctx.policy(repo, [GREEN])
    tree = ctx.tree(repo)
    post = ctx.run_on(ctx.post, repo, "run")
    pre = ctx.run_on(ctx.pre, repo, "run") if ctx.pre else None
    v, msg, ev, note = verdict_from(post, pre, "UNSUPPORTED_CHECKOUT",
                                    "hook created injected.txt",
                                    "pre-fix only checked missing entries, so the "
                                    "extra file was certified")
    record("A05", "post-checkout creates a file the tree does not declare", "P1",
           "core.hooksPath=.hooks; tracked post-checkout writes injected.txt",
           "run must refuse: the checkout holds a file the tree does not declare",
           msg, v, ev,
           "UNSUPPORTED_CHECKOUT" if post["code"] == "UNSUPPORTED_CHECKOUT" else "",
           note)


def a06_hook_same_length_rewrite(ctx: Ctx):
    repo = ctx.new_repo("hook-rewrite")
    ctx.hook(repo, "printf 'ALPHA\\n' > a.txt")
    ctx.policy(repo, [GREEN])
    tree = ctx.tree(repo)
    post = ctx.run_on(ctx.post, repo, "run")
    pre = ctx.run_on(ctx.pre, repo, "run") if ctx.pre else None
    v, msg, ev, note = verdict_from(post, pre, "UNSUPPORTED_CHECKOUT",
                                    "hook rewrote a.txt with equal length",
                                    "pre-fix skipped the comparison (no attribute "
                                    "source) and certified a rewritten file")
    record("A06", "post-checkout rewrites a file keeping the same byte length",
           "P0", "core.hooksPath=.hooks; tracked post-checkout writes ALPHA\\n "
           "over a.txt (blob alpha\\n)",
           "run must refuse the checkout by byte comparison", msg, v, ev,
           "UNSUPPORTED_CHECKOUT" if post["code"] == "UNSUPPORTED_CHECKOUT" else "",
           note)


def a07_hook_toctou(ctx: Ctx):
    repo = ctx.new_repo("hook-toctou")
    ctx.hook(repo, "( sleep 6; printf 'ALPHA\\n' > a.txt ) >/dev/null 2>&1 &")
    obligation = {
        "id": "toctou", "program": "python",
        "args": ["-c",
                 "import time,pathlib; time.sleep(9); "
                 "d=pathlib.Path('a.txt').read_bytes(); "
                 "print('content_hex='+d.hex()); "
                 "raise SystemExit(0 if d==b'ALPHA\\n' else 9)"],
        "required": True, "timeout_s": 60,
    }
    ctx.policy(repo, [obligation])
    tree = ctx.tree(repo)
    post = ctx.run_on(ctx.post, repo, "run", timeout=120)
    log = repo / ".flow" / "logs" / tree / "toctou.stdout.log"
    log_text = log.read_text(encoding="utf-8", errors="replace") if log.exists() else ""
    ev = {"post": brief(post), "obligation_log": log_text.strip()[:300],
          "receipt_status": (post["json"] or {}).get("status"),
          "receipt_present": ctx.receipt_path(repo, tree).exists()}
    # 7a034ff: the post-run re-check refuses AFTER the obligation already read the
    # rewritten bytes, so the log still proves what the obligation saw.
    detected = (post["exit"] == 1
                and post["code"] in ("CHECKOUT_MUTATED_DURING_RUN", "UNSUPPORTED_CHECKOUT")
                and "414c5048410a" in log_text
                and not ctx.receipt_path(repo, tree).exists())
    if detected:
        record("A07", "TOCTOU: post-checkout spawns a background process that "
               "rewrites a file after the fidelity check", "P1",
               "hook: `( sleep 6; printf 'ALPHA\\n' > a.txt ) &`; obligation "
               "sleeps 9 s then asserts a.txt == ALPHA\\n",
               "run must not certify bytes it did not check",
               f"REFUSED/{post['code']} after the obligation read ALPHA\\n "
               f"(log {log_text.strip()[:80]!r}); no receipt. The delayed write was "
               f"not prevented (the obligation saw it) but the run is refused, so "
               f"no proof is issued. Reason: {(post['reason'] or '')[:240]}",
               "REFUTED", ev, post["code"] or "")
    elif post["exit"] == 0 and post["status"] == "PASSED" and "414c5048410a" in log_text:
        record("A07", "TOCTOU: post-checkout spawns a background process that "
               "rewrites a file after the fidelity check", "P1",
               "hook: `( sleep 6; printf 'ALPHA\\n' > a.txt ) &`; obligation "
               "sleeps 9 s then asserts a.txt == ALPHA\\n",
               "run must not certify bytes it did not check; at minimum the "
               "receipt must let a user detect post-check mutation",
               "PASSED, exit 0, receipt written, and the obligation's own log "
               "shows it read ALPHA\\n (not the tree's alpha\\n). The receipt "
               "is indistinguishable from a faithful run; no observable records "
               "the checkout bytes at obligation time.",
               "DEFECT_REPRODUCED", ev, "none (fidelity check is one-shot, "
               "before obligations)")
    else:
        record("A07", "TOCTOU: background rewrite after the fidelity check",
               "P1", "hook starts a delayed writer; obligation reads later",
               "must not certify unseen bytes",
               f"exit={post['exit']} status={post['status']} code={post['code']} "
               f"log={log_text.strip()[:160]!r}",
               "REFUTED" if post["exit"] == 1 else "NOT_RUN",
               ev, post["code"] or "")


def a08_hook_flow_scratch_blind_spot(ctx: Ctx):
    repo = ctx.new_repo("hook-scratch")
    ctx.hook(repo,
             "printf 'evil\\n' > .flow-scratch-injected.txt; "
             "mkdir -p .flow-scratch-dir; "
             "printf 'payload\\n' > .flow-scratch-dir/payload.txt")
    obligation = {
        "id": "probe", "program": "python",
        "args": ["-c",
                 "import pathlib,sys; "
                 "p=pathlib.Path('.flow-scratch-injected.txt'); "
                 "q=pathlib.Path('.flow-scratch-dir/payload.txt'); "
                 "print('seen_file=%s seen_nested=%s' % (p.exists(), q.exists())); "
                 "raise SystemExit(0 if p.exists() and q.exists() else 7)"],
        "required": True, "timeout_s": 30,
    }
    ctx.policy(repo, [obligation])
    tree = ctx.tree(repo)
    post = ctx.run_on(ctx.post, repo, "run")
    log = repo / ".flow" / "logs" / tree / "probe.stdout.log"
    log_text = log.read_text(encoding="utf-8", errors="replace") if log.exists() else ""
    ev = {"post": brief(post), "obligation_log": log_text.strip()[:300],
          "receipt_present": ctx.receipt_path(repo, tree).exists(),
          "new_rule": "7a034ff skips only the exact `.flow-scratch-owned` marker"}
    if post["exit"] == 0 and post["status"] == "PASSED" and "seen_file=True" in log_text:
        record("A08", "Extra files hidden under a `.flow-scratch*` root name are "
               "not detected", "P3",
               "hook writes `.flow-scratch-injected.txt` and "
               "`.flow-scratch-dir/payload.txt`; obligation asserts they exist",
               "the checkout must hold nothing the tree does not declare",
               "PASSED; the obligation saw both undeclared files. The extra-file "
               "check skips `.git` and any root name starting with "
               "`.flow-scratch`, so that prefix is a blind spot.",
               "DEFECT_REPRODUCED", ev, "none (name-prefix skip)")
    else:
        refused = (post["exit"] == 1 and post["code"] == "UNSUPPORTED_CHECKOUT"
                   and not ctx.receipt_path(repo, tree).exists())
        record("A08", "`.flow-scratch*` extra-file blind spot", "P3",
               "hook writes names with the skipped prefix",
               "must refuse: a name that merely starts with `.flow-scratch` is "
               "not Flow's marker",
               (f"REFUSED/{post['code']}: {(post['reason'] or '')[:300]}; no receipt"
                if refused else
                f"exit={post['exit']} status={post['status']} log={log_text.strip()[:200]!r}"),
               "REFUTED" if refused else "NOT_RUN", ev, post["code"] or "")


def a09_hook_empty_extra_directory(ctx: Ctx):
    repo = ctx.new_repo("hook-emptydir")
    ctx.hook(repo, "mkdir -p .build-cache")
    obligation = {
        "id": "probe", "program": "python",
        "args": ["-c",
                 "import pathlib,sys; d=pathlib.Path('.build-cache'); "
                 "print('empty_dir_seen=%s' % d.is_dir()); "
                 "raise SystemExit(0 if d.is_dir() else 7)"],
        "required": True, "timeout_s": 30,
    }
    ctx.policy(repo, [obligation])
    tree = ctx.tree(repo)
    post = ctx.run_on(ctx.post, repo, "run")
    log = repo / ".flow" / "logs" / tree / "probe.stdout.log"
    log_text = log.read_text(encoding="utf-8", errors="replace") if log.exists() else ""
    ev = {"post": brief(post), "obligation_log": log_text.strip()[:200],
          "note": "7a034ff compares the checkout's directory set with "
                  "`git ls-tree -r -d`"}
    if post["exit"] == 0 and post["status"] == "PASSED" and "empty_dir_seen=True" in log_text:
        record("A09", "Empty extra directory created by the hook is not "
               "detected", "P3",
               "hook: `mkdir -p .build-cache`; obligation asserts the directory",
               "the checkout must hold nothing the tree does not declare",
               "PASSED; `.build-cache/` exists in the checkout and is not "
               "declared. Only files are enumerated, so extra directories "
               "(empty ones, or directories whose files are skipped) survive.",
               "DEFECT_REPRODUCED", ev, "none (collect_files enumerates files)")
    else:
        refused = (post["exit"] == 1 and post["code"] == "UNSUPPORTED_CHECKOUT"
                   and "directory" in (post["reason"] or "")
                   and not ctx.receipt_path(repo, tree).exists())
        record("A09", "Empty extra directory", "P3", "hook creates an empty dir",
               "must refuse: an empty directory the tree does not declare is "
               "visible to an obligation that discovers paths",
               (f"REFUSED/{post['code']}: {(post['reason'] or '')[:300]}; no receipt"
                if refused else
                f"exit={post['exit']} status={post['status']} reason="
                f"{(post['reason'] or '')[:200]}"),
               "REFUTED" if refused else "NOT_RUN", ev, post["code"] or "")


# ======================================================================
# Axis A3 - fidelity budget edges
# ======================================================================

def a10_entry_budget(ctx: Ctx):
    repo = ctx.new_repo("budget-entries")
    ctx.policy(repo, [GREEN])
    blob = ctx.git(repo, "hash-object", "-w", "--stdin", stdin=b"x\n")["stdout"].strip()
    count = 20001
    lines = "".join(f"100644 {blob}\tblob-{i:05d}.txt\n" for i in range(count))
    add = ctx.git(repo, "update-index", "--index-info", stdin=lines.encode(),
                  allow_fail=True)
    if add["exit"] != 0:
        record("A10", f"{count} regular entries exceed the fidelity budget", "P1",
               "index built with `git update-index --index-info`", "refused, not "
               "skipped", f"git update-index refused: {add['stderr'][:200]}",
               "NOT_RUN", {"git_update_index": brief(add)}, "git")
        return
    tree = ctx.tree(repo)
    post = ctx.run_on(ctx.post, repo, "run", timeout=900)
    pre = ctx.run_on(ctx.pre, repo, "run", timeout=900) if ctx.pre else None
    ev = {"entries": count, "tree": tree, "post": brief(post, 600),
          "pre": brief(pre, 600)}
    post_refused = (post["exit"] == 1 and post["code"] == "UNSUPPORTED_CHECKOUT"
                    and "20000-entry fidelity budget" in post["reason"])
    if post_refused:
        v = "REFUTED"
        msg = (f"post-fix refuses above the 20000-entry budget: `{post['reason'][:220]}`")
        if pre is not None:
            if pre["exit"] == 0 and pre["status"] == "PASSED":
                msg += ("; the pre-fix build reports PASSED because the "
                        "comparison (including its budget) was skipped")
            else:
                msg += (f"; pre-fix exit={pre['exit']} status={pre['status']}")
    elif post["status"] == "PASSED":
        v, msg = "DEFECT_REPRODUCED", "PASSED above the declared entry budget"
    else:
        v = "NOT_RUN"
        msg = f"exit={post['exit']} code={post['code']} reason={post['reason'][:240]}"
    record("A10", f"{count} regular files: entry-budget edge", "P1",
           "index with 20001 distinct paths pointing at one small blob",
           "the comparison must be refused (not skipped) above 20000 entries",
           msg, v, ev, "UNSUPPORTED_CHECKOUT" if post["code"] ==
           "UNSUPPORTED_CHECKOUT" else "")


def a11_byte_budget(ctx: Ctx):
    repo = ctx.new_repo("budget-bytes")
    ctx.policy(repo, [GREEN])
    size = 65 * 1024 * 1024
    (repo / "big.bin").write_bytes(b"a" * size)
    ctx.git(repo, "add", "-A")
    tree = ctx.tree(repo)
    post = ctx.run_on(ctx.post, repo, "run", timeout=600)
    post_receipt = ctx.receipt_path(repo, tree).exists()
    pre = ctx.run_on(ctx.pre, repo, "run", timeout=600) if ctx.pre else None
    ev = {"file_bytes": size, "post": brief(post, 600), "pre": brief(pre, 600),
          "post_receipt": post_receipt}
    post_refused = post["exit"] == 1
    if post_refused:
        msg = (f"post-fix refuses: code={post['code']} reason="
               f"{(post['reason'] or '')[:300]}")
        v = "REFUTED"
        if pre is not None and pre["exit"] == 0 and pre["status"] == "PASSED":
            msg += ("; the pre-fix build CERTIFIED the 65 MiB tree after "
                    "skipping the byte comparison")
    else:
        v = "DEFECT_REPRODUCED" if post["status"] == "PASSED" else "NOT_RUN"
        msg = f"exit={post['exit']} status={post['status']} code={post['code']}"
    record("A11", "65 MiB file: content-budget edge", "P1",
           "one 65 MiB file in the index tree (over the 64 MiB retained-bytes "
           "budget and over the 64 MiB git output cap)",
           "refused (fail closed), never certified after a skipped comparison",
           msg, v, ev, post["code"] or "")


# ======================================================================
# Axis A4 - entry types and path tricks
# ======================================================================

def a12_symlink_entry(ctx: Ctx):
    repo = ctx.new_repo("symlink")
    ctx.policy(repo, [GREEN])
    target = ctx.git(repo, "hash-object", "-w", "--stdin",
                     stdin=b"a.txt\n")["stdout"].strip()
    add = ctx.git(repo, "update-index", "--add", "--cacheinfo",
                  f"120000,{target},link", allow_fail=True)
    if add["exit"] != 0:
        record("A12", "Symlink tree entry", "P1", "mode 120000 entry", "refused",
               f"git update-index refused: {add['stderr'][:200]}", "NOT_RUN",
               {"git": brief(add)}, "git")
        return
    tree = ctx.tree(repo)
    post = ctx.run_on(ctx.post, repo, "run")
    ev = {"post": brief(post), "tree": tree,
          "core_symlinks": ctx.git(repo, "config", "--get",
                                   "core.symlinks", allow_fail=True)["stdout"].strip()}
    if post["exit"] == 1 and post["code"] == "UNSUPPORTED_CHECKOUT":
        record("A12", "Symlink tree entry checked out as a regular file", "P1",
               "mode 120000 `link`; core.symlinks is false on Windows",
               "refused, not certified", f"refused: {post['reason'][:240]}",
               "REFUTED", ev, "UNSUPPORTED_CHECKOUT")
    else:
        record("A12", "Symlink tree entry", "P1", "mode 120000", "refused",
               f"exit={post['exit']} status={post['status']} reason={post['reason'][:240]}",
               "DEFECT_REPRODUCED" if post["status"] == "PASSED" else "NOT_RUN",
               ev, post["code"] or "")


def a13_gitlink_entry(ctx: Ctx):
    repo = ctx.new_repo("gitlink")
    ctx.policy(repo, [GREEN])
    head = ctx.git(repo, "rev-parse", "HEAD")["stdout"].strip()
    add = ctx.git(repo, "update-index", "--add", "--cacheinfo",
                  f"160000,{head},sub", allow_fail=True)
    if add["exit"] != 0:
        record("A13", "Gitlink tree entry", "P1", "mode 160000", "refused",
               f"git update-index refused: {add['stderr'][:200]}", "NOT_RUN",
               {"git": brief(add)}, "git")
        return
    tree = ctx.tree(repo)
    post = ctx.run_on(ctx.post, repo, "run")
    ev = {"post": brief(post), "tree": tree}
    if post["exit"] == 1 and post["code"] == "UNSUPPORTED_CHECKOUT":
        record("A13", "Gitlink (submodule) entry is not materialised", "P1",
               "mode 160000 `sub` pointing at HEAD", "refused, not certified",
               f"refused: {post['reason'][:240]}", "REFUTED", ev,
               "UNSUPPORTED_CHECKOUT")
    else:
        record("A13", "Gitlink entry", "P1", "mode 160000", "refused",
               f"exit={post['exit']} status={post['status']} reason={post['reason'][:240]}",
               "DEFECT_REPRODUCED" if post["status"] == "PASSED" else "NOT_RUN",
               ev, post["code"] or "")


def a14_mode_100755_positive(ctx: Ctx):
    repo = ctx.new_repo("mode-100755")
    (repo / "exec.sh").write_text("#!/bin/sh\necho hi\n", encoding="utf-8",
                                  newline="\n")
    ctx.git(repo, "add", "-A")
    ctx.git(repo, "update-index", "--chmod=+x", "--", "exec.sh")
    ctx.git(repo, "commit", "-q", "-m", "exec")
    ctx.policy(repo, [GREEN])
    tree = ctx.tree(repo)
    post = ctx.run_on(ctx.post, repo, "run")
    rpath = ctx.receipt_path(repo, tree)
    receipt = json.loads(rpath.read_text(encoding="utf-8")) if rpath.exists() else {}
    ev = {"post": brief(post), "tree": tree,
          "receipt": rpath.exists(),
          "checkout_recheck": receipt.get("checkout_recheck")}
    if post["exit"] == 0 and post["status"] == "PASSED" \
            and receipt.get("checkout_recheck") == "OK":
        record("A14", "Positive control: mode 100755 regular file is still "
               "certified and the receipt records the re-check", "P2",
               "index entry mode 100755, content unchanged by checkout",
               "PASSED with a receipt carrying `checkout_recheck: \"OK\"`",
               "PASSED, receipt written, `checkout_recheck: \"OK\"`",
               "REFUTED", ev, "")
        return
    if post["exit"] == 0 and post["status"] == "PASSED":
        record("A14", "Positive control: receipt re-check field", "P2",
               "clean checkout", "PASSED with `checkout_recheck: OK`",
               f"PASSED but receipt checkout_recheck={receipt.get('checkout_recheck')!r}",
               "DEFECT_REPRODUCED", ev, "")
        return
    record("A14", "Positive control: mode 100755 regular file", "P2",
           "index entry mode 100755", "PASSED with a receipt",
           f"exit={post['exit']} status={post['status']} code={post['code']} reason={post['reason'][:200]}",
           "NOT_RUN", ev, "")


def a15_reserved_and_dotdot_paths(ctx: Ctx):
    """Reserved/`..` tree paths, reached the only way this host allows.

    `git update-index` and `git read-tree` refuse these names, but `git mktree`
    builds a raw tree from the same bytes and a `.flow/candidates/*.json` record
    is a file a clone can ship. `flow run --candidate` is therefore asked to
    materialise a tree whose path cannot be represented here: it must refuse,
    never certify.
    """
    cases = [("NUL", b"NUL"), ("CON", b"CON"), ("dotdot", b".."),
             (".git", b".git"), ("trailing-dot", b"x."),
             ("backslash", b"a\\b.txt")]
    detail = {}
    accepted = 0
    for label, raw in cases:
        repo = ctx.new_repo("path-" + label.replace("/", "_").replace(".", "dot"))
        ctx.policy(repo, [GREEN])
        # Context: the index route, which Git refuses (wave-2 FIL-06 finding).
        blob = ctx.git(repo, "hash-object", "-w", "--stdin",
                       stdin=b"payload\n")["stdout"].strip()
        idx_line = b"100644 " + blob.encode() + b"\t" + raw + b"\n"
        idx = ctx.git(repo, "update-index", "--index-info", stdin=idx_line,
                      allow_fail=True)
        mk = ctx.git(repo, "mktree", stdin=b"100644 blob " + blob.encode()
                     + b"\t" + raw + b"\n", allow_fail=True)
        if mk["exit"] != 0:
            detail[label] = {"index_route": {"exit": idx["exit"],
                                             "stderr": idx["stderr"].strip()[:120]},
                             "mktree": {"exit": mk["exit"],
                                        "stderr": mk["stderr"].strip()[:160]},
                             "verdict": "NOT_RUN"}
            continue
        tree = mk["stdout"].strip()
        listed = ctx.git(repo, "ls-tree", "-r", "-z", tree,
                         allow_fail=True)["stdout"]
        if raw.decode("utf-8", "replace") not in listed:
            detail[label] = {"stage": "mktree tree does not name the path",
                             "ls_tree": listed[:160], "verdict": "NOT_RUN"}
            continue
        accepted += 1
        cid = hashlib.sha256(label.encode()).hexdigest()[:32]
        head = ctx.git(repo, "rev-parse", "HEAD")["stdout"].strip()
        cdir = repo / ".flow" / "candidates"
        cdir.mkdir(parents=True, exist_ok=True)
        (cdir / f"{cid}.json").write_text(json.dumps({
            "schema": "flow-candidate/1", "candidate_id": cid,
            "base_commit": head, "tree_oid": tree,
            "proposals": [], "changes": [],
        }), encoding="utf-8")
        run = ctx.run_on(ctx.post, repo, "run", "--candidate", cid)
        detail[label] = {
            "index_route": {"exit": idx["exit"],
                            "stderr": idx["stderr"].strip()[:120]},
            "tree": tree,
            "candidate_record": str(cdir / f"{cid}.json"),
            "run": brief(run),
            "receipt": ctx.receipt_path(repo, tree).exists(),
            "outside_file_created": (repo.parent / "evil.txt").exists(),
        }
        if run["status"] == "PASSED":
            detail[label]["verdict"] = "DEFECT_REPRODUCED"
        elif run["exit"] == 1:
            detail[label]["verdict"] = "REFUTED"
        else:
            detail[label]["verdict"] = "NOT_RUN"
    passed = [k for k, v in detail.items() if v.get("verdict") == "DEFECT_REPRODUCED"]
    refused = [k for k, v in detail.items() if v.get("verdict") == "REFUTED"]
    if accepted == 0:
        record("A15", "Windows-reserved and `..` tree paths", "P1",
               "mktree raw trees + a clone-shippable candidate record",
               "an unusable path in a verified tree must be refused, not "
               "certified",
               "neither `update-index` nor `mktree` could produce a tree naming "
               "the path on this host", "NOT_RUN", {"paths": detail}, "git")
        return
    record("A15", "Windows-reserved and `..` tree paths reached through a "
           "candidate record", "P1",
           "git mktree builds NUL / CON / `..` / .git / `x.` / `a\\b.txt`; a "
           "32-hex candidate record names each tree; flow run --candidate is "
           "asked to materialise it",
           "git worktree add refuses an invalid path, so run must refuse with "
           "no receipt (wave-2 FIL-06 was NOT_RUN)",
           f"refused: {refused}; pass-through: {passed}; codes=" +
           json.dumps({k: (v.get("run") or {}).get("code") for k, v in detail.items()}),
           "DEFECT_REPRODUCED" if passed else ("REFUTED" if refused else "NOT_RUN"),
           {"paths": detail}, "")


def a19_hook_makes_declared_path_a_directory(ctx: Ctx):
    repo = ctx.new_repo("hook-dirswap")
    ctx.hook(repo, "rm -f a.txt; mkdir a.txt")
    ctx.policy(repo, [GREEN])
    tree = ctx.tree(repo)
    post = ctx.run_on(ctx.post, repo, "run")
    ev = {"post": brief(post), "tree": tree,
          "receipt": ctx.receipt_path(repo, tree).exists()}
    if post["exit"] == 1 and post["code"] == "UNSUPPORTED_CHECKOUT":
        record("A19", "Hook replaces a declared regular file with a directory",
               "P1", "post-checkout: `rm a.txt; mkdir a.txt`",
               "the type gate must refuse; the comparison must never read a "
               "directory as if it were the blob",
               f"refused: {post['reason'][:240]}", "REFUTED", ev,
               "UNSUPPORTED_CHECKOUT")
    else:
        record("A19", "Hook replaces a declared file with a directory", "P1",
               "post-checkout directory swap", "refused",
               f"exit={post['exit']} status={post['status']} reason={post['reason'][:240]}",
               "DEFECT_REPRODUCED" if post["status"] == "PASSED" else "NOT_RUN",
               ev, post["code"] or "")


def a16_non_utf8_paths(ctx: Ctx):
    """A tree path whose bytes are not valid UTF-8.

    On Windows `git update-index` rewrites raw invalid bytes to U+FFFD, so the
    index route cannot carry a genuinely non-UTF-8 path here (the wave-2/3
    fixture instead produced a lossy case collision). A clone can still ship a
    candidate record naming a raw `git mktree` tree, which is the route that
    reaches the product's own UTF-8 refusal at `7a034ff`.
    """
    repo = ctx.new_repo("nonutf8")
    ctx.policy(repo, [GREEN])
    blob = ctx.git(repo, "hash-object", "-w", "--stdin",
                   stdin=b"payload\n")["stdout"].strip()

    # Evidence 1: the index route, where Git folds invalid bytes to U+FFFD.
    idx_input = (b"100644 " + blob.encode() + b"\tbad\xff.txt\n"
                 + b"100644 " + blob.encode() + b"\tbad\xfe.txt\n")
    idx = ctx.git(repo, "update-index", "--index-info", stdin=idx_input,
                  allow_fail=True)
    index_evidence = {"update_index_exit": idx["exit"]}
    if idx["exit"] == 0:
        idx_tree = ctx.tree(repo)
        idx_run = ctx.run_on(ctx.post, repo, "run")
        index_evidence.update({"tree": idx_tree, "run": brief(idx_run),
                               "kind": "lossy collapse (U+FFFD), not the UTF-8 "
                                       "gate"})
    else:
        index_evidence["stderr"] = idx["stderr"].strip()[:160]

    # Evidence 2: a raw tree naming a non-UTF-8 path, reached via a candidate.
    mk = ctx.git(repo, "mktree",
                 stdin=b"100644 blob " + blob.encode() + b"\tbad\xff-name.txt\n",
                 allow_fail=True)
    if mk["exit"] != 0:
        record("A16", "Non-UTF-8 tree path", "P2",
               "git mktree with raw invalid bytes", "refused with its own reason",
               f"git mktree refused: {mk['stderr'][:200]}", "NOT_RUN",
               {"index_route": index_evidence}, "git")
        return
    tree = mk["stdout"].strip()
    head = ctx.git(repo, "rev-parse", "HEAD")["stdout"].strip()
    cid = hashlib.sha256(b"nonutf8-7a").hexdigest()[:32]
    cdir = repo / ".flow" / "candidates"
    cdir.mkdir(parents=True, exist_ok=True)
    (cdir / f"{cid}.json").write_text(json.dumps({
        "schema": "flow-candidate/1", "candidate_id": cid,
        "base_commit": head, "tree_oid": tree, "proposals": [], "changes": [],
    }), encoding="utf-8")
    post = ctx.run_on(ctx.post, repo, "run", "--candidate", cid)
    ev = {"post": brief(post), "tree": tree, "candidate_route": True,
          "index_route": index_evidence,
          "receipt": ctx.receipt_path(repo, tree).exists()}
    if (post["exit"] == 1 and post["code"] == "UNSUPPORTED_CHECKOUT"
            and "not valid UTF-8" in (post["reason"] or "")
            and not ctx.receipt_path(repo, tree).exists()):
        record("A16", "Non-UTF-8 tree path", "P2",
               "raw mktree tree `bad\\xff-name.txt`, reached through a shipped "
               "candidate record",
               "refused with the explicit `not valid UTF-8` reason, no receipt",
               f"REFUSED/{post['code']}: {(post['reason'] or '')[:300]}; no receipt",
               "REFUTED", ev, post["code"] or "")
    elif post["status"] == "PASSED":
        record("A16", "Non-UTF-8 tree path", "P2", "raw invalid path bytes",
               "refused or represented faithfully",
               "PASSED: a tree path that is not valid UTF-8 was certified",
               "DEFECT_REPRODUCED", ev, "")
    else:
        record("A16", "Non-UTF-8 tree path", "P2", "raw invalid path bytes",
               "refused with the explicit reason",
               f"exit={post['exit']} code={post['code']} reason={(post['reason'] or '')[:240]}",
               "NOT_RUN", ev, post["code"] or "")


# ======================================================================
# Axis A5 - publish and documentation
# ======================================================================

def a17_publish_not_unverified(ctx: Ctx):
    # Positive: a PASSED run publishes exactly the verified tree. The index
    # carries one file HEAD does not, so publication has a real move to make.
    repo_ok = ctx.new_repo("publish-ok")
    ctx.policy(repo_ok, [GREEN])
    (repo_ok / "landed.txt").write_text("new\n", encoding="utf-8")
    ctx.git(repo_ok, "add", "landed.txt")
    tree_ok = ctx.tree(repo_ok)
    run_ok = ctx.run_on(ctx.post, repo_ok, "run")
    pub_ok = ctx.run_on(ctx.post, repo_ok, "publish", "--ref", "refs/heads/main")
    landed = ctx.git(repo_ok, "rev-parse", "refs/heads/main^{tree}",
                     allow_fail=True)["stdout"].strip()
    # Negative: a refused run leaves no receipt, so publish must refuse.
    repo_bad = ctx.new_repo("publish-bad")
    ctx.policy(repo_bad, [GREEN])
    (repo_bad / "landed.txt").write_text("new\n", encoding="utf-8")
    ctx.git(repo_bad, "add", "landed.txt")
    (repo_bad / ".git" / "info").mkdir(exist_ok=True)
    (repo_bad / ".git" / "info" / "attributes").write_text(
        "a.txt filter=evil\n", encoding="utf-8")
    ctx.git(repo_bad, "config", "filter.evil.smudge", str(SMUDGE).replace("\\", "/"))
    ctx.git(repo_bad, "config", "filter.evil.clean", str(PASSTHRU).replace("\\", "/"))
    ctx.git(repo_bad, "config", "filter.evil.required", "true")
    bad_head = ctx.git(repo_bad, "rev-parse", "refs/heads/main")["stdout"].strip()
    tree_bad = ctx.tree(repo_bad)
    run_bad = ctx.run_on(ctx.post, repo_bad, "run")
    pub_bad = ctx.run_on(ctx.post, repo_bad, "publish", "--ref", "refs/heads/main")
    head_after = ctx.git(repo_bad, "rev-parse", "refs/heads/main")["stdout"].strip()
    ev = {"run_ok": brief(run_ok), "publish_ok": brief(pub_ok),
          "verified_tree": tree_ok, "landed_tree": landed,
          "run_bad": brief(run_bad), "publish_bad": brief(pub_bad),
          "bad_ref_unchanged": head_after == bad_head,
          "bad_tree": tree_bad}
    ok = (run_ok["status"] == "PASSED" and pub_ok["status"] == "PUBLISHED"
          and landed == tree_ok
          and run_bad["exit"] == 1 and pub_bad["code"] == "NO_RECEIPT"
          and head_after == bad_head)
    record("A17", "publish cannot land a tree that was not verified", "P0",
           "one repo with a PASSED run then publish; one repo whose run was "
           "refused (local filter) then publish",
           "publish lands exactly the receipt's tree; after a refused run it "
           "refuses with NO_RECEIPT and moves nothing",
           ("verified tree published; refused-run repo kept its ref and "
            "publish returned NO_RECEIPT" if ok else
            f"publish_ok={brief(pub_ok)} landed={landed} expected={tree_ok} "
            f"publish_bad={brief(pub_bad)} ref_unchanged={head_after == bad_head}"),
           "REFUTED" if ok else "DEFECT_REPRODUCED", ev, "")


def a18_doctor_disclosure(ctx: Ctx):
    repo = ctx.new_repo("doctor")
    ctx.hook(repo, "printf 'x\\n' > injected.txt")
    ctx.git(repo, "config", "core.attributesFile",
            str((ctx.root / "host-attributes")).replace("\\", "/")
            if (ctx.root / "host-attributes").exists() else "C:/does-not-exist")
    (repo / ".git" / "info").mkdir(exist_ok=True)
    (repo / ".git" / "info" / "attributes").write_text("a.txt text eol=crlf\n",
                                                       encoding="utf-8")
    ctx.policy(repo, [GREEN])
    doc = ctx.run_on(ctx.post, repo, "doctor")
    j = doc["json"] or {}
    checks = {c.get("id"): c for c in (j.get("checks") or [])}
    non_guarantees = [g for g in (j.get("non_guarantees") or []) if "hook" in g]
    ev = {"exit": doc["exit"], "gitattributes_check":
          (checks.get("gitattributes") or {}).get("status"),
          "gitattributes_detail": (checks.get("gitattributes") or {}).get("detail"),
          "non_guarantees_hooks": non_guarantees}
    overstates = any("detected and refused" in g for g in non_guarantees)
    record("A18", "doctor disclosure vs measured TOCTOU behaviour", "P2",
           "doctor in a repo with .git/info/attributes + a tracked "
           "post-checkout hook",
           "the disclosure must not claim a hook change is always detected",
           ("doctor says a post-checkout that changes the checkout is "
            "'detected and refused', but A07 measured a delayed rewrite that is "
            "certified PASSED; the word 'detected' is unqualified"
            if overstates else "doctor does not make the unqualified claim"),
           "DEFECT_REPRODUCED" if overstates else "REFUTED", ev, "")


# ======================================================================
# Driver
# ======================================================================

SMUDGE: Path = Path(__file__).resolve().parent / "smudge.exe"
PASSTHRU: Path = Path(__file__).resolve().parent / "passthru.exe"


def main(argv=None):
    global SMUDGE
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--post", required=True)
    parser.add_argument("--pre", default="")
    parser.add_argument("--out", default="experiments/verify-p0/results-p0.json")
    parser.add_argument("--filter", default="")
    parser.add_argument("--only", default="",
                        help="comma-separated scenario ids (A01,A05,...)")
    args = parser.parse_args(argv)
    post = Path(args.post).resolve()
    pre = Path(args.pre).resolve() if args.pre else None
    if not post.is_file():
        print(f"error: no binary at {post}", file=sys.stderr)
        return 2
    if pre is not None and not pre.is_file():
        print(f"error: no pre-fix binary at {pre}", file=sys.stderr)
        return 2
    root = Path(tempfile.mkdtemp(prefix="flow-p0-review-"))
    ctx = Ctx(post, pre, root)
    scenarios = [
        a01_local_info_attributes_filter,
        a02_core_attributes_file_filter,
        a03_local_info_attributes_eol,
        a04_committed_working_tree_encoding,
        a05_hook_adds_file,
        a06_hook_same_length_rewrite,
        a07_hook_toctou,
        a08_hook_flow_scratch_blind_spot,
        a09_hook_empty_extra_directory,
        a10_entry_budget,
        a11_byte_budget,
        a12_symlink_entry,
        a13_gitlink_entry,
        a14_mode_100755_positive,
        a15_reserved_and_dotdot_paths,
        a16_non_utf8_paths,
        a17_publish_not_unverified,
        a18_doctor_disclosure,
        a19_hook_makes_declared_path_a_directory,
    ]
    if args.filter:
        scenarios = [s for s in scenarios if s.__name__.startswith(args.filter)
                     or s.__name__.startswith("a" + args.filter)]
    if args.only:
        wanted = {x.strip().upper() for x in args.only.split(",") if x.strip()}
        scenarios = [s for s in scenarios
                     if s.__name__.split("_")[0].upper() in wanted]
    started = time.time()
    for fn in scenarios:
        scn_started = time.monotonic()
        before = len(RESULTS)
        try:
            fn(ctx)
        except Exception as exc:  # noqa: BLE001 - a harness bug must be visible
            RESULTS.append({
                "id": fn.__name__.split("_")[0].upper(),
                "title": fn.__name__, "severity": "?", "setup": "",
                "expected": "", "observed": f"harness exception: {exc}",
                "verdict": "NOT_RUN", "gate": "harness", "notes": "",
                "evidence": {},
            })
        finally:
            for rec in RESULTS[before:]:
                rec["duration_ms"] = round((time.monotonic() - scn_started) * 1000)
                rec.setdefault("commands", ctx.commands[-6:])
    if not os.environ.get("FLOW_P0_KEEP"):
        safe_remove_tree(root)
    meta = {
        "review": "FLOW-P0-GIT-FIDELITY-REVIEW",
        "date": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(started)),
        "post": {"path": str(post), "sha256": sha256(post)},
        "pre": ({"path": str(pre), "sha256": sha256(pre)} if pre else None),
        "smudge_helper": {"path": str(SMUDGE), "sha256": sha256(SMUDGE)},
        "passthru_helper": {"path": str(PASSTHRU), "sha256": sha256(PASSTHRU)},
        "fixtures_root": str(root) if os.environ.get("FLOW_P0_KEEP") else "<removed>",
        "duration_s": round(time.time() - started, 1),
        "platform": sys.platform,
    }
    summary = {}
    for rec in RESULTS:
        summary[rec["verdict"]] = summary.get(rec["verdict"], 0) + 1
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps({"meta": meta, "summary": summary,
                               "scenarios": RESULTS}, indent=2,
                              ensure_ascii=False) + "\n", encoding="utf-8")
    width = max(len(r["id"]) for r in RESULTS) if RESULTS else 2
    print(f"{'ID':<{width}} {'VERDICT':26} {'SEV':4} TITLE")
    for rec in RESULTS:
        print(f"{rec['id']:<{width}} {rec['verdict']:26} {rec['severity']:4} "
              f"{rec['title'][:90]}")
    print(f"\n{len(RESULTS)} scenarios: " +
          ", ".join(f"{k}={v}" for k, v in sorted(summary.items())))
    print(f"results: {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
