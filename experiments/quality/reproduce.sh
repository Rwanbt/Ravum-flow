#!/usr/bin/env bash
# Reproduce the Flow quality gates in one command (Linux / WSL).
#
#   bash experiments/quality/reproduce.sh                 # fast: fmt, clippy, tests
#   bash experiments/quality/reproduce.sh --coverage      # + product-only coverage gate
#   bash experiments/quality/reproduce.sh --mutation crates/flow/src/lock.rs
#
# Exit code 0 only when every requested step passes. The crate has no
# dependencies, so `--offline` is meaningful. A mutation rate printed by
# `--mutation` is a lower bound for `--bins`, never a crate-wide score.
set -u
REPO=${REPO:-$(cd "$(dirname "$0")/../.." && pwd)}
cd "$REPO" || exit 1
COVERAGE=0
MUTATION=''
while [ $# -gt 0 ]; do
  case "$1" in
    --coverage) COVERAGE=1 ;;
    --mutation) shift; MUTATION=${1:-} ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

failed=0
row() { printf '[%s] %-22s %6ss  %s\n' "$1" "$2" "$3" "$4"; }
check() {
  name=$1; shift
  start=$(date +%s)
  out=$("$@" 2>&1); code=$?
  end=$(date +%s)
  if [ $code -eq 0 ]; then row PASS "$name" $((end - start)) "$*"; else row FAIL "$name" $((end - start)) "$*"; failed=$((failed + 1)); echo "$out" | tail -20; fi
  LAST_OUT=$out
}

echo "repo  : $REPO"
echo "head  : $(git rev-parse --short HEAD)"
echo "rustc : $(rustc --version)"

check fmt cargo fmt --all -- --check
check clippy cargo clippy --locked --offline --all-targets -- -D warnings
check tests cargo test --locked --offline --all-targets
if [ -n "${LAST_OUT:-}" ]; then
  echo "$LAST_OUT" | grep -E '^test result' | tail -4 | sed 's/^/       /'
fi

if [ "$COVERAGE" -eq 1 ]; then
  TC=nightly-2026-05-06
  if ! rustup run "$TC" rustc --version >/dev/null 2>&1; then
    row FAIL coverage 0 "toolchain $TC missing (rustup toolchain install $TC --component llvm-tools-preview)"
    failed=$((failed + 1))
  else
    start=$(date +%s)
    cargo "+$TC" llvm-cov clean --workspace >/dev/null 2>&1
    cargo "+$TC" llvm-cov --locked --offline --all-targets --branch \
      --ignore-filename-regex '[\/]tests[\/]' --no-report >/dev/null 2>&1
    code=$?
    if [ $code -eq 0 ]; then
      cargo "+$TC" llvm-cov report --branch --ignore-filename-regex '[\/]tests[\/]' \
        --lcov --output-path target/flow-coverage-lcov.info >/dev/null 2>&1
      code=$?
    fi
    end=$(date +%s)
    if [ $code -eq 0 ]; then
      gate=$(python3 experiments/quality/analyze_lcov.py target/flow-coverage-lcov.info \
        --product-only --fail-under-lines 95 --fail-under-branches 90 2>&1); code=$?
      total=$(echo "$gate" | grep 'PRODUCT-ONLY TOTAL' | tr -d '\n')
      if [ $code -eq 0 ]; then row PASS coverage $((end - start)) "$total"; else row FAIL coverage $((end - start)) "$total"; failed=$((failed + 1)); echo "$gate"; fi
    else
      row FAIL coverage $((end - start)) 'measurement failed'; failed=$((failed + 1))
    fi
  fi
fi

if [ -n "$MUTATION" ]; then
  start=$(date +%s)
  out=$(cargo mutants --file "$MUTATION" --jobs 2 --timeout 240 --gitignore true \
    --colors never --no-times -- --bins 2>&1); code=$?
  end=$(date +%s)
  summary=$(echo "$out" | grep -E 'mutants tested|caught|missed' | tail -2 | tr '\n' ' ')
  if [ $code -eq 0 ]; then row PASS "mutation $MUTATION" $((end - start)) "$summary"; else row FAIL "mutation $MUTATION" $((end - start)) "$summary"; failed=$((failed + 1)); fi
else
  echo '[SKIP] mutation (pass --mutation <path> to run one module)'
fi

echo
if [ $failed -gt 0 ]; then echo "$failed step(s) failed"; exit 1; fi
echo 'all requested steps passed'
