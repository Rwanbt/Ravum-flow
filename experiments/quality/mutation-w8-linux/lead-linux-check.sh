#!/usr/bin/env bash
# Wave 8: verify the new Unix-only gate on Linux and kill-check line 396.
set -u
cd /home/aes
rm -rf flow-w8-lead
git clone -q /mnt/d/App/Ravum-flow flow-w8-lead || exit 1
cd flow-w8-lead || exit 1
git checkout -q 90640b8 || exit 1
echo "HEAD=$(git rev-parse HEAD)"
echo '=== suite ==='
cargo test --locked --offline --all-targets > /tmp/w8-suite.log 2>&1
echo "suite exit=$?"
grep -E '^test result:' /tmp/w8-suite.log
echo "passed=$(grep -oE '[0-9]+ passed' /tmp/w8-suite.log | grep -oE '[0-9]+' | awk '{s+=$1} END {print s+0}')"
grep -E 'only_a_real_parent_directory_is_flushed' /tmp/w8-suite.log || echo 'GATE-TEST-NOT-FOUND'
echo '=== probe ==='
sed -i '396s#$# // wave8-probe#' crates/flow/src/state.rs
git diff -U0 > /tmp/w8-probe.diff
cat /tmp/w8-probe.diff
echo '=== mutants ==='
cargo mutants --file crates/flow/src/state.rs --in-diff /tmp/w8-probe.diff \
  --jobs 1 --timeout 240 --gitignore true \
  -- --bins --test cli --test error_paths --test operator_surfaces --test crash_run \
  > /tmp/w8-mutants.log 2>&1
echo "mutants exit=$?"
tail -25 /tmp/w8-mutants.log
echo '=== outcomes ==='
ls mutants.out 2>/dev/null | head
cat mutants.out/missed.txt 2>/dev/null
cat mutants.out/caught.txt 2>/dev/null
