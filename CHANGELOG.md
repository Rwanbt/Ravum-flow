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
- A timed-out obligation's **process group** is now actually killed on POSIX. `sh`'s `kill` parsed
  `-<pgid>` as an option (dash answers "Illegal option -9") and the failure was silenced, so only
  the direct child died and a descendant of a killed obligation could outlive the containment. The
  operand is now passed after `--`; the new containment test fails without the fix (wave 6).
- A tree naming a **gitlink** was refused as "the checkout is missing N tree entries": the presence
  check ran before the type check, so the operator read the symptom, not the cause. The gitlink is now
  refused first, with its own reason (submodule content is not materialised), and the dead `160000`
  match arm is removed (wave 7). The behaviour is a refusal in both versions.

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

### Tests and evidence (wave 6, 2026-11-11 — remaining modules and a randomised parser sweep)
- Mutation completed for the four modules wave 5 left `NOT_RUN` (`state`, `gitcli`, `main`,
  `commands`): 599 mutants, 549 applicable, 462 caught = 84.2 %, with per-module scopes and no
  whole-crate claim. The campaign ran from a worktree pinned at the wave-5 head; two runs were
  interrupted by the host and resumed with `--iterate`, and the merge is reported as arithmetic over
  the tool's own counts, never as a fresh classification.
- 45 of the 87 reported survivors are caught by targeted `--in-diff` killchecks on a tree that
  already contains the assertions that close them. The 42 residual sites are classified with proofs:
  15 are `#[cfg(unix)]`/`#[cfg(not(windows))]` code this host does not compile, 11 are
  host-equivalent with the reason, 3 are boundaries the fixtures do not reach, and 13 are real
  operator-surface gaps (listed by name in the report). One campaign survivor (`gitcli:515`) is a
  false negative, proved with a dependency-free reproduction.
- A deterministic randomised parser sweep stands in for `cargo-fuzz`, which the zero-dependency,
  `--locked --offline` contract excludes: 2 000–4 000 adversarial and well-formed shapes per parser
  with explicit invariants, a fixed seed, and a check that both the accepted and the refused answer
  are exercised. It is a sweep, not coverage-guided fuzzing, and is never called that.
- 25 new tests (**459 executed on Windows, 456 on Linux, 0 ignored**, measured in run `42951607307`;
  the CI floor is set to exactly those counts) and 11 new named gates, so a deleted new test fails the
  build like a deleted old one. No test, gate or security property was removed or weakened, and no
  waiver is used.

### Tests and evidence (wave 7, 2026-11-11 — the last assertion gaps and the Unix sites on Linux)
- The 13 operator-surface assertion gaps wave 6 listed are closed by tests, not by waivers: the
  receipt's `stdout_preview`/`stderr_preview` (exactly 400 characters, ellipsis only when cut), the
  pending-intent counter and its `>=` boundary, a duplicate intent fully closed by two outcomes,
  `explain`'s per-tree journal filter, the two checkout type guards (a symlink materialised as a
  symlink; a file replaced by a symlink named by its own reason), the gitlink refusal, the journal's
  unreadable-file arm, and the two Unix-only gaps the Linux measurement found (a declared-`100755`
  script whose bit a hook removed; a hard-linked state file refused by name).
- The three documented boundaries now have fixtures: exactly 20 000 regular files are compared (not
  refused), a batch of exactly 64 MiB is accepted, and a ref of exactly 1 024 characters reaches the
  receipt gate instead of the cheap length refusal.
- The 15 `#[cfg(unix)]`/`#[cfg(not(windows))]` sites wave 6 could only record as "not compiled here"
  were measured on Linux: 12/15 caught on the wave-6 head, and after the two new tests the two real
  gaps are caught too (`commands.rs:1005:41`, `state.rs:340:43`) — 14/15, the two guard mutants of
  `commands.rs:873`/`878` measuring 6/6 caught in a separate Unix-only run. The only survivor,
  `state.rs:377` (directory fsync), is unobservable without a crash-consistency harness and is
  documented as such, never as equivalent. One raw "caught" was a flake in an unrelated `scratch`
  test and is reported as missed after a single-mutant rerun.
- One new test was **red on Linux** at its first committed head and is fixed: `git add -A` in
  `install_tracked_hook` re-staged the `0644` worktree file over the forced `100755` index mode, so
  the tree declared a non-executable file and the test passed vacuously (`308bf6a`). The Windows job
  could not see it because the test is `#[cfg(unix)]`.
- **13 new tests (468 executed on Windows, 469 on Linux, 0 ignored; the CI floor is raised to exactly
  those counts) and 13 new named gates** (4 Unix-only, not claimed on Windows). No test, gate or
  security property was removed or weakened. CI on the evidence head: push run `43013333990` and PR
  run `43013345750` (468 executed on Windows, 469 on Linux, 0 ignored), 5/5 jobs each; product-only
  coverage re-measured at **95.88 % lines / 93.56 % branches** (4399/4588, 799/854), above the 95/90
  gate with no waiver.
- Two `scratch` unit tests no longer predict the process-global name sequence: they retry with a fresh
  value within a bounded number of attempts, so a parallel test that consumes a value between the
  read and `materialise_in` can no longer turn the collision they build into an unrelated Git failure.
  This is a test-stability fix (the Linux mutation campaign had serialised test threads to work
  around it); the guards under test are unchanged.

### Known limitations (unchanged)
No OS sandbox, no hook execution by Flow, no context-completeness proof, receipt integrity is not a
signature, the local lock does not fence an external `git` writer, `publish` moves refs only,
macOS is not tested, performance/productivity is not proven.

## [0.1.0] — 2026-11-09 — verified-tree foundation (unreleased)
- Exact-tree receipts, CAS publication, durable intent/outcome journal, `reconcile`, bounded
  obligations, scratch isolation, policy/context binding, adoption of the MPL-2.0 licence.
