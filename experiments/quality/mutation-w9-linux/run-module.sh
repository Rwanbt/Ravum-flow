#!/usr/bin/env bash
# Run (or resume) one mutation module of the wave-9 Linux campaign.
#   bash run-module.sh <name> <source file> [test selectors...]
# The PATH is built explicitly: a non-login shell does not read ~/.profile, and a
# PATH that contains parentheses (`Program Files (x86)`) must never be expanded
# unquoted.
set -u
PATH=/home/aes/.cargo/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin
export PATH
unset CARGO_TARGET_DIR
name=$1; shift
file=$1; shift
OUT=/mnt/d/App/Ravum-flow/experiments/quality/mutation-w9-linux
mkdir -p "$OUT"
# The WSL virtual disk lives on C:, which was down to ~2 GB free (wave 8 died when
# it reached 0). Every mutant copy and its build tree therefore go to D:, whose
# volume has room; the outcomes and logs already live there too.
mkdir -p /mnt/d/mutants-w9-tmp
export TMPDIR=/mnt/d/mutants-w9-tmp
export TMP=/mnt/d/mutants-w9-tmp
cd /home/aes/flow-w9 || { echo "CLONE_MISSING"; exit 1; }
echo "=== $name start $(date -Is) head=$(git rev-parse --short HEAD) shard=${SHARD:-none}" | tee -a "$OUT/run.log"
shard_args=()
if [ -n "${SHARD:-}" ]; then shard_args=(--shard "$SHARD"); fi
cargo mutants --file "$file" --output "$OUT/$name" --iterate --jobs "${JOBS:-8}" --timeout 240 \
  "${shard_args[@]}" --gitignore true --colors never --no-times -- "$@" > "$OUT/$name.log" 2>&1
echo "$name exit=$? $(date -Is)" | tee -a "$OUT/run.log"
grep -E "Iteration excludes|Found|mutants tested|caught|missed|timeout" "$OUT/$name.log" | tail -4 | tee -a "$OUT/run.log"
grep -c "^MISSED\|^caught\|^TIMEOUT" "$OUT/$name.log" 2>/dev/null | sed 's/^/outcome lines: /' | tee -a "$OUT/run.log"
