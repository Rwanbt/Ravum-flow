#!/usr/bin/env bash
# Crash points on the *release* binary, through an LD_PRELOAD shim.
#
# The product's own `FLOW_TEST_CRASH` hook is compiled out of release builds (the
# right behaviour, and the reason a shipped binary's crash windows were only ever
# proved in debug builds). This harness kills the real binary at a known syscall
# and asserts the recovery contract: the journal stays readable, `reconcile`
# classifies the outcome, and a later publish still works.
#
# Linux only: it needs LD_PRELOAD and a C compiler.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../.." && pwd)"
work="$(mktemp -d)"
shim="$work/shim.so"
flow="${FLOW_BIN:-$root/target/release/flow}"

cleanup() { rm -rf "$work"; }
trap cleanup EXIT

if ! command -v cc >/dev/null 2>&1; then
  echo "SKIP: no C compiler, the shim cannot be built"
  exit 0
fi
cc -shared -fPIC -O2 -o "$shim" "$here/shim.c" -ldl || { echo "FAIL: cannot build the shim"; exit 1; }
[ -x "$flow" ] || { echo "FAIL: build the release binary first (cargo build --release), looked for $flow"; exit 1; }

json_field() {
  # $1 = file with JSON, $2 = key
  python3 - "$1" "$2" <<'PY'
import json, sys
try:
    with open(sys.argv[1]) as handle:
        value = json.load(handle)
    print(value.get(sys.argv[2], ""))
except Exception:
    print("<unparsable>")
PY
}

failures=0
rows=()

new_fixture() {
  local dir="$1"
  rm -rf "$dir"
  mkdir -p "$dir"
  cd "$dir" || return 1
  git init -q -b main
  git config user.email crash@example.invalid
  git config user.name Crash
  echo base > base.txt
  git add -A && git commit -qm base
  "$flow" init >/dev/null 2>&1 || return 1
  cat > .flow/config.json <<'JSON'
{
  "schema": "flow-config/1",
  "policy_version": "FLOW-POLICY-1",
  "execution_boundary": "local-trusted",
  "obligations": [
    { "id": "noop", "program": "sh", "args": ["-c", "true"], "required": true, "timeout_s": 30 }
  ],
  "disabled": false
}
JSON
  "$flow" approve >/dev/null 2>&1 || return 1
  echo change > change.txt
  git add -A
  "$flow" run >/dev/null 2>&1 || return 1
  git rev-parse HEAD
}

case_one() {
  local name="$1" on="$2" at="$3"
  local dir="$work/$name"
  local base code before after status_out reconcile_out final_out status_field reconcile_field final_field reconcile_code final_code
  base="$(new_fixture "$dir")" || {
    rows+=("| $name | fixture failed | - | - | - | FAIL |")
    failures=$((failures + 1))
    return
  }
  before="$(cd "$dir" && git rev-parse refs/heads/main)"

  ( cd "$dir" && FLOW_CRASH_ON="$on" FLOW_CRASH_AT="$at" LD_PRELOAD="$shim" \
      "$flow" publish --expected-old "$base" >/dev/null 2>&1 )
  code=$?
  after="$(cd "$dir" && git rev-parse refs/heads/main)"

  ( cd "$dir" && "$flow" status >"$work/status.json" 2>&1 ); status_out=$?
  status_field="$(json_field "$work/status.json" status)"
  ( cd "$dir" && "$flow" reconcile >"$work/reconcile.json" 2>&1 ); reconcile_out=$?
  reconcile_field="$(json_field "$work/reconcile.json" status)"
  reconcile_code="$(json_field "$work/reconcile.json" code)"

  # After recovery, the publication must still be possible (or already done).
  ( cd "$dir" && "$flow" publish --expected-old "$base" >"$work/final.json" 2>&1 ); final_out=$?
  final_field="$(json_field "$work/final.json" status)"
  final_code="$(json_field "$work/final.json" code)"

  local moved="no"
  [ "$before" != "$after" ] && moved="yes"
  local verdict="PASS"
  if [ "$code" -ne 137 ]; then verdict="FAIL (exit $code, expected 137)"; fi
  if [ "$status_out" -ne 0 ]; then verdict="FAIL (status exit $status_out)"; fi
  if [ "$reconcile_out" -ne 0 ]; then verdict="FAIL (reconcile exit $reconcile_out)"; fi
  case "$final_field" in
    PUBLISHED | ALREADY_PUBLISHED | REFUSED) ;;
    *) verdict="FAIL (final publish: $final_field)" ;;
  esac
  [ "$verdict" = "PASS" ] || failures=$((failures + 1))

  rows+=("| $name | exit $code | $status_field | ${reconcile_field}${reconcile_code:+($reconcile_code)} | ${final_field}${final_code:+($final_code)} | $moved | $verdict |")
}

echo "# Crash points on the release binary"
echo
echo "Binary: \`$flow\`"
echo
echo "| case | killed | status after | reconcile | publication afterwards | ref moved | verdict |"
echo "|---|---|---|---|---|---|---|"

case_one "rename-1" "rename" 1
case_one "rename-2" "rename" 2
case_one "fsync-1" "fsync" 1

for row in "${rows[@]}"; do echo "$row"; done
echo
echo "Failures: $failures"
[ "$failures" -eq 0 ]
