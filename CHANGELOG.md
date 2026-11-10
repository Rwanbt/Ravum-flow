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
- A checkout that rewrites bytes (filters, CRLF, `core.eol`) is now compared byte-for-byte with
  the tree blobs and refused (`UNSUPPORTED_CHECKOUT`) instead of producing a false `PASSED`. Since the
  checkout-fidelity wave the comparison is **unconditional**: it no longer depends on a "a filter
  looks possible" guess, so a conversion driven by a **local** attribute source
  (`.git/info/attributes`, `core.attributesFile`, a `smudge` driver) is refused too, and the file set
  must match the tree exactly — no extra file, no extra directory, no added executable bit on Unix (a
  `post-checkout` hook that adds, rewrites, chmods or deletes is refused). A path the platform cannot
  represent (non-UTF-8) is refused with that reason instead of being misdiagnosed as a case collision.
- A declared file changed **while the obligations ran** (a delayed `post-checkout` writer, or an
  obligation rewriting its own input) is detected by a re-proof after the last obligation: the run is
  refused (`CHECKOUT_MUTATED_DURING_RUN`), no receipt is written, and a clean receipt carries
  `checkout_recheck: OK`. Detection, not isolation: a write reverted before the re-check is not seen.
- A boolean flag that a command does not implement is a usage error (exit 2). `flow run --dry-run`
  used to execute the obligations and write a receipt for an operator who asked for a plan.
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

### Tests and evidence (wave 4, 2026-11-10 — tests only)
- New `tests/operator_surfaces.rs` (30 tests) pins the operator-facing contracts: `init` shows a
  committed policy and adopts nothing without `--adopt-policy`, `doctor` discloses a policy changed
  since approval, a corrupt adoption record, hooks, every attribute source, case folding, a live or
  unparseable lock and a disabled policy, `publish` refuses an unusable ref name, a detached HEAD, an
  unknown or pruned candidate and an unattributable lock, `reconcile` reports incomplete intents,
  shared ids and only decisive observations, and `status`/`proposals`/`assemble` stay safe on corrupt
  or missing local state.
- Unit tests inside six modules cover private refusal paths the CLI cannot reach deterministically
  (JSON shapes and pretty nesting, config/adoption/journal/receipt validation, lock ownership, path
  validation, scratch ownership, secret redaction).
- Candidate coverage gates **met**: 93.14 % lines / 85.37 % branches / 89.78 % functions
  (93.04 / 86.17 % product-only), up from 88.68 / 75.67. The CI floor rises to 349 executed tests
  and 11 new blocking gates; no threshold was changed and no test was removed or ignored.
- Mutation widened from 13/119 mutants in one module to 221 mutants in three modules (`json`,
  `paths`, `lock`) with per-module denominators; the rest of the crate stays `NOT_RUN` and no
  whole-crate rate is claimed (`docs/reviews/FLOW-QUALITY-WAVE4.md`).

### Tests and evidence (wave 5, 2026-11-10 — refactors and tests)
- The tree/content parsers of `commands.rs` and `proposal.rs`, the symlink-target rule, the
  cross-proposal conflict diagnosis and PATHEXT resolution are pure functions now, so a hostile Git
  answer can be fed byte for byte instead of requiring a fake binary. No behaviour changed; the same
  callers call the same code.
- Two never-panic fixes found while testing those parsers: a `cat-file --batch` header whose size
  overflows `usize` (`checked_add`), and a batch that ends exactly between two entries (the slice
  used to advance past the end and panic instead of refusing). A panic is not a refusal: it skips
  `Drop` cleanup.
- Product-only coverage (unit-test code excluded) **raised from 93.04 % lines / 86.17 % branches to
  95.74 % / 92.81 %** on Windows and **95.66 % / 92.72 %** on the Linux CI job, with 54 new test functions (422 executed on Windows, 419 on Linux, 0 ignored). The candidate thresholds are now
  **95 % lines / 90 % branches**, enforced by a new `coverage` CI job; no test, gate or security
  property was removed or weakened, and no waiver is used.
- The executed-test floor and the named gates were raised with the tests (17 new gates), so a
  deleted new test fails the build like a deleted old one.
- Mutation was extended beyond the three wave-4 modules (see `docs/reviews/FLOW-QUALITY-WAVE5.md`
  §5 for the per-module, scoped numbers); each rate is a lower bound for its module, never a
  whole-crate score.

### Known limitations (unchanged)
No OS sandbox, no hook execution by Flow, no context-completeness proof, receipt integrity is not a
signature, the local lock does not fence an external `git` writer, `publish` moves refs only,
macOS is not tested, performance/productivity is not proven.

## [0.1.0] — 2026-11-09 — verified-tree foundation (unreleased)
- Exact-tree receipts, CAS publication, durable intent/outcome journal, `reconcile`, bounded
  obligations, scratch isolation, policy/context binding, adoption of the MPL-2.0 licence.
