#!/bin/bash
# Linux (WSL) data point for the P0 checkout-fidelity review.
# Two scenarios only, mirroring the Windows A01 (local filter) and A07 (TOCTOU)
# measurements. Everything is disposable under /tmp.
set -u

cd /mnt/d/App/Ravum-flow || exit 1
CARGO_TARGET_DIR=/tmp/flow-target-verify cargo build --locked --offline >/dev/null 2>&1 || {
  echo "BUILD_FAILED"; exit 1; }
FLOW=/tmp/flow-target-verify/debug/flow
echo "flow: $($FLOW version | tr -d '\n' | head -c 80)"

mk_repo() {
  local r="$1"
  git -C "$r" init -q -b main
  git -C "$r" config user.name p
  git -C "$r" config user.email p@x
  git -C "$r" config core.autocrlf false
  printf 'alpha\n' > "$r/a.txt"
  git -C "$r" add -A
  git -C "$r" commit -q -m seed
}

write_policy() {
  local r="$1" obid="$2" prog="$3" code="$4"
  mkdir -p "$r/.flow"
  OBID="$obid" PROG="$prog" CODE="$code" python3 - "$r" <<'PY'
import json, os, pathlib, sys
r = pathlib.Path(sys.argv[1])
cfg = {
    "schema": "flow-config/1",
    "policy_version": "FLOW-POLICY-1",
    "execution_boundary": "local-trusted",
    "obligations": [{
        "id": os.environ["OBID"],
        "program": os.environ["PROG"],
        "args": (["-c", os.environ["CODE"]] if os.environ["CODE"] else ["--version"]),
        "required": True,
        "timeout_s": 60,
    }],
    "disabled": False,
}
(r / ".flow" / "config.json").write_text(json.dumps(cfg), encoding="utf-8")
PY
}

# ---- 1. local .git/info/attributes + same-length smudge filter ------------
R1=$(mktemp -d /tmp/p0lin-filter.XXXXXX)
mk_repo "$R1"
write_policy "$R1" green git ""
(cd "$R1" && "$FLOW" init --adopt-policy >/dev/null)
git -C "$R1" config filter.evil.smudge "/usr/bin/tr '[:lower:]' '[:upper:]'"
git -C "$R1" config filter.evil.clean cat
git -C "$R1" config filter.evil.required true
printf 'a.txt filter=evil\n' > "$R1/.git/info/attributes"
echo "LOCAL-FILTER tree=$(git -C "$R1" write-tree)"
OUT1=$(cd "$R1" && "$FLOW" run 2>&1)
RC1=$?
echo "LOCAL-FILTER exit=$RC1"
echo "$OUT1" | tr -d '\n' | head -c 400
echo

# ---- 2. TOCTOU: background rewrite after the fidelity check --------------
R2=$(mktemp -d /tmp/p0lin-toctou.XXXXXX)
mk_repo "$R2"
mkdir -p "$R2/.hooks"
printf '#!/bin/sh\n( sleep 4; printf "ALPHA\\n" > a.txt ) >/dev/null 2>&1 &\n' \
  > "$R2/.hooks/post-checkout"
chmod +x "$R2/.hooks/post-checkout"
git -C "$R2" add -A
git -C "$R2" commit -q -m seed
git -C "$R2" config core.hooksPath .hooks
TOCTOU_CODE=$'import time,pathlib; time.sleep(7); d=pathlib.Path("a.txt").read_bytes(); print("content_hex="+d.hex()); raise SystemExit(0 if d==b"ALPHA\\n" else 9)'
write_policy "$R2" toctou python3 "$TOCTOU_CODE"
(cd "$R2" && "$FLOW" init --adopt-policy >/dev/null)
OUT2=$(cd "$R2" && "$FLOW" run 2>&1)
RC2=$?
echo "TOCTOU exit=$RC2"
echo "$OUT2" | tr -d '\n' | head -c 300
echo
echo -n "TOCTOU obligation log: "
find "$R2/.flow/logs" -name 'toctou.stdout.log' -exec cat {} \; 2>/dev/null | head -c 120
echo
