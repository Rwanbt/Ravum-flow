#!/usr/bin/env bash
# Wave 9 — re-measure the three `state` survivors that wave 8 closed *after* its
# campaign head, so the module row stops saying "closed by a later commit".
#
# Wave 8 measured `9bbb02d` and reported `state` 134/140 with six misses: three
# `#[cfg(windows)]` (measured by the wave-9 Windows pass) and three that were
# closed by assertions landed afterwards: `377:52` (directory fsync, now
# `parent_to_flush`), `390:41` (journal schema stamp) and `661:22`
# (`iso8601_now` day count). This script finds them by description at the current
# source, drops a one-token probe on each line, and runs them with `--in-diff`.
#
# Lines are discovered, never hard-coded: if the source moves, the probe follows.
set -u
export PATH="$HOME/.cargo/bin:$PATH"
unset CARGO_TARGET_DIR
WORK=/home/aes/flow-w9
OUT=/mnt/d/App/Ravum-flow/experiments/quality/mutation-w9-linux/state-recheck
mkdir -p "$OUT"
cd "$WORK" || exit 1
[ -d .git ] || { echo "CLONE_MISSING" | tee "$OUT/run.log"; exit 1; }

echo "HEAD=$(git rev-parse HEAD)" | tee "$OUT/run.log"
git status --porcelain | tee -a "$OUT/run.log"

cargo mutants --file crates/flow/src/state.rs --list > "$OUT/list.txt" 2> "$OUT/list.err"
echo "LIST_EXIT=$? lines=$(wc -l < "$OUT/list.txt")" | tee -a "$OUT/run.log"

# The three targets, by the mutation they introduce and the function they live in.
grep -E ": (delete !|replace == with !=|replace / with %) in (parent_to_flush|append_journal|iso8601_now)$" \
  "$OUT/list.txt" | tee "$OUT/targets.txt" | tee -a "$OUT/run.log"
LINES=$(cut -d: -f2 "$OUT/targets.txt" | sort -u)
echo "probe lines: $LINES" | tee -a "$OUT/run.log"
[ -n "$LINES" ] || { echo "NO_TARGETS" | tee -a "$OUT/run.log"; exit 1; }

for ln in $LINES; do
  sed -i "${ln}s#\$# // w9recheck#" crates/flow/src/state.rs
done
git diff -U0 -- crates/flow/src/state.rs > "$OUT/probe.diff"
cat "$OUT/probe.diff" | tee -a "$OUT/run.log"

cargo mutants --file crates/flow/src/state.rs --in-diff "$OUT/probe.diff" \
  --jobs 2 --timeout 240 --gitignore true --colors never --no-times \
  --output "$OUT/mutants" -- --bins > "$OUT/mutants.log" 2>&1
echo "MUTANTS_EXIT=$?" | tee -a "$OUT/run.log"

git checkout -- crates/flow/src/state.rs
echo "restored: $(git status --porcelain | wc -l) modified file(s)" | tee -a "$OUT/run.log"
for f in caught missed unviable timeout; do
  echo "--- $f ---" | tee -a "$OUT/run.log"
  cat "$OUT/mutants/mutants.out/$f.txt" 2>/dev/null | tee -a "$OUT/run.log"
done
