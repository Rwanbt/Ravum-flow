#!/usr/bin/env bash
# Wave 9 — Linux per-module mutation rates for the five modules that were only
# ever measured on Windows (waves 4 and 5).
#
# The scopes are the wave-4/wave-5 Windows scopes, copied verbatim:
#   json/paths/lock            : --bins
#   proposal                   : --bins --test proposals --test error_paths --test operator_surfaces
#   scratch                    : --bins --test isolation --test error_paths --test git_fidelity --test crash_run
# so the two platforms are comparable. The scope is part of the result: each rate
# is a lower bound for its module and is never a whole-crate score.
#
# A disposable ext4 clone is pinned at the committed head; every artefact is
# written straight to /mnt/d so a WSL restart cannot lose it (wave 8 lost /tmp
# twice). One module at a time, two jobs: the host has 16 GB.
set -u
# A non-login shell does not read ~/.profile, so the toolchain was not on PATH and
# every module exited 127. Make the location explicit instead of relying on login.
export PATH="$HOME/.cargo/bin:$PATH"
unset CARGO_TARGET_DIR
SRC=/mnt/d/App/Ravum-flow
WORK=/home/aes/flow-w9
OUT=/mnt/d/App/Ravum-flow/experiments/quality/mutation-w9-linux
PIN=3d0bc4d
mkdir -p "$OUT"

if [ ! -d "$WORK/.git" ]; then
  git clone -q "$SRC" "$WORK" || { echo "CLONE_FAILED" | tee "$OUT/run.log"; exit 1; }
fi
cd "$WORK" || exit 1
git fetch -q origin 2>/dev/null
git checkout -q "$PIN" || { echo "CHECKOUT_FAILED" | tee -a "$OUT/run.log"; exit 1; }
echo "HEAD=$(git rev-parse HEAD) cargo-mutants=$(cargo mutants --version 2>/dev/null)" | tee "$OUT/run.log"
echo "uname=$(uname -r) nproc=$(nproc)" | tee -a "$OUT/run.log"

cargo test --locked --offline --all-targets > "$OUT/baseline.log" 2>&1
echo "BASELINE_EXIT=$?" | tee -a "$OUT/run.log"
grep -E "^test result" "$OUT/baseline.log" | tail -4 | tee -a "$OUT/run.log"

run() {
  name=$1; file=$2; shift 2
  echo "=== $name start $(date -Is)" | tee -a "$OUT/run.log"
  cargo mutants --file "$file" --output "$OUT/$name" --jobs 2 --timeout 240 \
    --gitignore true --colors never --no-times -- "$@" > "$OUT/$name.log" 2>&1
  echo "$name exit=$? $(date -Is)" | tee -a "$OUT/run.log"
  grep -E "Found|mutants tested|caught|missed" "$OUT/$name.log" | tail -3 | tee -a "$OUT/run.log"
}

run json     crates/flow/src/json.rs     --bins
run paths    crates/flow/src/paths.rs    --bins
run lock     crates/flow/src/lock.rs     --bins
run proposal crates/flow/src/proposal.rs --bins --test proposals --test error_paths --test operator_surfaces
run scratch  crates/flow/src/scratch.rs  --bins --test isolation --test error_paths --test git_fidelity --test crash_run

echo "ALL_DONE $(date -Is)" | tee -a "$OUT/run.log"
