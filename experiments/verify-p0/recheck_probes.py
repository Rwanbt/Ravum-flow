#!/usr/bin/env python3
"""Re-check probes for the bae7390/d7e1015 claims (wave-4, independent).

Targets the Lead's specific claims that are cheap to measure but not covered by
the 19-scenario P0 harness:

1. a clean `PASSED` receipt carries `checkout_recheck: "OK"`;
2. `run --dry-run`, `status --force`, `doctor --adopt-policy` are usage errors
   (exit 2) with no receipt / no execution; `publish --dry-run` still plans;
3. `doctor` guarantees/non_guarantees wording (quoted verbatim);
4. the non-UTF-8 tree-path reason;
5. a delayed write still certifies (A07 cross-check inside the same fixture).

Reuses the helpers of `p0_attacks.py` (no product code).
"""

import json
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import p0_attacks as P  # noqa: E402


def main(argv):
    post = Path(argv[1]).resolve()
    root = Path(tempfile.mkdtemp(prefix="flow-recheck-probes-"))
    ctx = P.Ctx(post, None, root)
    result = {"meta": {"post": str(post), "sha256": P.sha256(post)},
              "probes": {}}

    # 1. clean run: receipt fields -------------------------------------
    repo = ctx.new_repo("clean-receipt")
    ctx.policy(repo, [P.GREEN])
    tree = ctx.tree(repo)
    run = ctx.run_on(post, repo, "run")
    rpath = ctx.receipt_path(repo, tree)
    receipt = json.loads(rpath.read_text(encoding="utf-8")) if rpath.exists() else {}
    result["probes"]["clean_receipt"] = {
        "run": P.brief(run),
        "receipt_written": rpath.exists(),
        "receipt_status": receipt.get("status"),
        "receipt_keys": sorted(receipt.keys()),
        "checkout_recheck_present": "checkout_recheck" in receipt,
        "checkout_recheck_value": receipt.get("checkout_recheck"),
        "checkout_unchanged_present": any("checkout" in k for k in receipt),
    }

    # 2. per-command boolean flags -------------------------------------
    flags_repo = ctx.new_repo("flags")
    ctx.policy(flags_repo, [P.GREEN])
    flags_tree = ctx.tree(flags_repo)
    ref_before = ctx.git(flags_repo, "rev-parse", "refs/heads/main")["stdout"].strip()
    dry = ctx.run_on(post, flags_repo, "run", "--dry-run")
    status_force = ctx.run_on(post, flags_repo, "status", "--force")
    doctor_adopt = ctx.run_on(post, flags_repo, "doctor", "--adopt-policy")
    ref_after = ctx.git(flags_repo, "rev-parse", "refs/heads/main")["stdout"].strip()
    log_dir = flags_repo / ".flow" / "logs" / flags_tree
    result["probes"]["flags"] = {
        "run_dry_run": P.brief(dry),
        "status_force": P.brief(status_force),
        "doctor_adopt_policy": P.brief(doctor_adopt),
        "receipt_written": ctx.receipt_path(flags_repo, flags_tree).exists(),
        "obligation_logs_written": log_dir.exists(),
        "ref_unchanged": ref_before == ref_after,
    }
    # positive control: publish --dry-run still plans
    pub = ctx.run_on(post, flags_repo, "publish", "--dry-run")
    result["probes"]["flags"]["publish_dry_run"] = P.brief(pub)

    # 3. doctor wording -------------------------------------------------
    doc_repo = ctx.new_repo("doctor-strings")
    ctx.hook(doc_repo, "printf 'x\\n' > injected.txt")
    info = doc_repo / ".git" / "info"
    info.mkdir(parents=True, exist_ok=True)
    (info / "attributes").write_text("*.md text eol=crlf\n", encoding="utf-8")
    host = root / "host-attributes"
    host.write_text("data.txt text eol=crlf\n", encoding="utf-8")
    ctx.git(doc_repo, "config", "core.attributesFile", str(host))
    ctx.policy(doc_repo, [P.GREEN])
    doc = ctx.run_on(post, doc_repo, "doctor")
    j = doc["json"] or {}
    checks = {(c.get("id") or ""): c for c in (j.get("checks") or [])}
    result["probes"]["doctor"] = {
        "guarantees": j.get("guarantees"),
        "non_guarantees": j.get("non_guarantees"),
        "gitattributes_status": (checks.get("gitattributes") or {}).get("status"),
        "gitattributes_detail": (checks.get("gitattributes") or {}).get("detail"),
    }

    # 4. non-UTF-8 path reason -----------------------------------------
    repo = ctx.new_repo("nonutf8")
    ctx.policy(repo, [P.GREEN])
    blob = ctx.git(repo, "hash-object", "-w", "--stdin", stdin=b"payload\n")["stdout"].strip()
    add = ctx.git(repo, "update-index", "--index-info",
                  stdin=b"100644 " + blob.encode() + b"\tbad\xff.txt\n", allow_fail=True)
    result["probes"]["non_utf8"] = {"update_index": P.brief(add)}
    if add["exit"] == 0:
        tree = ctx.tree(repo)
        run = ctx.run_on(post, repo, "run")
        result["probes"]["non_utf8"].update({
            "run": P.brief(run), "tree": tree,
            "receipt": ctx.receipt_path(repo, tree).exists()})
        # what git stored for the path
        ls = ctx.git(repo, "ls-tree", "-r", "-z", tree, allow_fail=True)
        result["probes"]["non_utf8"]["ls_tree_hex"] = ls["stdout"].encode(
            "utf-8", "backslashreplace")[:120].hex()

    # 5. delayed write cross-check (same fixture shape as A07) ---------
    toctou = ctx.new_repo("toctou")
    ctx.hook(toctou, "( sleep 6; printf 'ALPHA\\n' > a.txt ) >/dev/null 2>&1 &")
    ctx.policy(toctou, [{
        "id": "toctou", "program": "python",
        "args": ["-c",
                 "import time,pathlib; time.sleep(9); "
                 "d=pathlib.Path('a.txt').read_bytes(); "
                 "print('content_hex='+d.hex()); "
                 "raise SystemExit(0 if d==b'ALPHA\\n' else 9)"],
        "required": True, "timeout_s": 60,
    }])
    tree = ctx.tree(toctou)
    run = ctx.run_on(post, toctou, "run", timeout=120)
    log = toctou / ".flow" / "logs" / tree / "toctou.stdout.log"
    result["probes"]["delayed_write"] = {
        "run": P.brief(run),
        "receipt_written": ctx.receipt_path(toctou, tree).exists(),
        "obligation_log": (log.read_text(encoding="utf-8", errors="replace").strip()
                           if log.exists() else ""),
    }

    out = Path(argv[2]) if len(argv) > 2 else Path("experiments/verify-p0/results-recheck-probes.json")
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n",
                   encoding="utf-8")
    print(json.dumps(result, indent=2, ensure_ascii=False)[:4000])
    print(f"\nwritten: {out}")
    if not P.os.environ.get("FLOW_P0_KEEP"):
        P.safe_remove_tree(root)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
