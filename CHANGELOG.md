# Changelog — Ravum Flow

All notable changes to the `flow` CLI. Dates are ISO-8601 (Europe/Paris).

## [0.2.0-rc.1] — 2026-11-10 — production candidate (unreleased)

### Added
- `flow propose`, `flow proposals list|show`, `flow assemble`: attributed, immutable, idempotent
  proposals applied only to a temporary Git index, and immutable exact-tree candidates built from an
  explicit order with deterministic conflict diagnostics.
- `flow run --candidate` / `flow publish --candidate`: the same receipt, CAS and journal engine bound
  to the candidate tree and to the candidate base (`BASE_MISMATCH` on a stale base).
- `flow publish --dry-run`: full preflight reported as `PLAN`, no commit, no ref move, no journal line.
- `flow approve` and `init --adopt-policy`: a `.flow/config.json` that arrived with a clone is never
  executed until this working copy approves its digest.
- `status` reports the policy path/digest/adoption and qualifies a tampered receipt; `explain` reports
  `receipt_trust`, `receipt_status_display`, `evidence_complete` and `missing_logs`.
- Lock contract `flow-lock/2`: a live owner is never displaced, a dead owner is recovered, an
  unattributable lock is `LOCK_UNKNOWN` with `manual_required`.
- Every Git invocation is bounded (`run_bounded`: 64 MiB per stream, 300 s, descendants killed).
- TypeScript fixture (`tsc --noEmit` + executed tests), alongside Rust, Node and Python.

### Changed
- `doctor` exits 1 when `overall` is `FAILED` (documented contract); `status` carries `flow-status/1`.
- Unknown options and value flags without a value are usage errors (exit 2); a **boolean** flag with an
  inline value (`--dry-run=true`) is a usage error too, instead of being validated and ignored.
- `--timeout` must be a whole number of seconds ≥ 1: an unparseable or zero value is a usage error, so
  a receipt can no longer prove a deadline the run did not use.
- Any document whose `code` is `USAGE` exits 2 (`flow propose` included), matching the published
  contract.
- `flow version`'s `schemas` lists every schema this build emits, including `flow-status/1` and
  `flow-proposal-list/1`, built from the `state::SCHEMA_*` constants.
- `explain --tree` accepts only a 40/64-character hex object id (`INVALID_TREE_ID` otherwise).
- The obligation environment allowlist includes MSVC toolchain variables, so older receipts become
  `STALE_POLICY` and must be re-run.
- The toolchain is pinned: `rust-toolchain.toml` (`1.95.0` + rustfmt/clippy) and
  `rust-version = "1.95"` in the crate manifest; CI names the same version.

### Fixed
- A cloned repository could ship `.flow/journal.jsonl` or `.flow/receipts` as a symlink and make Flow
  write outside the workspace; every write below `.flow/` now refuses links and reparse points
  (Unix hardlinks too).
- A checkout that rewrites bytes (filters, CRLF, `core.eol`) is now compared byte-for-byte with the
  tree blobs and refused (`UNSUPPORTED_CHECKOUT`) instead of producing a false `PASSED`. Since the
  checkout-fidelity wave the comparison is **unconditional**: it no longer depends on a "a filter
  looks possible" guess, so a conversion driven by a **local** attribute source
  (`.git/info/attributes`, `core.attributesFile`) is refused too, and the file set must match the
  tree exactly (a `post-checkout` hook that adds, rewrites or deletes a file is refused).
- `FLOW_TEST_CRASH` is compiled out of release builds (proved on the release binary in CI).
- A symbolic ref is refused as a publication target (`REF_IS_SYMREF`), and the ref is read back after
  the CAS (`NEEDS_RECONCILIATION` if it is not the prepared commit).
- The journal write flushes the containing directory after the rename on Unix: an intent is no longer
  "fsynced" only in the file-sense. Windows has no directory fsync and the README says so.
- `redact()` used `env::vars()`, which panics on a non-Unicode environment value in the middle of
  `run`; it now uses `vars_os()` with a lossy view.
- `redact()` also covers AWS access-key ids (`AKIA`/`ASIA`) and the variable names
  `*ACCESS_KEY*`, `*CREDENTIAL*`, `*AUTH_TOKEN*`, `*APIKEY*`, and the obligation environment boundary
  is now pinned end to end: a variable exported by the operator is absent from the obligation process,
  and a secret-shaped value printed by an obligation never reaches a durable file under `.flow/`.
- Unix lock liveness treated `kill -0`'s EPERM (a live process owned by another user) as "provably
  dead" and could steal a live lock. Linux now asks `/proc/<pid>` first and otherwise accepts only a
  message that proves the process is gone; an unprovable answer is `LOCK_UNKNOWN`, never a steal.
- A failed `git diff --name-only` was turned into an empty `worktree_diff`, i.e. a receipt claiming no
  worktree changes; the run now refuses to certify.
- `save_receipt` built a file name from an unvalidated `tree_oid`; it now requires a 40/64 hex object
  id, like `Receipt::load`.

### Known limitations (unchanged)
No OS sandbox, no hook execution by Flow, no context-completeness proof, receipt integrity is not a
signature, the local lock does not fence an external `git` writer, `publish` moves refs only,
macOS is not tested, performance/productivity is not proven.

## [0.1.0] — 2026-11-09 — verified-tree foundation (unreleased)
- Exact-tree receipts, CAS publication, durable intent/outcome journal, `reconcile`, bounded
  obligations, scratch isolation, policy/context binding, adoption of the MPL-2.0 licence.
