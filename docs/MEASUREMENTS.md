# Measurements — the single source of the numbers

**Rule.** Every other document cites this table instead of restating a count. A number that is not
here is not a claim; a number that is here names its head, its platform and its evidence. Historical
reports keep their own dated numbers on purpose (a record that follows the code is not a record).

**Head of the merged line:** `3b0fb6a` (2026-11-15, branch `feat/flow-production-hardening-fable`), plus the documentation commits on top.
**Toolchain:** `rustc 1.95.0`, pinned by `rust-toolchain.toml`; crate `flow 0.2.0-rc.2`, zero runtime
dependencies.

## 1. Tests executed

| Platform | Executed | Failed | Ignored | Evidence |
|---|---|---|---|---|
| Windows x86_64 (MSVC) | **537** | 0 | 0 | CI run `43705496705`, job `test windows-latest` (`executed_passed=537`) |
| Linux x86_64 (GNU) | **546** | 0 | 0 | CI run `43705496705`, job `test ubuntu-latest` (`executed_passed=546`) |
| macOS arm64 | **546** | 0 | 0 | CI run `43705496705`, job `test macos-latest` (`executed_passed=546`) |
| Windows, local | 537 | 0 | 0 | `cargo test --locked --offline --all-targets` on this host |

Differences are platform gates, not skips: one Windows-only `PATH`/`PATHEXT` gate, and the
non-UTF-8 checkout gate that skips on APFS because the filesystem refuses such a name; the nine
Unix-only tests (host-identity fallbacks, `kill` liveness, a read-only state directory, link
refusal in `gc`, the Git fault-injection shims, the context-probe gate and the aggressive `gc`
during a run) compile and run on Linux and macOS.
**144 named gates** are required by name in CI; each one was verified to resolve to a real
`fn <name>` in `crates/flow`.

Reproduce: `cargo test --locked --offline --all-targets` (the CI floors are 537 / 546 / 546 and
any `ignored` count fails the build).

## 2. Coverage (product-only, branch counters)

| Platform | Gate | Result | Evidence |
|---|---|---|---|
| Linux (CI runner) | lines ≥ 95 %, branches ≥ 90 % | **green, no waiver** | CI job `coverage (product-only gate)`, run `43705496705`: **95.98 % lines (5372/5597) / 90.69 % branches (1003/1106)** |

The measurement is `cargo-llvm-cov --all-targets --branch` with
`--ignore-filename-regex '[\/]tests[\/]'`, then `analyze_lcov.py --product-only`. Test code never
enters the product view. The last quoted figures on an earlier head were **95.91 % lines /
93.68 % branches** (`00f87b7`, see [wave 9](reviews/FLOW-QUALITY-WAVE9.md)) — that head had no
content-addressed store, run-scoped logs, host-tokened lock or strict policy decoder; the gate,
not the decimal, is what CI enforces. The hardening wave closed the gate it had opened: the
2026-11-15 measurements before the test wave were 92.46 % / 86.04 %, and the added tests raise
the same instrumented tree above 95/90 without lowering a threshold or adding a waiver.


## 3. Mutation

| Scope | Result | Evidence |
|---|---|---|
| Module subset, hand-measured (Windows/Linux) | `state` 140/140 · `commands` 96.7 % · `gitcli` 88.6 % · `main` 94.2 % (97.1 % adjudicated) · aggregate **95.4 %** | [wave 9 §3.5](reviews/FLOW-QUALITY-WAVE9.md) |
| **`paths` on Linux, in CI** | **31/31 caught (100 %)** in 2 min | scheduled run [`43491500539`](https://github.com/Rwanbt/Ravum-flow/actions/runs/43491500539), `cargo-mutants 27.1.0` |
| Other modules on Linux, in CI | **campaign armed, runs scheduled**; no rate reported yet | `.github/workflows/flow-mutation.yml` (per-module matrix, report mode) |

Scope is `--bins` (the unit suite): every rate above is a **lower bound**, never a crate-wide
score. Mutation is measured, not yet enforcing a threshold — the threshold has to come from the
first full matrix. The workflow fails when a campaign *runs nothing* (that defect was caught on its
first dispatch: `--bins` was passed to `cargo-mutants` instead of `cargo test`, and the job
reported success while mutating nothing).

## 4. Crash windows on the release binary (Linux)

The product's own `FLOW_TEST_CRASH` hook is inert in release builds (checked in CI). The harness
kills the **shipped** binary from outside, at a counted syscall, and asserts recovery. Raw output:
[`experiments/quality/crash-release/REPORT.md`](../experiments/quality/crash-release/REPORT.md).

| Case | Killed | Journal after | `reconcile` | Republish | Ref moved | Verdict |
|---|---|---|---|---|---|---|
| killed at the 1st `rename` (before the intent is durable) | exit 137 | readable | `RECONCILED` | `PUBLISHED` | no | PASS |
| killed at the 2nd `rename` (after the ref moved) | exit 137 | readable | `RECONCILED` | `REFUSED(STALE_REF)` — the CAS correctly refuses the stale base | yes | PASS |
| killed at the 1st `fsync` (before the rename) | exit 137 | readable | `RECONCILED` | `PUBLISHED` | no | PASS |

Linux only: `LD_PRELOAD` has no Windows equivalent, so **Windows crash windows remain proved on the
debug binary only**. That is a limit, not a claim.

## 5. Retention (`flow gc`)

Opt-in: `--keep N` and/or `--max-age-days D`, `--dry-run` to plan. Never collects the journal, the
config, the adoption record or the proposals; refuses while a publication intent is unresolved or
while a run marker is live. A plain file under `logs/` is collected as a file (calling
`remove_dir_all` on it without a type check is a Windows `ERROR_DIRECTORY` failure, found by the
new tests). Covered by `gc_refuses_without_an_explicit_policy`,
`gc_keeps_the_newest_receipts_and_their_logs`,
`gc_dry_run_reports_the_same_plan_and_removes_nothing`,
`gc_removes_old_evidence_and_keeps_fresh_evidence`,
`gc_never_removes_a_file_it_cannot_name_or_a_link`,
`gc_refuses_while_a_publication_intent_is_unresolved`,
`gc_collects_partial_receipts_plain_log_files_and_unnameable_candidates`,
`gc_keeps_fresh_partial_receipts_candidates_and_logs_and_ages_the_rest`,
`gc_interprets_run_markers_and_clears_only_the_provably_stale_ones`,
`gc_refuses_on_a_lock_it_can_attribute_to_a_live_writer`,
`gc_refuses_to_follow_a_link_in_receipts_or_logs`,
`gc_refuses_while_a_verification_is_actually_running`, the CLI gate
`gc_requires_an_explicit_policy_and_reports_its_plan`, the usage gate
`gc_refuses_retention_values_it_cannot_apply`, and the run-marker gate
`gc_refuses_while_a_live_run_marker_exists`.

## 6. Real-project pilot (2026-11-13)

Windows 11, 16 logical CPUs, release binary built from `447b0c6`. Raw results:
[`experiments/pilot-2026-11-13/results.json`](../experiments/pilot-2026-11-13/results.json).

| Scenario | Input | Result | Time |
|---|---|---|---|
| A — this repository | 2 705 tracked files, real Rust workspace | `run` **PASSED**, `publish` **PUBLISHED** | `run` 23.3 s (2 obligations: `cargo metadata`, `cargo fmt --check`), `publish` 0.55 s |
| A — one real, slow obligation | the same tree, `cargo test --bin flow` + `cargo metadata` | `run` **PASSED** | 18.8 s |
| A — two agents, disjoint files | two proposals from one base, assembled | candidate → `run` **PASSED** → `publish` **PUBLISHED** | propose 0.89 s / 0.32 s, assemble 0.74 s, candidate run 12.5 s |
| A — two agents, same path | two proposals creating the same file | **REFUSED**, `PATH_OPERATION_CONFLICT`, exit 1 | — |
| B — above the old envelope | **25 000 files**, one no-op obligation | `run` **PASSED**, `publish` **PUBLISHED** | `run` 185.7 s (≈ 7.4 ms/file) |

Scenario B is the point: the same operation was a **refusal** before this branch (`20 000`-file
budget). Scenario A is the collaboration claim measured, not asserted: two attributed proposals on
one immutable base produce one verified tree, and a real collision is refused by name.

## 7. Performance (descriptive, not a benchmark)

| Command | Measured | Gate |
|---|---|---|
| `flow version` | < 1 s | CI fails above 2 s (Linux) |
| `flow doctor` | < 5 s | CI fails above 5 s (Linux) |

The CI budgets are regression tripwires, not performance claims. Productivity against other
workflows stays `UNPROVEN` until `H4` fixes the S9 thresholds ([BENCHMARK_S9.md](BENCHMARK_S9.md)).

## 8. Security and supply chain

| Item | State |
|---|---|
| Runtime dependencies | none (`Cargo.lock` = 1 package) |
| `unsafe`, `TODO`/`FIXME`, `#[ignore]` in product code | none (grep-verified) |
| GitHub Actions | pinned to 40-hex commits; `permissions: contents: read` |
| GitHub secret scanning / push protection | enabled |
| Branch protection / required checks | **active** since 2026-11-13: ruleset `protect main and dev`, deletion and force-push blocked, six required checks |
| Release artefact | **`v0.2.0-rc.2`** published from `e1022d86` with `PROVENANCE.json`, `SBOM.json`, `SHA256SUMS.txt` |
| Code signing | **none** (`"unsigned": true`) |

## 9. Maintenance of this table

Update it when a head changes the numbers, and cite the CI run id. Do not copy a number from this
table into prose without its platform and its evidence: that is how four contradictory test counts
appeared three times (R2-06, waves 5 and 8, then again on 2026-11-13).
