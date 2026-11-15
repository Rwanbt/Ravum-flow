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
| `54ea806` | test(hardening): close the coverage gate on the new store, lock and publish edges |
| `c084a20` | test(hardening): pin the G4 context probe, aggressive gc and legacy-receipt gates |
| `ac38bfb` | test(hardening): expect the platform's honest host identity when no fallback exists |
| `9baa72b` | fix(scratch): retry git's unlocked worktree-admin race instead of failing the run (FBL-013) |
| `3b0fb6a` | style(scratch): restore the file's trailing newline after the clippy fix |
| `922bd22` | fix(policy,publish,gc): close the three defects found by the internal pre-review |
| `fb00d8d` | fix(state): answer `create_once` for an existing name before the hard-link guard (FBL-014) |
| `2b54b99` | feat(journal): segment the journal so one append no longer rewrites the whole history (FBL-012) |
| `4cf6c13` | refactor(commands): split policy identity and checkout certification into submodules |
| final HEAD | recorded at the end of this wave (see §9) |

## 4. FBL matrix

`HEAD status` is what this wave established on the branch. A row is `CLOSED` only when a named test
proves the failure mode is gone; `PARTIAL` names what is not covered; `DEFERRED` names why.

| ID | Sev | Fable status | HEAD status | Fix and proof | State |
|---|---|---|---|---|---|
| FBL-001 | P0 | CONFIRMED_BY_TEST (G1 reproduced: 15/15 rounds multi-admit, 1 digest break) | patch bytes are content-addressed (`.flow/blobs/<2>/<sha256>`); the record is published once (`create_once` hard link); `load` re-hashes the patch; `assemble` rebuilds every proposal tree and change list before conflict detection | `tests/concurrency.rs::g1_concurrent_proposals_same_key_have_exactly_one_winner`: 15 rounds × 8 real processes, **exactly 1 ADMITTED + 7 IDEMPOTENCY_CONFLICT per round**, disk digest equals `content_digest`; tampered/missing blobs → `PROPOSAL_INTEGRITY` (`tests/error_paths.rs`) | **CLOSED** |
| FBL-002 | P1 | CONFIRMED_BY_CODE | unique `run_id`; logs under `logs/<tree>/<run_id>/`; the sealed receipt records the run id and the sha256 of every log; an active marker is removed on every exit; `explain` verifies the digests | `tests/concurrency.rs::g2_concurrent_runs_keep_separate_integrity_bound_evidence` (2 concurrent runs, 2 run dirs, receipt digests verified, swapped log → `LOG_INTEGRITY_INVALID`) | **CLOSED** |
| FBL-003 | P1 | CONFIRMED_BY_CODE, NOT_REPRODUCED (thread harness 0/60) | the steal is claimed by a per-generation marker (`lock.steal.<nonce>`); only that marker's creator may remove the exact observed lock; a dead lock is stolen only when the record names this host; liveness is re-checked immediately before the removal | `tests/concurrency.rs::g3_dead_lock_recovery_with_eight_publishers_keeps_one_holder` (8 real `publish` processes over a dead lock → 1 PUBLISHED, 7 named refusals, exactly 1 intent + 1 outcome, no lock artifact); `a_live_old_lock_is_never_stolen`, `a_foreign_host_lock_is_never_stolen`, `a_hostless_dead_lock_is_refused_not_stolen`; deterministic interleaving: `lock::tests::a_steal_claimed_by_another_writer_is_waited_for_then_reported_unknown` (a second stealer waits the full grace and never deletes the observed lock) and `kill_liveness_answers_a_successful_probe_for_the_caller_group` | **CLOSED (by code + multi-process stress + deterministic claim test)**. The pre-fix overlap itself was never reproduced; G3 proves exclusion after the fix, not the old race |
| FBL-004 | P1 | CONFIRMED_BY_CODE | `flow-context/3` hashes allowlisted env **values** (digests only), resolved obligation program bytes, and declared `context_probes`; the declared probes are also part of the **approved policy digest** (adding one after `flow approve` is `POLICY_NOT_APPROVED` and nothing runs) | `tests/concurrency.rs::g4_environment_change_between_run_and_publish_refuses_context_mismatch` (PATH change → `CONTEXT_MISMATCH`, ref unmoved); `tests/cli.rs::a_changed_context_probe_refuses_publication_without_a_policy_change` (a probe answers differently, the policy file is untouched, publication refused); `an_unapproved_context_probe_is_never_executed` (canary absent) | **PARTIAL**: the context is only as complete as the declared inputs. A policy that declares no probe cannot see a toolchain switch behind an unchanged shim binary (`rustup default` with a same-bytes wrapper), and undeclared environment values are out of scope. The `CONTEXT_MISMATCH` wording says "an allowlisted environment value, an obligation program's bytes, a declared probe's answer, or the platform", not "the toolchain" generally |
| FBL-005 | P2 | CONFIRMED_BY_CODE | `gc` takes the publish/reconcile lock; it refuses `RUN_ACTIVE` while a live run marker exists and clears only provably dead markers; kept partial receipts keep their tree's logs; a plain file under `logs/` is removed as a file | `tests/cli.rs::gc_refuses_while_a_live_run_marker_exists` and `gc_refuses_while_a_verification_is_actually_running` (a real `flow run` is observed writing evidence, `gc` refuses `RUN_ACTIVE`, then succeeds after the marker is gone); the extended unit suite: partial receipts (`gc_collects_partial_receipts_plain_log_files_and_unnameable_candidates`, `gc_keeps_fresh_partial_receipts_candidates_and_logs_and_ages_the_rest`), markers (`gc_interprets_run_markers_and_clears_only_the_provably_stale_ones`), lock (`gc_refuses_on_a_lock_it_can_attribute_to_a_live_writer`) and links (`gc_refuses_to_follow_a_link_in_receipts_or_logs`) | **CLOSED**, with the documented race: `run` does not hold the lock (it is long-lived); a run that starts after gc's scan creates its logs afterwards and is never deleted |
| FBL-006 | P2 | CONFIRMED_BY_CODE (= AUD-03) | `Config::from_json` is strict: closed key set, exact types, `timeout_s` integer in `[1, 86400]`, `execution_boundary` enum, probes shape; unknown/mistyped input → `INVALID_POLICY` | `state::tests::strict_policy_decoder_refuses_every_silent_coercion` (11 negative cases), `a_config_whose_obligations_are_not_a_list_is_refused` | **CLOSED** (G6) |
| FBL-007 | P2 | CONFIRMED_BY_CODE (= AUD-04) | `publish --candidate` requires `receipt.candidate_id == candidate` (`RECEIPT_CANDIDATE_MISMATCH`); `receipt_candidate_id` is exposed in the plan, the published output and the journal intent | `tests/proposals.rs::m11_publish_binds_the_receipt_to_the_named_candidate` (C1 and C2 build the same tree; C2 with C1's receipt is refused; C1 publishes) | **CLOSED** |
| FBL-008 | P2 | HYPOTHESIS | the lock record now carries a host token; a foreign or host-less record is `LOCK_UNKNOWN`, never stolen; same-host dead locks are recovered | concurrency tests above | **PARTIAL**: host identity is the host name hash, not a boot id; PID reuse after a reboot on the same host is still a wedge, and two machines with the same host name are indistinguishable. Multi-host `.flow` is **not supported** (G8) |
| FBL-009 | P3 | CONFIRMED_BY_CODE | candidates are published with `create_once`; a loser re-reads and confirms the identical assembly or reports `CANDIDATE_IMMUTABLE` | `assembling_the_same_proposals_twice_reuses_the_candidate` (idempotent reuse) and `assemble_refuses_a_candidate_record_from_another_schema` / `..._whose_proposal_list_changed` (tampered record refused by `CANDIDATE_INTEGRITY`) | **CLOSED (write path)**. The write race is closed by `create_once`; `CANDIDATE_IMMUTABLE` is a defensive branch that is unreachable through the CLI (same id ⇒ same base/tree/proposal list; a tampered record fails `CANDIDATE_INTEGRITY` first), so it is not counted as a proof |
| FBL-010 | P2 | CONFIRMED_BY_CODE | proposal ids are recomputed at load; candidate ids are recomputed from base+tree+proposal list; records carry a seal; `assemble` re-derives each tree and change list; a tampered record is refused | `tests/error_paths.rs` (tampered candidate list/tree → `integrity digest`; tampered blob → `PROPOSAL_INTEGRITY`), `m11` | **CLOSED** |
| FBL-011 | P3 | CONFIRMED_BY_CODE | persisted evidence uses workspace-relative paths: `patch_blob`, `receipt_path`, `journal`, `loc_path` in refusals, status config path; proposal records no longer store an absolute `patch_path` | relative paths asserted in the G1/G2 outputs; full suite | **PARTIAL**: `doctor` diagnostic *details* may still print absolute host paths (not persisted evidence); program resolution paths are hashed, never stored |
| FBL-012 | P3 | CONFIRMED_BY_CODE (documented) | **segmented**: the active journal is bounded to 256 KiB; past that it is archived by an atomic rename to `journal.<20-digit sequence>.jsonl` and a fresh active file starts. Readers concatenate archived segments (oldest first) then the active one, so an append no longer rewrites the whole history. **Destructive compaction is rejected, not deferred**: dropping closed intents/outcomes would break recovery and audit; archives are evidence and are never deleted | `state::tests::the_journal_rotates_and_still_reads_every_record_in_order`, `a_journal_without_an_active_segment_keeps_its_archived_history`, `a_corrupt_archived_segment_is_refused_and_named`, `journal_segments_only_accept_the_archived_name_shape`; `tests/cli.rs::reconcile_reads_an_intent_closed_in_an_archived_segment` | **CLOSED** (segmentation). The measured ~9 ms/append is now bounded by the segment size instead of growing with age; the env-gated measurement remains available |
| FBL-013 | P2 | **DISCOVERED_BY_CI** (run `43704406911`, `g2` on Ubuntu) | `git worktree add` is not internally locked: it can read another process's half-written `.git/worktrees/<name>/commondir` while that process runs `git worktree prune`, and the loser failed a valid run with `SCRATCH_REFUSED` ("failed to read .../commondir"). `add_worktree` now retries exactly that transient error a bounded number of times and refuses by name when it persists | `tests/cli.rs::a_transient_worktree_admin_race_is_retried_instead_of_failing_the_run` and `an_unresolved_worktree_admin_race_refuses_the_run_by_name`, both driving a real `git` shim that fails the first/ every `worktree add` with Git's own message | **CLOSED (by fault injection)**. The underlying Git race is not removed by Flow; it is tolerated, and a host with a persistently broken worktree admin still fails closed |
| FBL-014 | P2 | **DISCOVERED_BY_CI** (run `43711951264`, `g1` on macOS) | a concurrent creator's staging link makes the final name legitimately have `nlink == 2` for a few microseconds (`name` + `name.tmp-flow-*`); the Unix hard-link guard refused a valid concurrent `propose` with "it has 2 hard links". `create_once` now answers "not created" for any existing name before running the guard, without writing; the caller re-reads and validates the winner's bytes | `state::tests::create_once_treats_a_staging_link_as_an_existing_name_not_an_error` pins the two-link case, and `g1_concurrent_proposals_same_key_have_exactly_one_winner` is the live stress that found it | **CLOSED**. The guard still refuses a hard-linked target for `write_atomic` (journal, config, receipts), where Flow would be writing through a link; only the immutable create-once names short-circuit |

## 5. Gates

| Gate | Result |
|---|---|
| G1 | **PASS** — `g1_concurrent_proposals_same_key_have_exactly_one_winner` (15×8 real processes, one winner, conflict for losers, digest on disk); `assemble` refuses a corrupted/missing patch with `PROPOSAL_INTEGRITY` |
| G2 | **PASS** — `g2_concurrent_runs_keep_separate_integrity_bound_evidence` (separate run dirs, receipt log digests verified, tamper detected by `explain`) |
| G3 | **PASS (multi-process + deterministic claim)** — `g3_dead_lock_recovery_with_eight_publishers_keeps_one_holder`; never two holders observed, one intent/outcome; foreign/host-less locks refused; `a_steal_claimed_by_another_writer_is_waited_for_then_reported_unknown` pins the TOCTOU window deterministically: the second stealer waits `STEAL_GRACE` and never deletes a lock another process claimed |
| G4 | **PASS (environment, program bytes and probes)** — `g4_environment_change_between_run_and_publish_refuses_context_mismatch`; `a_changed_context_probe_refuses_publication_without_a_policy_change` changes only what a declared probe answers and publication is refused with `CONTEXT_MISMATCH` |
| G5 | **PASS (bounded)** — tampered proposal blob, proposal record, candidate record, receipt and log are refused by name (`PROPOSAL_INTEGRITY`, `CANDIDATE_INTEGRITY`, `RECEIPT_INTEGRITY`, `LOG_INTEGRITY_INVALID`, `MISSING_LOGS`); `assembly_revalidates_the_record_it_applies` covers four tampered-record shapes, and `publish_refuses_swapped_or_missing_evidence` proves the publication gate checks the log bytes, not only their names (it did not before the pre-review) |
| G6 | **PASS** — strict policy decoder matrix (11 negative cases, plus omitted-field defaults and probe shapes) |
| G7 | **PASS** — `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --locked --offline --all-targets` green locally on Windows; CI Linux/Windows/macOS and both polyglot jobs green in run `43712727257` (see §9) |
| G8 | **DOCUMENTED** — local-trusted, single host; foreign/host-less locks are refused; no multi-host sync, no OS sandbox, no signed receipts |

## 6. Local evidence (Windows 11, 2026-11-15)

| Check | Command | Result |
|---|---|---|
| Baseline suite | `cargo test --locked --offline --all-targets` | **489 passed**, 0 failed |
| Hardened suite | same command | **539 passed**, 0 failed (see §9 for the exact head) |
| Formatting | `cargo fmt --check` | exit 0 |
| Lints | `cargo clippy --locked --offline --all-targets -- -D warnings` (CI: `--locked --all-targets`) | exit 0 |
| Release suite | `cargo test --release --locked --offline --all-targets` | **507 passed**, 0 failed, 0 ignored at the pre-test-wave head. The six crash-injection tests report a named `SKIP` in release because `FLOW_TEST_CRASH` is inert there by design; CI proves the released binary ignores the hook from outside |
| Journal measurement | `FLOW_JOURNAL_BENCH=1 cargo test --bin flow measure_journal_append_cost -- --nocapture` | `n=500 4.5 s`; `n=1000 9.0 s`; `n=2000 17.97 s`, 274 KB (env-gated, counts as a pass otherwise) |
| Fable G1-equivalent | `g1_...` | 15 rounds × 8 processes: 1 winner/round, digest verified |
| Release stress | `cargo test --release --test concurrency` × 6 consecutive runs | 6/6 green after the two fixes below |
| CI (head `422fad2`) | `gh run view 43712727257` | Linux **550**, Windows **539**, macOS **550** executed, 0 ignored; both polyglot jobs **green**; `coverage` **green** at 95.95 % lines / 90.52 % branches |

**Two real defects were found by the release-mode stress, not by the audit, and fixed on this branch:**

1. `run` called `git write-tree` on the shared index; two concurrent runs made one fail with
   `index.lock` ("Another git process seems to be running"). `run` now reads a private byte copy of
   the index through `GIT_INDEX_FILE` and never writes the user's index (`index_tree_without_locking`).
2. A burst of eight publishers could turn a transient/stale "dead" answer into a steal attempt and a
   `LOCK_UNKNOWN` refusal at the budget boundary. Windows liveness now requires two consecutive
   definitive "no such task" answers, a steal is re-checked against liveness before deleting anything,
   and a claimed recovery has its own 15 s grace window (`STEAL_GRACE`).

**A third defect was found by CI on Ubuntu (FBL-013):** two concurrent runs in one repository can
make `git worktree add` read another process's half-written `.git/worktrees/<name>/commondir` while
that process prunes; the run was refused with `SCRATCH_REFUSED` although nothing was wrong with the
requested tree. `add_worktree` now retries exactly that transient error (≤3 attempts, 150 ms apart)
and refuses by name when it persists; two fault-injection tests drive both outcomes.

**A fourth defect was found by CI on macOS (FBL-014):** the hard-link guard added for FBL-001 saw a
concurrent creator's own staging link (`nlink == 2` for microseconds) and refused a valid concurrent
`propose` with "it has 2 hard links". `create_once` now answers "not created" for any existing name
before the guard, without writing; the regression test pins the two-link case. Both defects are
concurrency bugs that only a real multi-process run could expose, which is why they are recorded as
CI findings rather than audit findings.

**Test-count caveat.** 489 → 539 is +50 executed (Linux 550: the same suite plus the Unix-only
gates). The test wave covers the store, lock, host, context, revalidation, gc, publish and migrate
edges; no test was deleted or weakened, and four existing tests were adapted to the new authoritative
storage (blob path, run subdirectory, host token), each adaptation keeping or strengthening the
asserted failure mode. Raising the CI floors from 482/479/479 to 539/550/550 is the same change: a
floor that does not move with the measured count stops protecting it.

### 6.1 Internal adversarial pre-review (not G10)

Two separate agents that did not write the branch reviewed it anti-adversarially before the external
`G10` slot was filled: one on the transactional contracts, one on the claims and evidence. Their
findings are recorded in [FLOW-PRE-G10-ADVERSARIAL-REVIEW.md](FLOW-PRE-G10-ADVERSARIAL-REVIEW.md).
That review is **not** the independent review and cannot close `G10`.

It found three real defects, all fixed on this branch before promotion:

1. **P1 — an unapproved `context_probes` entry executed.** `effective_policy` did not include the
   probes, so adding one to a cloned `.flow/config.json` changed only the context, not the approved
   policy digest: `flow run` executed it without `flow approve`. The probes are now part of the
   approved digest and listed by `flow approve`; `an_unapproved_context_probe_is_never_executed`
   proves the canary is never written.
2. **P2 — publication did not check the log bytes.** `explain` reported `LOG_INTEGRITY_INVALID` for a
   swapped log while `publish` published the tree anyway. Publication now runs the same check
   (`MISSING_LOGS`, `LOG_INTEGRITY_INVALID`) before the intent; `publish_refuses_swapped_or_missing_evidence`
   drives both outcomes.
3. **P2 — `gc --keep N` deleted fresh candidates and partial receipts** because "no age policy" made
   every entry "expired". Only an explicit age policy now collects them; `--keep` alone never deletes
   unpublished work.

The audit also corrected documentation overclaims: the README denied the `gc` capability, the named
gates were 152 (not 144), the skip caveat was restored in `MEASUREMENTS.md`, FBL-004 is `PARTIAL` and
FBL-009 no longer counts an unreachable defensive branch as proof, and `CAPABILITIES.md` now says that
`.flow/blobs` and `.flow/proposals` are not collected.

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
| FBL-012 journal segmentation | **CLOSED** on this wave: active segment bounded to 256 KiB, archives read in order and never deleted; destructive compaction rejected (it would break recovery/audit) |
| FBL-011 doctor diagnostic paths | **PARTIAL**; diagnostics, not persisted evidence |
| FBL-008 host identity | **PARTIAL**; host name only, no boot id, PID reuse after reboot wedges the lock |
| `commands.rs` modularisation (3.6k LOC) | **PARTIAL**: `commands/mod.rs` + `policy.rs` (policy identity, context, validation) + `checkout.rs` (tree listing, fidelity/mode/content certification, lock-free index read) extracted behaviour-constantly; the command bodies remain in `mod.rs` (~2.6k product lines) and can be split per command later |
| Coverage gate | **CLOSED** on this line: 95.95 % lines / 90.52 % branches on the CI Linux runner, no waiver, thresholds unchanged (were 92.46 / 86.04 before the test wave) |
| Mutation | **RUN 2026-11-15** on `state`/`lock`/`proposal`/`scratch`: killed **85.5 % / 50.7 % / 56.5 % / 61.7 %** (`--bins`, lower bounds); survivors concentrate in refusal/comparison branches. The campaign also found two harness defects (a moved file silently measured nothing; timeouts exit 3 were treated as a broken campaign), both fixed. `commands*` re-dispatched with the fixed mapping. CI stays report mode: no threshold yet |
| Confirmatory S9 | **RUN 2026-11-15, NEGATIVE**: G median 4.925 s vs E 2.823 s, paired ratio 1.784, 10/10 pairs, p=0.000977 — the H4 criterion (≤0.70) failed. No productivity claim; see [the campaign summary](../../experiments/s9/CAMPAIGN-2026-11-15.md) |
| **O3 workstream (`H1 = O3 complet`)** | **NOT IMPLEMENTED**: the delivered code is the O1 + O2 subset. O3 needs its own ADR, budget, threat model and P0 review; four shapes and their tradeoffs are compared in [FLOW-O3-PROPOSALS.md](FLOW-O3-PROPOSALS.md). No document may present O3 as delivered |
| Multi-OS CI | **green on the head**: Linux, Windows, macOS and both polyglot jobs pass in run `43712727257`; coverage also green (see §9) |

## 9. Final head, CI and verdict

**Final code head:** `422fad2` (`fix(state): answer create_once for an existing name before the
hard-link guard`), plus this documentation commit. The PR is
**[#71](https://github.com/Rwanbt/Ravum-flow/pull/71)**, a **draft** against `dev`; no merge is
performed. Tracking issue: [#72](https://github.com/Rwanbt/Ravum-flow/issues/72).

**CI on `422fad2` — run [`43712727257`](https://github.com/Rwanbt/Ravum-flow/actions/runs/43712727257):**

| Job | Result |
|---|---|
| `test ubuntu-latest` (incl. the external crash harness on the release binary) | **success** — 550 executed, 0 ignored |
| `test windows-latest` | **success** — 539 executed, 0 ignored |
| `test macos-latest` | **success** — 550 executed, 0 ignored |
| `polyglot ubuntu-latest` / `polyglot windows-latest` | **success** / **success** |
| `coverage (product-only gate)` | **success**: 95.95 % lines (5421/5650), 90.52 % branches (1012/1118), thresholds 95/90 unchanged, no waiver |

The branch is now mergeable **from a CI standpoint**: every required check is green on the head. It is
not merged, because the remaining conditions are human and review acts, not code:

**Verdict: `PRODUCTION_PARTIAL`.**
- No P0 remains (FBL-001 closed by G1).
- All P1 findings are treated (FBL-002/003 closed; FBL-004 is `PARTIAL` with its scope named;
  FBL-005 closed as P2), and every OS test job, the polyglot jobs and the coverage gate are green on
  the head.
- P2/P3 residuals are listed in §8. The branch is **not** `PRODUCTION_READY_LOCAL_TRUSTED`:
  promotion needs the independent final review (`G10`) and `GO_PRODUCTION`, and
  **`H1 = O3 complet` changes the target**: the delivered code is the O1 + O2 subset, so O3 needs its
  own design, budget and P0 review before this line can be presented as the target architecture.
- `NOT_READY` would be the verdict if a P0 were still open; it is not.

**Human decisions of 2026-11-15 (recorded verbatim in the [register](../DECISION_REGISTER.md)):**
`H1 = O3 complet` (target; **not implemented**), `H3` = refuse/fail-closed without OS isolation,
`H4` = S9 thresholds and budget granted (campaign not yet run), `H6` = draft handoff authorized with
no Ravum write, `H8` = review stop rule adopted, ADR-012 stays `PROPOSED` until after `G10`.
`G10` and `GO_PRODUCTION` remain open.

