#!/usr/bin/env bash
# Clean-room smoke matrix for a Linux Flow release binary.
#
# Runs the binary by absolute path from a private install directory, with a
# scrubbed environment (`env -i`: no repository, no .flow/, no host GIT_*), on
# throwaway Git repositories: version/init/run/publish, the negative cases, the
# test-crash hook check, and the "nothing written outside the expected
# directories" audit. Exits 1 if any case does not match its expectation.
#
# Usage: bash scripts/release/smoke-linux.sh <binary> [root] [out.json]
set -euo pipefail

BIN="${1:-}"
ROOT="${2:-/tmp/flow-smoke-$(date +%s)}"
OUT="${3:-}"
[ -n "$BIN" ] || { echo "usage: smoke-linux.sh <binary> [root] [out.json]" >&2; exit 2; }
BIN="$(readlink -f "$BIN")"
[ -x "$BIN" ] || { echo "binary not executable: $BIN" >&2; exit 2; }
command -v python3 >/dev/null || { echo "python3 required for the JSON audit" >&2; exit 2; }

rm -rf "$ROOT"
mkdir -p "$ROOT"/{install,home,tmp,repos}
INSTALL="$ROOT/install"
FLOW="$INSTALL/flow"
cp "$BIN" "$FLOW"
chmod 755 "$FLOW"

# The scrubbed runtime environment: only a PATH that finds git, a private HOME
# and TMPDIR. Nothing else is needed for the binary to work.
MIN_PATH="/usr/local/bin:/usr/bin:/bin"
run_flow() {
    local case="$1" repo="$2" expect="$3"; shift 3
    local out="$ROOT/tmp/$case.out" err="$ROOT/tmp/$case.err"
    local extra=()
    if [ "${FLOW_EXTRA_ENV:-}" = "crash" ]; then extra=(FLOW_TEST_CRASH=after-intent); fi
    local code=0
    ( cd "$repo" && env -i PATH="$MIN_PATH" HOME="$ROOT/home" TMPDIR="$ROOT/tmp" LC_ALL=C \
        "${extra[@]}" "$FLOW" "$@" ) > "$out" 2> "$err" || code=$?
    python3 - "$out" "$case" "$code" "$expect" "$err" <<'PY' >> "$ROOT/results.jsonl"
import json, sys
out, case, code, expect, err = sys.argv[1:6]
try:
    d = json.load(open(out, encoding="utf-8", errors="replace"))
except Exception:
    d = {}
ob = (d.get("obligations") or [{}])[0]
if not isinstance(ob, dict):
    ob = {}
print(json.dumps({
    "case": case,
    "exit": int(code),
    "expect_exit": int(expect),
    "status": d.get("status"),
    "code": d.get("code"),
    "error": d.get("error"),
    "reason": d.get("reason"),
    "obligation_status": ob.get("status"),
    "obligation_stderr": ob.get("stderr_preview"),
    "ref": d.get("ref"),
    "tree": d.get("tree"),
    "pass": int(code) == int(expect),
}))
PY
    return 0
}

new_repo() {
    local dir="$ROOT/repos/$1"
    mkdir -p "$dir"
    (
        cd "$dir"
        git init -q -b main .
        git config user.name 'Flow Smoke'
        git config user.email 'flow-smoke@example.invalid'
        git config commit.gpgsign false
        printf 'x\n' > a.txt
        git add -A
        git commit -qm seed
    )
    printf '%s\n' "$dir"
}

write_config() {
    local repo="$1" obligations="$2"
    mkdir -p "$repo/.flow"
    printf '%s' "{\"schema\":\"flow-config/1\",\"policy_version\":\"FLOW-POLICY-1\",\"execution_boundary\":\"local-trusted\",\"obligations\":[$obligations],\"disabled\":false}" > "$repo/.flow/config.json"
}

: > "$ROOT/results.jsonl"

# ---------------------------------------------------------------- positive cycle
run_flow version "$INSTALL" 0 version
POS="$(new_repo positive)"
run_flow init-default "$POS" 0 init
write_config "$POS" '{"id":"green","program":"git","args":["--version"],"required":true,"timeout_s":30}'
run_flow init-adopt "$POS" 0 init --adopt-policy
printf 'y\n' > "$POS/b.txt"
git -C "$POS" add -A
run_flow run-pos "$POS" 0 run
run_flow publish-pos "$POS" 0 publish

# ---------------------------------------------------------------- negative cases
MISSING="$(new_repo missing-config)"
run_flow missing-config "$MISSING" 1 run

CORRUPT="$(new_repo corrupt-config)"
mkdir -p "$CORRUPT/.flow"; printf '%s' '{not json' > "$CORRUPT/.flow/config.json"
run_flow corrupt-config "$CORRUPT" 1 run

UNAPPROVED="$(new_repo unapproved-policy)"
write_config "$UNAPPROVED" '{"id":"green","program":"git","args":["--version"],"required":true,"timeout_s":30}'
run_flow unapproved-policy "$UNAPPROVED" 1 run

MISSING_PROGRAM="$(new_repo missing-program)"
write_config "$MISSING_PROGRAM" '{"id":"gone","program":"flow-definitely-missing-command-xyz","args":[],"required":true,"timeout_s":10}'
run_flow init-adopt-missing-program "$MISSING_PROGRAM" 0 init --adopt-policy
run_flow missing-program "$MISSING_PROGRAM" 1 run

READONLY="$(new_repo read-only)"
chmod 555 "$READONLY"
READONLY_CODE=0
run_flow read-only-dir "$READONLY" 1 init || READONLY_CODE=$?
chmod 755 "$READONLY"

# ---------------------------------------------------------------- crash hook
HOOK="$(new_repo crash-hook)"
write_config "$HOOK" '{"id":"green","program":"git","args":["--version"],"required":true,"timeout_s":30}'
run_flow init-adopt-crash-hook "$HOOK" 0 init --adopt-policy
printf 'y\n' > "$HOOK/b.txt"
git -C "$HOOK" add -A
run_flow run-crash-hook "$HOOK" 0 run
FLOW_EXTRA_ENV=crash run_flow crash-hook-publish "$HOOK" 0 publish

# ---------------------------------------------------------------- audit
python3 - "$ROOT" "$BIN" "$OUT" <<'PY'
import hashlib, json, os, subprocess, sys
from pathlib import Path

root = Path(sys.argv[1])
binary = Path(sys.argv[2])
out = sys.argv[3]

def sha256(p):
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()

results = [json.loads(line) for line in (root / "results.jsonl").read_text().splitlines() if line.strip()]
by = {r["case"]: r for r in results}

report = {
    "schema": "flow-smoke/1",
    "platform": "linux",
    "binary": str(binary),
    "binary_sha256": sha256(binary),
    "started_root": str(root),
    "minimal_env": {"PATH": "/usr/local/bin:/usr/bin:/bin", "HOME": str(root / "home"), "TMPDIR": str(root / "tmp")},
    "results": results,
    "checks": {},
    "audit": {},
    "failures": 0,
}

checks = {
    "version": by["version"]["status"] == "VERSION" and by["version"]["exit"] == 0,
    "init-default": by["init-default"]["status"] == "INITIALIZED",
    "init-adopt": by["init-adopt"]["status"] == "INITIALIZED",
    "run-pos": by["run-pos"]["status"] == "PASSED" and by["run-pos"]["obligation_status"] == "PASSED",
    "publish-pos": by["publish-pos"]["status"] == "PUBLISHED" and by["publish-pos"]["ref"] == "refs/heads/main",
    "missing-config": by["missing-config"]["exit"] == 1,
    "corrupt-config": by["corrupt-config"]["exit"] == 1,
    "unapproved-policy": by["unapproved-policy"]["status"] == "REFUSED" and by["unapproved-policy"]["code"] == "POLICY_NOT_ADOPTED",
    "missing-program": by["missing-program"]["status"] == "UNSUPPORTED" and by["missing-program"]["obligation_status"] == "UNSUPPORTED",
    "missing-program-never-passed": by["missing-program"]["status"] != "PASSED",
    "read-only-dir": by["read-only-dir"]["exit"] == 1,
    "crash-hook-inert": by["crash-hook-publish"]["status"] == "PUBLISHED" and by["crash-hook-publish"]["exit"] == 0,
    "installed-dir-clean": sorted(p.name for p in (root / "install").iterdir()) == ["flow"],
    "no-scratch-left": not any(p.name.startswith("flow-scratch") for p in (root / "tmp").iterdir()),
}
report["checks"] = checks
report["failures"] = sum(1 for v in checks.values() if not v) + sum(1 for r in results if not r["pass"])

repo_files = [str(p.relative_to(root)) for p in (root / "repos").rglob("*") if p.is_file()]
outside = []
for candidate in [Path.home() / ".flow", Path.home() / ".config" / "flow", Path("/usr/local/bin/flow"), Path("/usr/bin/flow")]:
    outside.append({"path": str(candidate), "exists": candidate.exists()})
report["audit"] = {
    "install_dir": str(root / "install"),
    "install_files": sorted(p.name for p in (root / "install").iterdir()),
    "flow_scratch_left": sorted(p.name for p in (root / "tmp").iterdir() if p.name.startswith("flow-scratch")),
    "repo_file_count": len(repo_files),
    "repo_flow_files": [p for p in repo_files if ".flow/" in p],
    "outside_candidates": outside,
}

text = json.dumps(report, indent=2) + "\n"
if out:
    Path(out).parent.mkdir(parents=True, exist_ok=True)
    Path(out).write_text(text, encoding="utf-8")
    print(f"wrote {out}")

print(f"{'case':22} {'exit':>4} {'expect':>6} {'status':14} {'code':20} pass")
for r in results:
    print(f"{r['case']:22} {r['exit']:>4} {r['expect_exit']:>6} {str(r['status']):14} {str(r['code']):20} {r['pass']}")
print("checks:")
for k, v in checks.items():
    print(f"  {k:30} {v}")
print("install dir:", report["audit"]["install_files"])
print("scratch left:", len(report["audit"]["flow_scratch_left"]))
print("failures:", report["failures"])
sys.exit(1 if report["failures"] else 0)
PY
