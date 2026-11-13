# Flow production candidate — invariants and their proofs (G1)

Every row names the **executed** test that would fail if the invariant were violated. A gate is
`PASSED` only when the test ran on the head named at the end of this document and on both supported
OSes (locally Windows; Ubuntu + Windows in CI). `UNSUPPORTED` is a refusal, never a pass.

Legend for "crash/race": `n/a` means the invariant is not about interruption; otherwise the test
that injects the interruption.

## 1. Verification and receipts

| # | Invariant | Threat | Implementation | Positive test | Negative test | Crash/race | Verdict |
|---|---|---|---|---|---|---|---|
| I1 | `PASSED` means every **required** obligation actually ran on the content of the verified tree | a selective run presented as complete | `cmd_run` scope + `receipt.verification_scope`; `publish` re-checks each required id | `run_passing_obligation_binds_the_index_tree` | `p0a_selective_run_never_yields_a_publishable_receipt`, `p0a_full_run_with_one_failing_required_obligation_is_never_publishable` | n/a | PASSED |
| I2 | The obligations see the **bytes the tree stores**, not a filtered checkout | `core.autocrlf`, `core.eol`, a committed `.gitattributes`, a **local** `.git/info/attributes` / `core.attributesFile`, driver filters | `verify_checkout_content` compares **every** regular file with its blob through one bounded `git cat-file --batch` — unconditionally, never on a "a filter looks possible" guess; above the 64 MiB / 20 000-entry budget the tree is refused | `run_passing_obligation_binds_the_index_tree`, `a_faithful_checkout_is_still_certified` | `a_converted_checkout_is_refused_not_certified`, `a_local_info_attributes_conversion_is_refused_not_certified`, `a_local_smudge_filter_is_refused_not_certified`, `a_core_attributes_file_conversion_is_refused_not_certified`, `a_hook_that_rewrites_a_file_is_refused_not_certified`, `p0a_case_colliding_tree_is_refused_not_certified`, `p0a_symlink_rewritten_as_a_file_is_refused`, `a_tree_path_that_is_not_valid_utf8_is_refused_with_its_own_reason` | n/a | PASSED |
| I3 | A receipt is bound to tree + effective policy + context | policy silently weakened after a green run | `policy_digest`, `context_id`, `integrity`, recomputed in `publish` | `publish_moves_the_ref_with_cas_and_records_the_journal` | `p0b_adding_a_required_obligation_makes_the_receipt_stale`, `p0b_changing_a_timeout…`, `p0b_changing_the_policy_version…`, `p0b_a_timeout_override_is_never_publishable`, `p0b_a_tampered_receipt_is_not_publishable` | n/a | PASSED |
| I4 | Inspection never presents a tampered receipt as a valid proof | `status`/`explain` showing `PASSED` for edited bytes | `status` trust field, `explain.receipt_trust` / `receipt_status_display` | `run_passing_obligation_binds_the_index_tree` | `status_and_explain_qualify_a_tampered_receipt` | n/a | PASSED |
| I5 | Missing obligation logs are missing **evidence**, not a silent pass | deleting logs to hide a failure | `explain.evidence_complete` + `missing_logs` | `explain_reports_missing_logs_as_incomplete_evidence` | idem | n/a | PASSED |
| I6 | A receipt written under another schema is refused, not reinterpreted | schema drift after an upgrade | `Receipt::load` schema check, `SCHEMA_RECEIPT` | `publish_after_run` tests | `p0b_…` schema staleness | n/a | PASSED |

## 2. Publication transaction

| # | Invariant | Threat | Implementation | Positive | Negative | Crash/race | Verdict |
|---|---|---|---|---|---|---|---|
| I7 | A ref moves only by CAS from the expected old OID, and only for a real branch ref | forcing a move, publishing on a stale base | `check-ref-format`, `REF_IS_SYMREF`, `show_ref` + `update-ref old new`, read-back | `m1_two_disjoint_agents_publish_exactly_the_verified_tree` | `m3_stale_base_is_refused_and_a_rebased_proposal_succeeds`, `p1_a_symbolic_ref_is_never_a_publication_target`, `p0b_explicit_ref_must_name_a_branch` | `eight_concurrent_publishes_publish_at_most_once` | PASSED |
| I8 | The published commit carries exactly the verified tree | a commit with another tree published under a valid receipt | `commit-tree` + `rev-parse ^{tree}` before CAS | `m1` asserts `ref^{tree} == candidate tree` | `RECEIPT_COMMIT_MISMATCH` path | n/a | PASSED |
| I9 | An interrupted publication is observed, never replayed | double publication after a crash | fsynced intent → CAS → outcome; `reconcile` observes | `crash_after_ref_is_observed_as_published` | `crash_after_intent_is_not_published_and_reconciles`, `crash_before_intent_leaves_no_pending_intent`, `p0c_one_outcome_cannot_close_two_intents`, `p0c_duplicate_intent_id_requires_a_human` | all three crash points + torn journal | PASSED |
| I10 | A corrupt or unreadable journal is never "reconciled" | torn line read as empty | typed `JournalError`, propagated, `manual_required` | `crash_after_intent_…` | `p0c_corrupt_journal_is_not_reported_as_reconciled`, `p0c_status_refuses_to_guess_on_a_corrupt_journal` | torn/duplicated journal | PASSED |
| I11 | Two Flow writers never both publish the same old OID | race between publishers | `flow-lock/2`, atomic creation, live-owner-safe (an unprovable liveness answer is `LOCK_UNKNOWN`, never a steal — EPERM included) | `two_concurrent_publishes_publish_at_most_once`, `eight_concurrent_publishes_publish_at_most_once` | `a_live_old_lock_is_never_stolen`, `an_unparseable_lock_is_refused_and_preserved`, `a_dead_owners_lock_is_recovered` | 8-process contention | PASSED (does not fence an external `git`) |

## 3. Filesystem and workspace boundary

| # | Invariant | Threat | Implementation | Positive | Negative | Crash/race | Verdict |
|---|---|---|---|---|---|---|---|
| I12 | Flow never writes the user's worktree or index | a "verification" tool mutating the tree | temporary index for proposals, scratch for obligations, ref-only publish | `m6_user_files_and_index_are_preserved_through_the_whole_cycle` | `dirty_and_untracked_files_are_preserved_byte_for_byte` | n/a | PASSED |
| I13 | Flow never writes outside `.flow/` through a link | symlinked/junctioned journal, receipts, logs | `guard_below_flow` on every write path | `m9_a_hostile_cloned_repository_cannot_execute_or_escape` | `a_symlinked_journal_is_never_written_through`; Unix: `a_hardlinked_journal_is_not_written_through` | substitution is detected, documented TOCTOU limit | PASSED (Unix hardlink), Windows reparse PASSED |
| I25 | The checkout holds **nothing the tree does not declare** — no extra file, no extra directory (an empty directory is invisible to Git but not to an obligation that discovers one). Flow's own scratch ownership marker is the single exception; a `.git` link or directory is **not** skipped, and the link is removed before the check | a `post-checkout` hook (run by `git worktree add`, including via a tracked `core.hooksPath`) adds a toolchain file, a test, a build config, a cache directory or re-creates `.git` | `verify_checkout_fidelity` compares the file **and directory** sets exactly and refuses an unexpected path | `a_faithful_checkout_is_still_certified` | `a_hook_that_adds_a_file_is_refused_not_certified`, `a_hook_that_deletes_a_file_is_refused_not_certified`, `a_hook_file_named_like_a_scratch_marker_is_refused_not_certified`, `a_hook_that_creates_an_empty_directory_is_refused_not_certified` | the hook runs before the check; a delayed re-creation of `.git` races the check (reviewer probe, documented) | PASSED |
| I26 | A declared file is **re-proved after the last obligation** | a hook spawns a writer that changes the checkout after the one-shot fidelity check; an obligation rewrites its own input | `verify_checkout_unchanged` re-lists the tree and re-runs the byte and (Unix) mode proof after the obligation loop; a change refuses the run (`CHECKOUT_MUTATED_DURING_RUN`) and no receipt is written; a clean run records `checkout_recheck: OK` | `a_faithful_checkout_is_still_certified` | `a_delayed_write_during_the_obligations_is_refused` | the delayed write in the test happens while the obligation runs; a write reverted before the re-check is not detected | PASSED (detection only) |
| I14 | An identifier is never a path | `explain --tree ../../etc/passwd` | `is_object_id` / `is_record_id` before any path is built | `explain --tree <tree>` tests | `explain_refuses_a_path_instead_of_a_tree_id` | n/a | PASSED |
| I15 | A cloned policy is not executed without explicit approval | a repository shipping a hostile obligation | adoption digest gate; `approve` prints the exact commands | `m9_…` after `approve` | `m9_…` before `approve` (`POLICY_NOT_ADOPTED`/`POLICY_NOT_APPROVED`), `unsafe_obligation_id_is_refused` | n/a | PASSED |
| I16 | Scratch directories are Flow-owned, bounded and removed | deleting a pre-existing directory, leaking a path | `scratch.rs` ownership marker, no delete-before-check, RAII cleanup | `scratch_*` (6 tests) | `scratch_*` refusals | cleanup budget | PASSED |

## 4. Multi-agent workflow

| # | Invariant | Threat | Implementation | Positive | Negative | Crash/race | Verdict |
|---|---|---|---|---|---|---|---|
| I17 | A proposal is immutable and attributed | silent edit of an admitted proposal | content digest + per-actor idempotency slot | `m7_a_replacement_agent_resumes_from_the_immutable_artefacts` | `admission_is_idempotent_and_immutable` | n/a | PASSED |
| I18 | Assembly is deterministic and never merges silently | two agents' hunks silently combined | explicit order, `PATH_OPERATION_CONFLICT`, `CONFLICT` with the proposal named | `m1` | `m2_three_agents_collision_is_diagnosed_and_resolved_explicitly`, `assembly_reports_order_and_stays_deterministic` | n/a | PASSED |
| I19 | A candidate cannot land on a base it was not built on | publishing somebody's work on a rebased branch | candidate base == `--expected-old` (`BASE_MISMATCH`), `m8` for cross-base assembly | `m3` rebase path | `m3_stale_base_…`, `m8_proposals_on_different_bases_are_refused_with_both_bases` | n/a | PASSED |
| I20 | A proposal cannot write Flow's own state or Git metadata | `.git/config`, `.flow/config.json` through a patch | `validate_path` (`FORBIDDEN_PATH`), tree validation | `binary_and_mode_changes_survive_the_cycle` | `proposals_outside_the_trusted_surface_are_refused` | n/a | PASSED |

## 5. Process and resources

| # | Invariant | Threat | Implementation | Positive | Negative | Verdict |
|---|---|---|---|---|---|---|
| I21 | Git and obligations are bounded in memory and wall clock | a repository or obligation that floods stdout | `run_bounded` (64 MiB, 300 s), obligation cap 1 MiB + drain | `obligation_stdout_is_bounded_through_the_cli` | `read_capped_at_retains_at_most_the_cap_and_flags_truncation`, 65 MiB Git stdout refused | PASSED |
| I22 | A hostile environment cannot redirect Git | `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_CONFIG_*` | environment scrub + one explicit absolute index | `hijacked_git_environment_does_not_redirect_the_cli` | idem (control run) | PASSED |
| I23 | A test hook does not exist in a released binary | shipping `FLOW_TEST_CRASH` | `crash_point()` compiled out of release | CI: "the release binary ignores the test crash hook" | local release proof (§G7) | PASSED (CI + local) |
| I24 | Usage errors never mutate | unknown, valueless **or value-carrying** option silently ignored; a boolean the command does not implement silently ignored | `validate_flags`, per-command boolean sets, exit 2 | `unknown_and_valueless_options_are_usage_errors`, `a_boolean_flag_with_an_inline_value_is_a_usage_error_before_any_mutation`, `a_timeout_that_cannot_be_applied_is_a_usage_error` | `a_boolean_flag_a_command_does_not_implement_is_a_usage_error` (`run --dry-run` used to execute and certify) | idem | PASSED |

## 6. What this matrix does **not** prove

- No OS sandbox: an obligation that is malicious runs with the user's rights (`H3`).
- Hooks are not suppressed. `git worktree add` runs `post-checkout` and
  `git update-ref` runs `reference-transaction`; a hook that changes the **disposable
  checkout** is detected and refused (I25, I2), and a declared file changed while the
  obligations run is caught by the post-run re-check (I26). Not detected: a write
  outside the checkout, a write **reverted before** the re-check, and a write after
  it — the receipt binds the tree OID, not a filesystem snapshot. Nothing is
  prevented: there is no sandbox.
- No authentication of receipts: integrity ≠ signature (`PRF-01`, accepted, documented).
- No context completeness: undeclared inputs can change a result.
- No fencing of an external `git` writer; the local lock only serialises Flow.
- macOS is not tested.
- Coverage/mutation/fuzz numbers are reported separately (`FLOW-PRODUCTION-TEST-MATRIX.md`) and are
  not part of the invariants above: a passing invariant is not a coverage claim.
