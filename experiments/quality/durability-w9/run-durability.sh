#!/usr/bin/env bash
# Wave 9 — observe the post-rename directory flush, instead of only pinning its rule.
#
# Three runs against the same disposable repository:
#   1. control   : no shim            -> the command succeeds;
#   2. logged    : shim, no failure   -> the directory fsync is attempted at least once;
#   3. injected  : shim + EIO         -> the command fails and names the fsync, i.e. the
#                                       error is surfaced, not swallowed.
#
# The product binary is the one built from the pinned source; nothing in the product
# is modified. This is an experiment (`NOT_A_GATE`), not a coverage or mutation run.
set -u
export PATH="$HOME/.cargo/bin:$PATH"
unset CARGO_TARGET_DIR
WORK=${WORK:-/home/aes/flow-w9}
OUT=/mnt/d/App/Ravum-flow/experiments/quality/durability-w9
SHIM=$OUT/fail_dir_fsync.so
mkdir -p "$OUT"
cd "$WORK" || exit 1
echo "HEAD=$(git rev-parse HEAD)" | tee "$OUT/run.log"

cc -shared -fPIC -O2 -o "$SHIM" "$OUT/fail_dir_fsync.c" -ldl 2>> "$OUT/build.log" \
  || { echo "BUILD_FAILED" | tee -a "$OUT/run.log"; exit 1; }
echo "shim built: $SHIM" | tee -a "$OUT/run.log"

cargo build --locked --offline --bin flow > "$OUT/build.log" 2>&1
echo "BUILD_EXIT=$?" | tee -a "$OUT/run.log"
FLOW=$WORK/target/debug/flow
[ -x "$FLOW" ] || { echo "BINARY_MISSING" | tee -a "$OUT/run.log"; exit 1; }

body() {
  local tag=$1 preload=$2 fail=$3
  local repo=$OUT/repo-$tag
  rm -rf "$repo"; mkdir -p "$repo"; ( cd "$repo" && git init -q . )
  local log=$OUT/fsync-$tag.log
  rm -f "$log"
  local out
  out=$(cd "$repo" && FLOW_FSYNC_LOG="$log" FLOW_FSYNC_FAIL="$fail" \
        LD_PRELOAD="$preload" "$FLOW" init --force 2>&1)
  local code=$?
  local fsyncs=0
  [ -f "$log" ] && fsyncs=$(wc -l < "$log")
  echo "run=$tag exit=$code dir_fsync_calls=$fsyncs" | tee -a "$OUT/run.log"
  echo "$out" | tail -3 | sed 's/^/    /' | tee -a "$OUT/run.log"
  RUN_EXIT=$code
  RUN_FSYNCS=$fsyncs
  RUN_OUT=$out
}

body control '' '0'
[ "$RUN_EXIT" -eq 0 ] || echo "UNEXPECTED: the control run failed" | tee -a "$OUT/run.log"

body logged "$SHIM" '0'
if [ "$RUN_FSYNCS" -ge 1 ]; then
  echo "PROVEN: a directory fsync is attempted ($RUN_FSYNCS call(s))" | tee -a "$OUT/run.log"
else
  echo "NOT PROVEN: no directory fsync observed through init" | tee -a "$OUT/run.log"
fi

body injected "$SHIM" '1'
if [ "$RUN_EXIT" -ne 0 ] && echo "$RUN_OUT" | grep -qi 'cannot fsync'; then
  echo "PROVEN: an fsync failure is surfaced to the caller" | tee -a "$OUT/run.log"
else
  echo "NOT PROVEN: the injected fsync failure was not reported (exit=$RUN_EXIT)" | tee -a "$OUT/run.log"
fi

echo "DONE $(date -Is)" | tee -a "$OUT/run.log"
