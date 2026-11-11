#!/usr/bin/env bash
# Wave 8, fourth check: on Linux, kill the two survivors closed in 09c0b41 —
# commands.rs:357 (cmd_doctor long-path check) and gitcli.rs:354 (allowed_env).
# jobs 1 and one run after the other: the WSL VM has 8 GB and restarts under
# concurrent cargo-mutants workspaces (observed at 14:0x on 2026-11-11).
set -u
cd /home/aes
rm -rf flow-w8-lead4
git clone -q /mnt/d/App/Ravum-flow flow-w8-lead4 || exit 1
cd flow-w8-lead4 || exit 1
git checkout -q 09c0b41 || exit 1
echo "HEAD=$(git rev-parse HEAD)"
echo '=== suite ==='
cargo test --locked --offline --all-targets > /tmp/w8d-suite.log 2>&1
echo "suite exit=$?"
echo "passed=$(grep -oE '[0-9]+ passed' /tmp/w8d-suite.log | grep -oE '[0-9]+' | awk '{s+=$1} END {print s+0}') failed=$(grep -oE '[0-9]+ failed' /tmp/w8d-suite.log | grep -oE '[0-9]+' | awk '{s+=$1} END {print s+0}')"

echo '=== probe commands:357 ==='
sed -i '357s#$# // wave8-probe4a#' crates/flow/src/commands.rs
git diff -U0 > /tmp/w8d-probe-a.diff
cargo mutants --file crates/flow/src/commands.rs --in-diff /tmp/w8d-probe-a.diff \
  --jobs 1 --timeout 240 --gitignore true \
  -- --bins --test cli --test git_fidelity --test operator_surfaces \
  > /tmp/w8d-mutants-a.log 2>&1
echo "mutants-a exit=$?"
grep -E 'mutants tested|Found' /tmp/w8d-mutants-a.log
cp -r mutants.out /tmp/w8d-out-a
git checkout -q crates/flow/src/commands.rs

echo '=== probe gitcli:354 ==='
sed -i '354s#$# // wave8-probe4b#' crates/flow/src/gitcli.rs
git diff -U0 > /tmp/w8d-probe-b.diff
cargo mutants --file crates/flow/src/gitcli.rs --in-diff /tmp/w8d-probe-b.diff \
  --jobs 1 --timeout 240 --gitignore true \
  -- --bins --test isolation --test bounds --test error_paths \
  > /tmp/w8d-mutants-b.log 2>&1
echo "mutants-b exit=$?"
grep -E 'mutants tested|Found' /tmp/w8d-mutants-b.log
cp -r mutants.out /tmp/w8d-out-b
git checkout -q crates/flow/src/gitcli.rs

echo '=== outcomes a ==='
cat /tmp/w8d-out-a/caught.txt /tmp/w8d-out-a/missed.txt /tmp/w8d-out-a/unviable.txt 2>/dev/null
echo '=== outcomes b ==='
cat /tmp/w8d-out-b/caught.txt /tmp/w8d-out-b/missed.txt /tmp/w8d-out-b/unviable.txt 2>/dev/null
