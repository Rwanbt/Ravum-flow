#!/usr/bin/env bash
# Wave 8, second check: kill the two survivors the Linux state campaign found.
set -u
cd /home/aes
rm -rf flow-w8-lead2
git clone -q /mnt/d/App/Ravum-flow flow-w8-lead2 || exit 1
cd flow-w8-lead2 || exit 1
git checkout -q 3b1e33e || exit 1
echo "HEAD=$(git rev-parse HEAD)"
echo '=== suite ==='
cargo test --locked --offline --all-targets > /tmp/w8b-suite.log 2>&1
echo "suite exit=$?"
grep -E '^test result:' /tmp/w8b-suite.log
echo "passed=$(grep -oE '[0-9]+ passed' /tmp/w8b-suite.log | grep -oE '[0-9]+' | awk '{s+=$1} END {print s+0}')"
echo '=== probe ==='
sed -i '403s#$# // wave8-probe2#' crates/flow/src/state.rs
sed -i '674s#$# // wave8-probe2#' crates/flow/src/state.rs
git diff -U0 > /tmp/w8-probe2.diff
grep -E '^\+' /tmp/w8-probe2.diff
echo '=== mutants ==='
cargo mutants --file crates/flow/src/state.rs --in-diff /tmp/w8-probe2.diff \
  --jobs 1 --timeout 240 --gitignore true \
  -- --bins --test cli --test error_paths --test operator_surfaces --test crash_run \
  > /tmp/w8b-mutants.log 2>&1
echo "mutants exit=$?"
grep -E 'mutants tested|Found' /tmp/w8b-mutants.log
echo '=== caught ==='
cat mutants.out/caught.txt 2>/dev/null
echo '=== missed ==='
cat mutants.out/missed.txt 2>/dev/null
echo '=== unviable ==='
cat mutants.out/unviable.txt 2>/dev/null
