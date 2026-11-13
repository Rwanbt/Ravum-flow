#!/usr/bin/env bash
# Wave 8, fourth check (re-run): the first attempt lost its artefacts with /tmp when the
# WSL VM restarted. This run writes the output straight to the Windows evidence tree.
set -u
cd /home/aes/flow-w8-lead4 || exit 1
echo "HEAD=$(git rev-parse HEAD)"
OUT=/mnt/d/App/Ravum-flow/experiments/quality/mutation-w8-linux/lead-gaps2
rm -rf "$OUT/commands-357" "$OUT/gitcli-354"
mkdir -p "$OUT"

echo '=== commands:357 ==='
sed -i '357s#$# // wave8-probe4a#' crates/flow/src/commands.rs
git diff -U0 > /tmp/w8e-a.diff
cargo mutants --file crates/flow/src/commands.rs --in-diff /tmp/w8e-a.diff \
  --jobs 1 --timeout 240 --gitignore true --output "$OUT/commands-357" \
  -- --bins --test cli --test git_fidelity --test operator_surfaces \
  > /tmp/w8e-a.log 2>&1
echo "a exit=$?"
grep -E 'Found|mutants tested' /tmp/w8e-a.log
git checkout -q crates/flow/src/commands.rs

echo '=== gitcli:354 ==='
sed -i '354s#$# // wave8-probe4b#' crates/flow/src/gitcli.rs
git diff -U0 > /tmp/w8e-b.diff
cargo mutants --file crates/flow/src/gitcli.rs --in-diff /tmp/w8e-b.diff \
  --jobs 1 --timeout 240 --gitignore true --output "$OUT/gitcli-354" \
  -- --bins --test isolation --test bounds --test error_paths \
  > /tmp/w8e-b.log 2>&1
echo "b exit=$?"
grep -E 'Found|mutants tested' /tmp/w8e-b.log
git checkout -q crates/flow/src/gitcli.rs

echo '=== outcomes commands ==='
cat "$OUT/commands-357/mutants.out/caught.txt" "$OUT/commands-357/mutants.out/missed.txt" "$OUT/commands-357/mutants.out/unviable.txt" 2>/dev/null
echo '=== outcomes gitcli ==='
cat "$OUT/gitcli-354/mutants.out/caught.txt" "$OUT/gitcli-354/mutants.out/missed.txt" "$OUT/gitcli-354/mutants.out/unviable.txt" 2>/dev/null
git status --porcelain
