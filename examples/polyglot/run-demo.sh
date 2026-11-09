#!/usr/bin/env bash
# End-to-end Ravum Flow demo on a disposable copy of a polyglot fixture.
#
#   1. locates the flow binary ($FLOW_BIN) or builds it once with an external
#      --target-dir, so this repository is never written to;
#   2. copies one fixture to a fresh temporary Git repository;
#   3. adopts the policy (`flow init --adopt-policy`);
#   4. verifies the index tree (`flow run` -> PASSED) and publishes it
#      (`flow publish` -> PUBLISHED) behind an expected-old compare-and-swap;
#   5. applies the fixture's documented red edit, shows `flow run` -> FAILED and
#      `flow publish` -> REFUSED with a non-zero exit and an unchanged ref.
#
# Usage: ./run-demo.sh [rust|node|python|typescript|all]     (default: rust)
#        FLOW_BIN=/path/to/flow ./run-demo.sh
#        KEEP_TEMP=1 ./run-demo.sh rust
#
# Requires: bash, git, and the fixture's own toolchain; no network.
# The typescript fixture additionally requires a TypeScript compiler (tsc) on
# PATH; without it the driver prints UNSUPPORTED and exits non-zero instead of
# skipping the fixture silently.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/../.." && pwd)"
fixture="${1:-rust}"

case "${fixture}" in
  rust | node | python | typescript | all) ;;
  *)
    printf 'usage: %s [rust|node|python|typescript|all]\n' "$0" >&2
    exit 2
    ;;
esac

fail() {
  printf 'ERROR: %s\n' "$*" >&2
  exit 1
}

require() {
  command -v "$1" >/dev/null 2>&1 || fail "fixture needs '$1' on PATH; it is missing, so no pass can be claimed"
}

# The TypeScript fixture is the one fixture whose prerequisite is not the
# language runtime itself. A skipped run must never look like a verified one, so
# a missing compiler is an explicit UNSUPPORTED failure with a non-zero exit.
require_tsc() {
  command -v tsc >/dev/null 2>&1 || fail "UNSUPPORTED: the TypeScript fixture requires a TypeScript compiler on PATH ('tsc', tested with 5.8.2) and Node >= 22.6 for 'node --test' on .ts files; 'tsc' is missing. No pass is claimed and the demo exits non-zero. One-time prerequisite: npm i -g typescript@5.8.2"
}

demo_root="$(mktemp -d "${TMPDIR:-/tmp}/flow-demo-polyglot-XXXXXX")"
flow_bin=""
FLOW_EXIT=0
FLOW_TEXT=""

cleanup() {
  local status=$?
  set +e
  cd "${TMPDIR:-/tmp}" 2>/dev/null || cd /
  if [ -n "${demo_root:-}" ] && [ -d "${demo_root}" ]; then
    if [ "${KEEP_TEMP:-0}" = "1" ]; then
      printf 'kept temporary directory: %s\n' "${demo_root}" >&2
    else
      case "$(basename "${demo_root}")" in
        flow-demo-polyglot-*) rm -rf -- "${demo_root}" ;;
        *) printf 'refusing to remove unexpected temporary path: %s\n' "${demo_root}" >&2 ;;
      esac
    fi
  fi
  exit "${status}"
}
trap cleanup EXIT

# Locate the CLI once. A build target directory inside the temporary demo root
# keeps every artefact out of the repository.
if [ -n "${FLOW_BIN:-}" ]; then
  [ -x "${FLOW_BIN}" ] || fail "FLOW_BIN=${FLOW_BIN} is not executable"
  flow_bin="${FLOW_BIN}"
else
  printf 'building flow (target directory kept outside the repository)...\n' >&2
  cargo build --offline --manifest-path "${repo_root}/Cargo.toml" --target-dir "${demo_root}/target" >&2
  flow_bin="${demo_root}/target/debug/flow"
  if [ ! -x "${flow_bin}" ]; then
    flow_bin="${flow_bin}.exe"
  fi
  [ -x "${flow_bin}" ] || fail "flow binary not found under ${demo_root}/target/debug"
fi

# Run the CLI, capture stdout in FLOW_TEXT and the exit code in FLOW_EXIT.
flow_raw() {
  local out
  if out="$("${flow_bin}" "$@" 2>"${demo_root}/flow.stderr")"; then
    FLOW_EXIT=0
  else
    FLOW_EXIT=$?
  fi
  FLOW_TEXT="${out}"
  if [ -s "${demo_root}/flow.stderr" ]; then
    cat "${demo_root}/flow.stderr" >&2
  fi
}

flow_expect() {
  local expect="$1"
  shift
  flow_raw "$@"
  if [ "${FLOW_EXIT}" -ne "${expect}" ]; then
    printf 'flow %s exited %s, expected %s\n%s\n' "$*" "${FLOW_EXIT}" "${expect}" "${FLOW_TEXT}" >&2
    exit 1
  fi
}

json_has() {
  printf '%s' "${FLOW_TEXT}" | grep -Eq "\"$1\"[[:space:]]*:[[:space:]]*\"$2\""
}

json_string() {
  printf '%s' "${FLOW_TEXT}" |
    sed -n "s/.*\"$1\"[[:space:]]*:[[:space:]]*\"\\([^\"]*\\)\".*/\\1/p" |
    head -n 1
}

run_fixture() {
  local name="$1"
  local source="${script_dir}/${name}"
  local work="${demo_root}/work-${name}"

  local edit_path append_line red_path red_old red_new
  case "${name}" in
    rust)
      edit_path='src/lib.rs'
      append_line=$'\n// demo change: a second, verified tree.\n'
      red_path='src/lib.rs'
      red_old='assert_eq!(add(2, 3), 5);'
      red_new='assert_eq!(add(2, 3), 6);'
      require cargo
      ;;
    node)
      edit_path='src/math.js'
      append_line=$'\n// demo change: a second, verified tree.\n'
      red_path='test/math.test.js'
      red_old='assert.equal(add(2, 3), 5);'
      red_new='assert.equal(add(2, 3), 6);'
      require node
      ;;
    python)
      edit_path='src/math_util.py'
      append_line=$'\n# demo change: a second, verified tree.\n'
      red_path='tests/test_math_util.py'
      red_old='self.assertEqual(add(2, 3), 5)'
      red_new='self.assertEqual(add(2, 3), 6)'
      require python
      ;;
    typescript)
      edit_path='src/math.ts'
      append_line=$'\n// demo change: a second, verified tree.\n'
      red_path='test/math.test.ts'
      red_old='assert.equal(add(2, 3), 5);'
      red_new='assert.equal(add(2, 3), 6);'
      require node
      require_tsc
      ;;
    *) fail "unknown fixture ${name}" ;;
  esac

  [ -f "${source}/.flow/config.json" ] || fail "fixture ${name} has no .flow/config.json"

  mkdir -p "${work}"
  cp -R "${source}"/. "${work}"/
  rm -rf "${work}/target" "${work}/node_modules" "${work}/__pycache__" \
    "${work}/.flow/receipts" "${work}/.flow/logs" "${work}/.flow/lock" \
    "${work}/.flow/journal.jsonl" "${work}/.flow/adoption.json"

  cd "${work}"
  git init -q -b main
  git config user.name 'Flow Polyglot Demo'
  git config user.email 'flow-demo@example.invalid'
  git config commit.gpgsign false
  # Pin the checkout to the bytes in the Git tree, so the demo is identical on
  # Windows and Linux (no smudge/clean conversion).
  git config core.autocrlf false
  git config core.safecrlf false
  git add -A
  git commit -q -m 'polyglot fixture: baseline'
  local baseline
  baseline="$(git rev-parse HEAD)"

  # 1. Policy adoption. A binary that predates the adoption gate refuses `init`
  #    when .flow/ exists and uses the shipped config implicitly; that fallback
  #    is reported, never silently assumed.
  local init_mode
  flow_raw init --adopt-policy
  if [ "${FLOW_EXIT}" -eq 0 ]; then
    init_mode='init --adopt-policy'
  else
    flow_raw help
    if printf '%s' "${FLOW_TEXT}" | grep -q 'adopt-policy'; then
      fail "flow init --adopt-policy failed unexpectedly"
    fi
    [ -f "${work}/.flow/config.json" ] || fail 'no --adopt-policy and no shipped .flow/config.json'
    init_mode='pre-adoption binary: shipped .flow/config.json used as-is (documented fallback)'
  fi

  # 2. A real change under verification: publishing a tree the ref already
  #    carries is refused as ALREADY_PUBLISHED.
  printf '%s' "${append_line}" >> "${work}/${edit_path}"
  git add -A
  local index_tree
  index_tree="$(git write-tree)"

  flow_expect 0 run --json
  [ "$(json_string status)" = 'PASSED' ] || fail "expected PASSED, got $(json_string status)"
  local receipt_tree
  receipt_tree="$(json_string tree_oid)"
  [ "${receipt_tree}" = "${index_tree}" ] || fail "receipt tree ${receipt_tree} is not the index tree ${index_tree}"

  flow_expect 0 publish --expected-old "${baseline}" --json
  [ "$(json_string status)" = 'PUBLISHED' ] || fail "expected PUBLISHED, got $(json_string status)"
  local published published_tree
  published="$(git rev-parse refs/heads/main)"
  [ "${published}" != "${baseline}" ] || fail 'publish did not move the branch'
  [ "${published}" = "$(json_string commit)" ] || fail 'published ref does not name the returned commit'
  published_tree="$(git rev-parse 'refs/heads/main^{tree}')"
  [ "${published_tree}" = "${receipt_tree}" ] || fail 'published commit does not carry the verified tree'

  # 3. The documented red edit: verification fails, publication refuses, and the
  #    ref does not move.
  sed -i.bak "s|${red_old}|${red_new}|" "${work}/${red_path}"
  rm -f "${work}/${red_path}.bak"
  grep -Fq "${red_new}" "${work}/${red_path}" || fail 'the red edit did not apply'
  git add -A
  local red_tree
  red_tree="$(git write-tree)"
  [ "${red_tree}" != "${receipt_tree}" ] || fail 'the red edit did not change the index tree'

  flow_expect 1 run --json
  [ "$(json_string status)" = 'FAILED' ] || fail "expected FAILED on the red edit, got $(json_string status)"

  flow_expect 1 publish --expected-old "${published}" --json
  [ "$(json_string status)" = 'REFUSED' ] || fail "expected REFUSED, got $(json_string status)"
  local refusal_code
  refusal_code="$(json_string code)"
  [ -n "${refusal_code}" ] || fail 'the refusal carries no machine-readable code'
  [ "$(git rev-parse refs/heads/main)" = "${published}" ] || fail 'the ref moved on a refused publication'

  printf '\nFlow polyglot demo — fixture: %s\n' "${name}"
  printf '  flow binary         %s\n' "${flow_bin}"
  printf '  policy adoption     %s\n' "${init_mode}"
  printf '  run (index tree)    PASSED     exit=0  tree=%s\n' "${receipt_tree}"
  printf '  publish             PUBLISHED  exit=0  ref=refs/heads/main  commit=%s\n' "${published}"
  printf '  run (red edit)      FAILED     exit=1  tree=%s\n' "${red_tree}"
  printf '  publish (red edit)  REFUSED    exit=1  code=%s\n' "${refusal_code}"
  printf '  ref after refusal   unchanged\n'
}

case "${fixture}" in
  all)
    run_fixture rust
    run_fixture node
    run_fixture python
    run_fixture typescript
    ;;
  *)
    run_fixture "${fixture}"
    ;;
esac

printf '\nRESULT: PASS — verification, publication and refusal demonstrated on %s.\n' "${fixture}"
