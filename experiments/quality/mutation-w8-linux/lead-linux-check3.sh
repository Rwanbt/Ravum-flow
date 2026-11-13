#!/usr/bin/env bash
# Wave 8, third check: kill the cmd_doctor longpaths survivor on Linux.
set -u
cd /home/aes
rm -rf flow-w8-lead3
git clone -q /mnt/d/App/Ravum-flow flow-w8-lead3 || exit 1
cd flow-w8-lead3 || exit 1
git checkout -q f5a19de || exit 1
echo "HEAD=$(git rev-parse HEAD)"
echo '=== suite ==='
cargo test --locked --offline --all-targets > /tmp/w8c-suite.log 2>&1
echo "suite exit=$?"
echo "passed=$(grep -oE '[0-9]+ passed' /tmp/w8c-suite.log | grep -oE '[0-9]+' | awk '{s+=$1} END {print s+0}') failed=$(grep -oE '[0-9]+ failed' /tmp/w8c-suite.log | grep -oE '[0-9]+' | awk '{s+=$1} END {print s+0}')"
echo '=== probe ==='
sed -i '357s#$# // wave8-probe3#' crates/flow/src/commands.rs
git diff -U0 > /tmp/w8c-probe.diff
grep -E '^\+' /tmp/w8c-probe.diff
echo '=== mutants ==='
cargo mutants --file crates/flow/src/commands.rs --in-diff /tmp/w8c-probe.diff \
  --jobs 1 --timeout 240 --gitignore true \
  -- --bins --test cli --test git_fidelity --test operator_surfaces \
  > /tmp/w8c-mutants.log 2>&1
echo "mutants exit=$?"
grep -E 'mutants tested|Found' /tmp/w8c-mutants.log
echo '=== caught ==='
cat mutants.out/caught.txt 2>/dev/null
echo '=== missed ==='
cat mutants.out/missed.txt 2>/dev/null
echo '=== unviable ==='
cat mutants.out/unviable.txt 2>/dev/null
