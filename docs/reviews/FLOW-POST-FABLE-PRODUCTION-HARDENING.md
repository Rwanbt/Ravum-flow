# Flow — post-Fable production hardening (2026-11-15)

**Status: IN PROGRESS — the verdict at the end is `PRODUCTION_PARTIAL`, not a release.**
This is the single tracking report for the Fable hardening wave. It carries the baseline, the GO
record, the FBL matrix, the gates, the evidence and the final verdict. Every claim names its status:
`CODE_READ`, `MEASURED`, `CONFIRMED_BY_TEST`, `NOT_REPRODUCED`, `NOT_RUN`, `OPEN`, `DEFERRED`.

## 1. Baseline

| Field | Value |
|---|---|
| Repository | `Rwanbt/Ravum-flow` (local `D:\App\Ravum-flow`) |
| Fable audit SHA | `1952325c973c3b002d6436a83656a5593003ad48` (`main` = `origin/main`) |
| `origin/dev` at start | `2ab79c754e0a48746fc50689af28d8124b536563` (same tree as `main`) |
| Working branch | `feat/flow-production-hardening-fable`, created from `origin/dev` |
| Version | `0.2.0-rc.2` |
| Toolchain | `rustc 1.95.0`, pinned by `rust-toolchain.toml`, zero runtime dependencies |
| Baseline working tree | clean except untracked `RAVUM_FLOW_FABLE_AUDIT.md` |
| Baseline local test run | **489 passed, 0 failed** (`cargo test --locked --offline --all-targets`, Windows, 2026-11-15) |

The local `dev` branch was stale (`cd0c636`, ancestor of `origin/dev`); it was fast-forwarded to
`origin/dev` before the feature branch was created. The Fable report is an independent counter-audit:
its G1 reproduced FBL-001 on a clone of the audited SHA; its G3 thread harness did **not** reproduce
FBL-003. Those two facts are kept separate below.

## 2. GO record

The owner granted the scoped GO on 2026-11-15; it is recorded verbatim in
[DECISION_REGISTER § Mandate of 2026-11-15](../DECISION_REGISTER.md). It authorizes hardening on this
repository and this branch only. **No merge, tag, release or `Rwanbt/Ravum` write is authorized.**
`GO_PRODUCTION` remains the owner's act. The GO text names the budgets (16 GB, no concurrent heavy
Cargo campaigns) and the stop conditions (red required check, real secret, Ravum write, destructive
Git, unsourced claim, P0 still open at the verdict).

## 3. Commits on the branch

| Commit | Subject |
|---|---|
| `cd3f023` | docs(governance): record the 2026-11-15 Fable hardening GO and tracking report |
| `e4ce30c` | fix(proposal): transactional admission and evidence revalidation (FBL-001, FBL-009, FBL-010) |
| `9c86ca2` | fix(run,gc,lock,context): per-run evidence, serialized gc, host-aware lock, richer context (FBL-002..005, FBL-008, FBL-011) |
| `d3fcc9c` | fix(policy,publish): strict policy decoder and receipt-candidate binding (FBL-006, FBL-007) |
| `4087e0e` | fix(privacy,evidence): relative persisted paths and journal cost measurement (FBL-011, FBL-012) |
| final HEAD | recorded at the end of this wave (see §9) |

## 4. FBL matrix

`HEAD status` is what this wave established on the branch. A row is `CLOSED` only when a named test
proves the failure mode is gone; `PARTIAL` names what is not covered; `DEFERRED` names why.

| ID | Sev | Fable status | HEAD status | Fix and proof | State |
|---|---|---|---|---|---|
| FBL-001 | P0 | CONFIRMED_BY_TEST (G1 reproduced: 15/15 rounds multi-admit, 1 digest break) | patch bytes are content-addressed (`.flow/blobs/<2>/<sha256>`); the record is published once (`create_once` hard link); `load` re-hashes the patch; `assemble` rebuilds every proposal tree and change list before conflict detection | `tests/concurrency.rs::g1_concurrent_proposals_same_key_have_exactly_one_winner`: 15 rounds × 8 real processes, **exactly 1 ADMITTED + 7 IDEMPOTENCY_CONFLICT per round**, disk digest equals `content_digest`; tampered/missing blobs → `PROPOSAL_INTEGRITY` (`tests/error_paths.rs`) | **CLOSED** |
| FBL-002 | P1 | CONFIRMED_BY_CODE | unique `run_id`; logs under `logs/<tree>/<run_id>/`; the sealed receipt records the run id and the sha256 of every log; an active marker is removed on every exit; `explain` verifies the digests | `tests/concurrency.rs::g2_concurrent_runs_keep_separate_integrity_bound_evidence` (2 concurrent runs, 2 run dirs, receipt digests verified, swapped log → `LOG_INTEGRITY_INVALID`) | **CLOSED** |
| FBL-003 | P1 | CONFIRMED_BY_CODE, NOT_REPRODUCED (thread harness 0/60) | the steal is claimed by a per-generation marker (`lock.steal.<nonce>`); only that marker's creator may remove the exact observed lock; a dead lock is stolen only when the record names this host | `tests/concurrency.rs::g3_dead_lock_recovery_with_eight_publishers_keeps_one_holder`: 8 real `publish` processes over a dead lock → 1 PUBLISHED, 7 named refusals, exactly 1 intent + 1 outcome, no lock artifact; `a_live_old_lock_is_never_stolen`, `a_foreign_host_lock_is_never_stolen`, `a_hostless_dead_lock_is_refused_not_stolen` | **CLOSED (by code + multi-process stress)**. The pre-fix overlap itself was never reproduced; G3 proves exclusion after the fix, not the old race |
| FBL-004 | P1 | CONFIRMED_BY_CODE | `flow-context/3` hashes allowlisted env **values** (digests only), resolved obligation program bytes, and declared `context_probes`; publish recomputes it | `tests/concurrency.rs::g4_environment_change_between_run_and_publish_refuses_context_mismatch` (PATH change → `CONTEXT_MISMATCH`, ref unmoved) | **CLOSED** |
| FBL-005 | P2 | CONFIRMED_BY_CODE | `gc` takes the publish/reconcile lock; it refuses `RUN_ACTIVE` while a live run marker exists and clears only provably dead markers; kept partial receipts keep their tree's logs | `tests/cli.rs::gc_refuses_while_a_live_run_marker_exists`; existing gc suite (dry-run, keep, age, pending intents, links) still green | **CLOSED**, with the documented race: `run` does not hold the lock (it is long-lived); a run that starts after gc's scan creates its logs afterwards and is never deleted |
| FBL-006 | P2 | CONFIRMED_BY_CODE (= AUD-03) | `Config::from_json` is strict: closed key set, exact types, `timeout_s` integer in `[1, 86400]`, `execution_boundary` enum, probes shape; unknown/mistyped input → `INVALID_POLICY` | `state::tests::strict_policy_decoder_refuses_every_silent_coercion` (11 negative cases), `a_config_whose_obligations_are_not_a_list_is_refused` | **CLOSED** (G6) |
| FBL-007 | P2 | CONFIRMED_BY_CODE (= AUD-04) | `publish --candidate` requires `receipt.candidate_id == candidate` (`RECEIPT_CANDIDATE_MISMATCH`); `receipt_candidate_id` is exposed in the plan, the published output and the journal intent | `tests/proposals.rs::m11_publish_binds_the_receipt_to_the_named_candidate` (C1 and C2 build the same tree; C2 with C1's receipt is refused; C1 publishes) | **CLOSED** |
| FBL-008 | P2 | HYPOTHESIS | the lock record now carries a host token; a foreign or host-less record is `LOCK_UNKNOWN`, never stolen; same-host dead locks are recovered | concurrency tests above | **PARTIAL**: host identity is the host name hash, not a boot id; PID reuse after a reboot on the same host is still a wedge, and two machines with the same host name are indistinguishable. Multi-host `.flow` is **not supported** (G8) |
| FBL-009 | P3 | CONFIRMED_BY_CODE | candidates are published with `create_once`; a loser re-reads and confirms the identical assembly or reports `CANDIDATE_IMMUTABLE` | `assemble` reuse test + `assemble_refuses_to_overwrite_an_immutable_candidate` (now refused earlier by `CANDIDATE_INTEGRITY`) | **CLOSED** |
| FBL-010 | P2 | CONFIRMED_BY_CODE | proposal ids are recomputed at load; candidate ids are recomputed from base+tree+proposal list; records carry a seal; `assemble` re-derives each tree and change list; a tampered record is refused | `tests/error_paths.rs` (tampered candidate list/tree → `integrity digest`; tampered blob → `PROPOSAL_INTEGRITY`), `m11` | **CLOSED** |
| FBL-011 | P3 | CONFIRMED_BY_CODE | persisted evidence uses workspace-relative paths: `patch_blob`, `receipt_path`, `journal`, `loc_path` in refusals, status config path; proposal records no longer store an absolute `patch_path` | relative paths asserted in the G1/G2 outputs; full suite | **PARTIAL**: `doctor` diagnostic *details* may still print absolute host paths (not persisted evidence); program resolution paths are hashed, never stored |
| FBL-012 | P3 | CONFIRMED_BY_CODE (documented) | measured: `append_journal` ≈ **9 ms/append**, fsync-dominated, 2 000 appends = 17.97 s / 274 KB on this host; the per-append cost is flat in the measured range, so no quadratic behaviour was observed there | `state::measure_journal_append_cost` (env-gated with `FLOW_JOURNAL_BENCH=1`, counts as a pass otherwise) | **DEFERRED** (P3): segmentation/compaction is a separate, migration-bearing change; it is not required for the local-trusted P0/P1 envelope. No performance claim beyond the measurement |

## 5. Gates

| Gate | Result |
|---|---|
| G1 | **PASS** — `g1_concurrent_proposals_same_key_have_exactly_one_winner` (15×8 real processes, one winner, conflict for losers, digest on disk); `assemble` refuses a corrupted/missing patch with `PROPOSAL_INTEGRITY` |
| G2 | **PASS** — `g2_concurrent_runs_keep_separate_integrity_bound_evidence` (separate run dirs, receipt log digests verified, tamper detected by `explain`) |
| G3 | **PASS (multi-process)** — `g3_dead_lock_recovery_with_eight_publishers_keeps_one_holder`; never two holders observed, one intent/outcome; foreign/host-less locks refused |
| G4 | **PASS** — `g4_environment_change_between_run_and_publish_refuses_context_mismatch`; program bytes and declared probes are part of the context (code-read + hashing) |
| G5 | **PASS (bounded)** — tampered proposal blob, proposal record, candidate record, receipt and log are refused by name (`PROPOSAL_INTEGRITY`, `CANDIDATE_INTEGRITY`, `RECEIPT_INTEGRITY`, `LOG_INTEGRITY_INVALID`) |
| G6 | **PASS** — strict policy decoder matrix (11 negative cases) |
| G7 | **LOCAL PASS, CI PENDING** — `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --locked --offline --all-targets` green locally on Windows; GitHub Actions Linux/Windows/macOS results are recorded in §9 once the branch is pushed |
| G8 | **DOCUMENTED** — local-trusted, single host; foreign/host-less locks are refused; no multi-host sync, no OS sandbox, no signed receipts |

## 6. Local evidence (Windows 11, 2026-11-15)

| Check | Command | Result |
|---|---|---|
| Baseline suite | `cargo test --locked --offline --all-targets` | **489 passed**, 0 failed |
| Hardened suite | same command | **499 passed**, 0 failed (see §9 for the exact head) |
| Formatting | `cargo fmt --check` | exit 0 |
| Lints | `cargo clippy --locked --offline --all-targets -- -D warnings` | exit 0 |
| Release suite | `cargo test --release --locked --offline --all-targets` | **499 passed**, 0 failed, 0 ignored. The six crash-injection tests report a named `SKIP` in release because `FLOW_TEST_CRASH` is inert there by design; CI proves the released binary ignores the hook from outside |
| Journal measurement | `FLOW_JOURNAL_BENCH=1 cargo test --bin flow measure_journal_append_cost -- --nocapture` | `n=500 4.5 s`; `n=1000 9.0 s`; `n=2000 17.97 s`, 274 KB (env-gated, counts as a pass otherwise) |
| Fable G1-equivalent | `g1_...` | 15 rounds × 8 processes: 1 winner/round, digest verified |
| Release stress | `cargo test --release --test concurrency` × 6 consecutive runs | 6/6 green after the two fixes below |

**Two real defects were found by the release-mode stress, not by the audit, and fixed on this branch:**

1. `run` called `git write-tree` on the shared index; two concurrent runs made one fail with
   `index.lock` ("Another git process seems to be running"). `run` now reads a private byte copy of
   the index through `GIT_INDEX_FILE` and never writes the user's index (`index_tree_without_locking`).
2. A burst of eight publishers could turn a transient/stale "dead" answer into a steal attempt and a
   `LOCK_UNKNOWN` refusal at the budget boundary. Windows liveness now requires two consecutive
   definitive "no such task" answers, a steal is re-checked against liveness before deleting anything,
   and a claimed recovery has its own 15 s grace window (`STEAL_GRACE`).

**Test-count caveat.** 489 → 499 is +10: G1, G2, G3, G4, M11, the gc RUN_ACTIVE test, the strict
policy test, the host-identity tests (foreign/host-less), and the measurement test. No test was deleted or
weakened; four existing tests were adapted to the new authoritative storage (blob path, run
subdirectory, host token), and each adaptation keeps or strengthens the asserted failure mode.

## 7. Threat model and limits

- **In scope**: a trusted local user, one Git working copy, Flow processes on one host, Git driven as
  a subprocess. The evidence store, the publication lock and the journal are protected against
  *concurrent Flow processes* and accidental corruption.
- **Out of scope**: an attacker with the same local rights (can rewrite `.flow/` and the config), OS
  sandboxing of obligations, signed receipts, multi-host `.flow/`, distributed leases, arbitrary
  concurrent edits of one working tree, automatic agent orchestration.
- The receipts are **integrity-sealed, not signed**. The lock is a mutual-exclusion device, not a
  security boundary. `run` does not hold the publication lock.
- Fable's unreviewed surfaces were reviewed against the branch: `cmd_reconcile` (re-read, no change
  required), `paths.rs`/`main.rs` dispatch, CI workflows, `MEASUREMENTS.md`, `CAPABILITIES.md`, the
  ADR registry. No new P0 was found.

## 8. Residual risks and open work

| Item | State |
|---|---|
| FBL-012 journal segmentation | **DEFERRED** (P3); measurement recorded, no quadratic behaviour observed at 2 000 records |
| FBL-011 doctor diagnostic paths | **PARTIAL**; diagnostics, not persisted evidence |
| FBL-008 host identity | **PARTIAL**; host name only, no boot id, PID reuse after reboot wedges the lock |
| `commands.rs` modularisation (3.6k LOC) | **DEFERRED**; behaviour-constant refactor, separate change |
| Coverage/mutation gates | not re-run locally; CI owns them (95/90 coverage, mutation report mode) |
| Confirmatory S9 | still `H4` open; no productivity claim is made |
| Multi-OS CI | Linux/Windows/macOS results pending on the pushed head (§9) |

## 9. Final head, CI and verdict

**Final head:** `c7ab4a3` (`fix(concurrency): lock-free index tree reads and conservative steal under burst`)
plus this documentation commit; the exact branch head and the GitHub Actions run ids are recorded
here before the PR is opened. The PR is a **draft** against `dev`; no merge is performed.

**CI:** to be recorded from the push (Linux, Windows, macOS, polyglot, coverage). A red required
check is a stop condition; it is not worked around.

**Verdict: `PRODUCTION_PARTIAL`.**
- No P0 remains (FBL-001 closed by G1).
- All P1 findings are treated (FBL-002/003/004 closed; FBL-005 closed as P2).
- P2/P3 residuals are explicitly listed, not hidden; the local-trusted single-host envelope is
  better protected but **not** promoted to `PRODUCTION_READY_LOCAL_TRUSTED`: that promotion needs the
  green multi-OS CI on this head, the human decisions below, and an independent final review.
- `NOT_READY` would be the verdict if a P0 were still open; it is not.

**Human decisions still required (not taken by this wave):** `H1`, `H3`, `H4`, `H6`, `H8`,
ratification of any `FLOW-ADR-*` (see [FLOW-ADR-012](../adr/FLOW-ADR-012-evidence-store-and-lock-protocol.md)),
and `GO_PRODUCTION` itself.
