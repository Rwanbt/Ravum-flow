# FLOW — production-candidate test matrix (independent quality evidence)

Evidence class legend, per `AGENTS.md`: **MEASURED** (a command ran and its raw
output is stored here), **NOT_MEASURED** (attempted, could not be produced — the
error is given), **NOT_RUN** (not attempted or out of the time box — the reason is
given), **CODE_READ** (read, not executed).

Branch: `work/prod-quality` · base `ea181d4` (`feat/flow-production-candidate`) ·
candidate under test `f237d57` (PR #54) is **not** checked out here: this evidence
is bound to `ea181d4` + the branch head recorded below.

| Item | Value |
| --- | --- |
| Base commit | `ea181d4` |
| Evidence head | the commit that adds this file on `work/prod-quality` |
| Host | Windows 11 Professionnel 10.0.26100, 16 logical cores, 15.7 GiB RAM |
| Rust (default) | rustc 1.95.0 (59807616e 2026-04-14), stable-x86_64-pc-windows-msvc |
| Rust (coverage) | rustc 1.97.0-nightly (365c0e1d7 2026-05-06) + `llvm-tools-preview` |
| cargo-llvm-cov | 0.9.1 (`cargo install cargo-llvm-cov --locked`) |
| cargo-mutants | 27.1.0 (`cargo install cargo-mutants --locked`) |
| git | 2.51.1.windows.1 |
| Python (CLI fuzz driver) | 3.13.7 |
| `crates/flow` dependencies | **none** (runtime and dev) — unchanged by this work |

This work adds **no file to `crates/flow`** and changes no source file. Everything
below is additive evidence under `experiments/quality/` plus this document.

---

## 1. Gate table

| # | Gate | Verdict | Evidence |
| --- | --- | --- | --- |
| G1 | Full suite executes and passes (CI parity command) | **PASSED** | **163 executed on Windows, 162 on Linux**, 0 failed, 0 ignored — `experiments/quality/test-run-full.log` |
| G2 | Concurrency suite is not flaky (3 consecutive runs) | **PASSED** | 8/8 passed 3× in 6.94 s / 7.97 s / 7.02 s — `concurrency-run-1..3.log` |
| G3 | Line coverage | **MEASURED** — 80.76 % (4 564 lines) | `coverage/summary.txt`, `coverage/lcov.info`, `coverage/coverage.json` |
| G4 | Branch coverage | **MEASURED** — 65.26 % (780 branches) | same run (`--branch`, nightly) |
| G5 | Per-module line/branch coverage | **MEASURED** — table in §3 | `coverage/summary.txt` |
| G6 | Mutation testing, whole crate (960 mutants) | **NOT_RUN** — time box; 960 × ~90 s test suite ≈ 24 h serial | count from `mutation/all-mutants-list.txt` |
| G7 | Mutation testing, bounded subset (`paths.rs`, `lock.rs`) | **PARTIAL / MEASURED** — 82 tested: 57 caught, 19 missed, 0 timeout, 6 unviable ⇒ **75.0 %** killed | `mutation-997f983/{caught,missed,unviable}.txt`, `outcomes.json` |
| G8 | Seeded property harness (no new dependency) | **PASSED** — 7/7 properties, seed 20261110 | `properties/run.log`, `properties/props.rs` |
| G9 | Seeded JSON-parser fuzz, 60 s | **FAILED (finding)** — round-trip contract violated by non-finite numbers | `fuzz/run-json.log`, `fuzz/stats-json.json`, `fuzz/crashes/` |
| G10 | Seeded CLI option-parser fuzz, 60 s | **PASSED** — 0 findings in 273 invocations | `fuzz/run-cli.log`, `fuzz/stats-cli.json` |
| G11 | `cargo-fuzz` / libFuzzer coverage-guided fuzzing | **NOT_RUN** — needs a `cargo-fuzz` install + nightly `-Z sanitizer=address` target; the hand-rolled harnesses were used instead (see §5) | — |
| G12 | GitHub Actions `flow-ci.yml` status for this head | **NOT_RUN** — branch not pushed from this worktree; CI result for `f237d57` must come from the PR | — |
| G13 | Miri / sanitizers / leak checks | **NOT_RUN** — no unsafe code in the crate (`#![forbid]`-free safe Rust, `CODE_READ`), and the time box was spent on coverage + mutation | — |

---

## 2. Suite inventory (executed names → invariant)

**163 tests executed on the delivered head `997f983`** (162 on Linux); the complete name list per binary is in
`experiments/quality/test-inventory.txt`. Unit tests are compiled into every test
binary (they are `#[cfg(test)]` modules re-reached through `#[path]`), so the same
name appears in several binaries; the count below is what CI counts.

### `crates/flow/src/**` — 24 unit tests (binary `src/main.rs`)

| Test | Invariant |
| --- | --- |
| `gitcli::tests::env_allowlist_is_sorted_and_unique` | the Git env allowlist is deterministic (no duplicate/unsorted entries) |
| `gitcli::tests::scrub_list_covers_every_documented_redirect` | every env var that can redirect Git is scrubbed |
| `gitcli::tests::redacts_secret_env_value` / `redacts_token_shape` | secrets never reach a log line |
| `gitcli::tests::read_capped_at_*` (3) | bounded reads: small streams intact, cap enforced and flagged as truncation, never unbounded |
| `gitcli::tests::missing_program_is_unsupported_not_failed_green` | a missing toolchain is `UNSUPPORTED`, never `PASSED` |
| `json::tests::round_trip` | print→parse round-trip for the values the crate emits |
| `json::tests::rejects_duplicate_keys` / `rejects_trailing_data` | ambiguous or trailing JSON is refused, not guessed |
| `paths::tests::rejects_traversal_and_absolute`, `rejects_windows_hazards`, `accepts_ordinary_paths`, `lexical_boundary` | path validation: traversal/absolute/reserved names refused, ordinary paths accepted, lexical containment component-wise |
| `state::tests::civil_dates`, `config_round_trip`, `rejects_unknown_schema` | timestamps, config round-trip, unknown schema refused |
| `scratch::tests::candidate_names_are_unique_per_sequence`, `within_is_component_wise`, `refuses_non_object_ids`, `bounded_removal_is_idempotent_and_never_follows_links` | scratch naming, containment, object-id validation, bounded removal never follows links |
| `lock::tests::acquires_and_releases`, `a_live_owner_is_never_displaced_even_when_old`, `an_unparseable_lock_is_refused_not_stolen` | lock ownership: no displacement of a live owner, corrupt lock refused and preserved |

### `tests/bounds.rs` — 21 tests

| Test | Invariant |
| --- | --- |
| `run_bounded_feeds_stdin_to_git` | stdin is fed to Git without deadlock |
| `run_bounded_ignores_ambient_index_and_honours_the_explicit_one` | ambient `GIT_INDEX_FILE` cannot redirect the run; an explicit one is honoured |
| `run_bounded_refuses_more_than_64_mib` | a > 64 MiB stream is refused, not buffered |
| `obligation_stdout_is_bounded_through_the_cli` | bounded obligation output end-to-end |
| `blocking_post_checkout_makes_run_refuse_the_scratch` | a blocking hook aborts the run instead of certifying |
| `blocking_reference_transaction_aborts_publish_without_moving_the_ref` | a blocking reference-transaction hook leaves the ref untouched |
| `nonblocking_hooks_are_dispatched_by_run_and_publish` | hooks run in the normal Git order when non-blocking |
| `missing_toolchain_is_unsupported_and_never_publishable` | `UNSUPPORTED` is not publishable |
| `hijacked_git_environment_does_not_redirect_the_cli` | hostile `GIT_*` env cannot redirect the CLI |
| `flood_child_probe`, `index_probe_child` | helper processes used by the fixtures above (guarded probes, counted by CI) |
| (unit names repeated in this binary) | as in §2 unit table |

### `tests/cli.rs` — 41 tests

| Test | Invariant |
| --- | --- |
| `init_creates_state_and_refuses_second_call` | idempotent setup, no silent re-init |
| `disable_blocks_run_and_publish` | the kill switch blocks both mutating paths |
| `doctor_is_read_only_and_reports_guarantees` | `doctor` mutates nothing |
| `p0a_selective_run_never_yields_a_publishable_receipt` | a partial run can never be published |
| `p0a_duplicate_or_unknown_obligation_ids_are_refused` | ambiguous obligation sets are refused |
| `p0a_full_run_with_one_failing_required_obligation_is_never_publishable` | one red required obligation blocks publication |
| `p0a_an_optional_failing_obligation_does_not_mask_a_green_required_set` | optional failure does not mask a green required set |
| `p0a_case_colliding_tree_is_refused_not_certified` | case-colliding trees refused |
| `p0a_symlink_rewritten_as_a_file_is_refused` / `a_converted_checkout_is_refused_not_certified` | materialisation must equal the verified tree |
| `p0b_adding_a_required_obligation_makes_the_receipt_stale`, `p0b_changing_a_timeout_...`, `p0b_changing_the_policy_version_...` | receipt staleness is bound to policy identity |
| `p0b_a_tampered_receipt_is_not_publishable` | receipt tampering detected |
| `p0b_a_timeout_override_is_never_publishable` | `--timeout` overrides do not produce publishable receipts |
| `p0b_an_ambiguous_config_is_refused_not_guessed` | ambiguous config refused |
| `p0b_explicit_ref_must_name_a_branch` / `p1_a_symbolic_ref_is_never_a_publication_target` | publication targets are explicit branches, never symbolic refs |
| `p0c_corrupt_journal_is_not_reported_as_reconciled`, `p0c_status_refuses_to_guess_on_a_corrupt_journal` | corrupt journal is `UNKNOWN`, never `RECONCILED` |
| `p0c_duplicate_intent_id_requires_a_human`, `p0c_one_outcome_cannot_close_two_intents` | one outcome closes at most one intent; duplicates need a human |
| `p0c_reconcile_is_idempotent`, `p0c_ref_moved_elsewhere_needs_reconciliation` | reconciliation is idempotent and detects a moved ref |
| `publish_without_receipt_is_refused`, `publish_with_stale_expected_old_is_refused`, `publish_moves_the_ref_with_cas_and_records_the_journal` | publication is receipt-gated, CAS-fenced and journaled |
| `run_without_obligations_is_unknown_never_passed` | nothing executed is `UNKNOWN` |
| `run_passing_obligation_binds_the_index_tree`, `run_failing_obligation_is_failed_and_publish_refuses` | the receipt binds the exact index tree |
| `obligation_timeout_is_timeout_not_pass` | a timeout is `TIMEOUT`, not `PASSED` |
| `dirty_and_untracked_files_are_preserved_byte_for_byte` | user files are never destroyed |
| `a_symlinked_journal_is_never_written_through` | journal writes do not follow links |
| `crash_after_intent_is_not_published_and_reconciles`, `crash_after_ref_is_observed_as_published` | crash recovery distinguishes both journal positions |
| `unsafe_obligation_id_is_refused` | unsafe ids refused |
| `unknown_and_valueless_options_are_usage_errors` | the option parser refuses unknown flags and valueless value flags |
| `two_concurrent_publishes_publish_at_most_once` | ref CAS publishes at most once |

### `tests/concurrency.rs` — 8 tests

| Test | Invariant |
| --- | --- |
| `eight_concurrent_publishes_publish_at_most_once` | 8 writers, at most one publication |
| `a_live_old_lock_is_never_stolen` | an old but live lock is never displaced |
| `a_dead_owners_lock_is_recovered` | a dead owner's lock is recovered |
| `an_unparseable_lock_is_refused_and_preserved` | corrupt lock refused, preserved |
| `reconcile_is_serialized_and_never_duplicates_outcomes` | reconciliation is serialised and does not duplicate outcomes |
| (unit names repeated) | as above |

### `tests/isolation.rs` — 25 tests

| Test | Invariant |
| --- | --- |
| `scratch_materialises_head_outside_repo_and_cleans_up` | scratch trees live outside the repo and are cleaned |
| `scratch_refuses_paths_inside_the_repository`, `scratch_refuses_non_object_ids`, `scratch_refuses_a_base_that_redirects_into_the_repository` | scratch never writes inside the repository |
| `scratch_never_deletes_a_preexisting_path`, `scratch_never_follows_or_deletes_a_preexisting_candidate_link` | destructive cleanup is bounded: links are never followed |
| `timeout_kills_the_child_and_returns_in_bounded_time`, `surviving_grandchild_does_not_hang_the_runner` | process-tree containment on timeout |
| `obligation_output_is_bounded_and_drained` | output is drained and bounded (no pipe deadlock) |
| `git_ignores_hijacked_repository_environment` | hostile Git env neutralised |
| `flood_child_probe`, `grandchild_child_probe`, `git_env_probe_child`, `timeout_child_probe` | helper processes for the fixtures above |
| (unit names repeated) | as above |

### `tests/proposals.rs` — 13 tests

| Test | Invariant |
| --- | --- |
| `admission_is_idempotent_and_immutable` | same key + same bytes is idempotent; different bytes conflicted |
| `proposals_outside_the_trusted_surface_are_refused` | forgeries outside the trusted surface refused |
| `assembly_reports_order_and_stays_deterministic` | order is part of the candidate id; re-assembly is deterministic |
| `publishing_a_candidate_requires_a_verified_receipt` | a candidate cannot be published unverified |
| `publish_dry_run_plans_without_mutating_anything` | `--dry-run` mutates nothing |
| `binary_and_mode_changes_survive_the_cycle` | binary/mode changes survive admit→assemble→publish |
| `m1_two_disjoint_agents_publish_exactly_the_verified_tree` | the published ref is the verified tree |
| `m2_three_agents_collision_is_diagnosed_and_resolved_explicitly` | collisions are diagnosed and resolved explicitly |
| `m3_stale_base_is_refused_and_a_rebased_proposal_succeeds` | stale base refused, rebase succeeds |
| `m6_user_files_and_index_are_preserved_through_the_whole_cycle` | user files/index preserved through the cycle |
| (unit names repeated) | as above |

---

## 3. Coverage (MEASURED)

Command (one instrumented run, then reports from the same profdata):

```
rustup component add llvm-tools-preview --toolchain nightly-x86_64-pc-windows-msvc
cargo install cargo-llvm-cov --locked            # 2 m 15 s, within the 15 min box
CARGO_BUILD_JOBS=4 cargo +nightly llvm-cov --locked --all-targets --branch --no-report
cargo +nightly llvm-cov report --branch --lcov --output-path experiments/quality/coverage/lcov.info
cargo +nightly llvm-cov report --json  --output-path experiments/quality/coverage/coverage.json
cargo +nightly llvm-cov report --branch > experiments/quality/coverage/summary.txt
```

All 163 tests passed inside the instrumented run, **on the delivered head `997f983`**
(`experiments/quality/coverage/summary-997f983.txt`). `--branch` is cargo-llvm-cov's unstable flag;
it worked on nightly and produced real branch counters, so branch coverage is MEASURED, not
estimated. `--ignore-filename-regex '[\\/]tests[\\/]'` keeps the number on the product code. Line-level
data for this head is `experiments/quality/coverage/lcov-997f983.info`; the `lcov.info` and
`coverage.json` beside it are the `ea181d4` measurement, kept as history.

| File | Lines | Line cover | Branches | Branch cover | Regions | Functions |
| --- | --- | --- | --- | --- | --- | --- |
| `commands.rs` | 1 816 | **81.66 %** (333 missed) | 313 | **68.05 %** (100 missed) | 86.18 % | 83.93 % |
| `gitcli.rs` | 457 | **91.68 %** (38) | 70 | **78.57 %** (15) | 93.64 % | 92.00 % |
| `json.rs` | 290 | **87.24 %** (37) | 48 | **81.25 %** (9) | 85.71 % | 96.67 % |
| `lock.rs` | 322 | **84.16 %** (51) | 38 | **68.42 %** (12) | 90.98 % | 96.30 % |
| `main.rs` | 240 | **75.00 %** (60) | 46 | **80.43 %** (9) | 84.77 % | 100 % |
| `paths.rs` | 210 | **96.67 %** (7) | 46 | **91.30 %** (4) | 95.61 % | 86.36 % |
| `proposal.rs` | 770 | **74.94 %** (193) | 146 | **55.48 %** (65) | 79.89 % | 75.51 % |
| `scratch.rs` | 297 | **71.04 %** (86) | 72 | **55.56 %** (32) | 77.09 % | 82.14 % |
| `sha256.rs` | 117 | **100 %** | 10 | **100 %** | 100 % | 100 % |
| `state.rs` | 497 | **92.35 %** (38) | 56 | **66.07 %** (19) | 90.77 % | 80.56 % |
| **TOTAL** | **5 016** | **83.19 %** (843) | **845** | **68.64 %** (265) | **86.94 %** | **85.82 %** |

Lowest branch coverage: `scratch.rs` 55.56 %, `proposal.rs` 55.48 %, `state.rs` 66.07 % — exactly
the modules where the whole-crate mutation run (`NOT_RUN`, see below) is missing. `paths.rs` and
`lock.rs` crossed their own line gates after the mutation-driven test wave; the global gate
(90 %/85 %) is not met.

---

## 4. Mutation testing (bounded — MEASURED on a subset, NOT_RUN for the rest)

```
cargo mutants --list > experiments/quality/mutation/all-mutants-list.txt      # 960 mutants
cargo mutants --file crates/flow/src/paths.rs --file crates/flow/src/lock.rs \
  -o experiments/quality/mutation-997f983 --timeout 240                      # head 997f983
```

Bounded subset on the delivered head `997f983`: **82 mutants** in 20 min with a verified green
baseline (3 s build + 30 s test). Per-mutant outcome (`experiments/quality/mutation-997f983/`):

| Outcome | Count |
| --- | --- |
| caught (killed) | **57** |
| missed (survived) | **19** |
| timeout | **0** |
| unviable (did not compile) | **6** |
| **total tested** | **82** |
| **mutation score** (killed ÷ (killed + survived + timeout)) | **75.0 %** (57/76) |

The same subset scored **50.8 %** (30/59) before this wave. The nine tests added for this measurement
target exactly what survived then: `canonical_within`'s return value, the nonce, liveness for a live
and a reaped PID, the acquire budget against a live foreign owner, the guard that must not delete a
successor's lock, a record written after the first read, the path length boundaries, the identifier
sizes, and the lock error rendering. Each of those mutants is killed now.

The full crate has **960** mutants; at ~35 s of suite per mutant a whole-crate run is ≈ 9 h serial
(the suite is 163 tests, ~30 s). `commands.rs` (177 mutants) is **NOT_RUN**, and so are `gitcli.rs`
(98), `json.rs` (138), `main.rs` (62), `proposal.rs` (123), `scratch.rs` (54), `sha256.rs` (98) and
`state.rs` (145). No percentage is claimed for the crate as a whole.

### The 19 survivors — classified, not excused (`missed.txt`)

| Class | Count | Why it survives | What would kill it |
| --- | --- | --- | --- |
| not compiled on this host | 10 | `kill_liveness`, the Linux `pid_alive` and the non-Linux-Unix `pid_alive` are `#[cfg]`-excluded on Windows (`lock.rs:342`, `:351`, `:363`, `:371`), so a Windows run cannot execute them | a Linux mutation run — the Linux job does compile and exercise that path through `liveness_is_answered_for_live_and_dead_pids` |
| equivalent at the `Ok`/`Err` level | 3 | `paths.rs:44` (absolute rule), `:48` (drive rule), `:55` (`.`/`..` rule): the mutated code still refuses the input because a later rule catches it, and only the message changes | asserting the message instead of `is_err()`, i.e. pinning wording rather than behaviour |
| needs fault injection | 6 | `lock.rs:236`, `:239` ×3 (error-kind guards), `:294` (nonce match before a steal) and `:208` (deadline boundary): an IO failure or a won race is required | a fault-injection hook for the lock, which does not exist |

The four `lock.rs` message/`Display`/`From` mutants and the `nonce` mutant that survived the earlier
run are killed now by `a_lock_error_renders_its_message` and `nonces_are_unique_and_file_safe`.

---

## 5. Property harness (MEASURED, no dependency added)

`experiments/quality/properties/props.rs` — standalone Rust file, compiled with
`rustc` (no cargo project, `experiments/` is excluded from the workspace), reaching
the crate's own `json.rs`, `paths.rs`, `sha256.rs` through `#[path]` — the same
mechanism `crates/flow/tests/cli.rs` uses:

```
rustc --edition 2021 -O experiments/quality/properties/props.rs -o target/quality-tools/props.exe
target/quality-tools/props.exe --flow target/debug/flow.exe      # seed 20261110
```

Seed **20261110** (printed), 30.7 s, 7/7 properties passed (`properties/run.log`):

| Property | What it proves | Scale |
| --- | --- | --- |
| P1 JSON round-trip | generated JSON (escapes, control chars, `é中🦀`, `-0.0`, `1e21`, `1e300`, `f64::MAX`, `f64::MIN_POSITIVE`) prints and re-parses exactly, compact and pretty; printing is canonical | 2 000 values |
| P2 duplicate-key rejection | duplicate names refused flat, nested, and with identical values | 1 000 objects |
| P3 dangerous paths refused | `..`, absolute, NUL/control bytes, empty, reserved device names, trailing dot/space, colons, >255-byte components, >4 096-byte paths | 29 fixed + 20 000 generated |
| P4 accepted paths are safe | every accepted path has no `.`/`..` component, no empty component, no `:`, no trailing dot/space, no reserved stem, is relative, ≤4 096 bytes, no control byte | 20 000 paths (2 238 accepted) |
| P5 proposal-id derivation | 32 lowercase hex, deterministic, whitespace-trim invariant, **0 collisions** | **100 000 inputs** |
| P5b formula ↔ CLI | the documented formula equals the id the shipped `flow propose` returns; resubmission is idempotent | 24 real CLI proposals |
| P6 assembly order | order changes `candidate_id` (`113d3a36…` vs `dae93fbb…`) but not `tree_oid` (`5e9d2d6c…`) for disjoint proposals, driven through the CLI on a disposable fixture | 1 fixture, 3 assemblies |

P1/P2 use the crate's `json` module directly; P5b/P6 drive `target/debug/flow.exe`
on a real Git fixture. The id formula is reproduced in the harness (the crate
exposes no public derivation API for a binary crate) — its correctness is tied to
the shipped binary by P5b, and the 100 k collision run exercises the crate's own
`sha256` implementation on that material.

---

## 6. Fuzzing (MEASURED, hand-rolled, seeded)

`cargo-fuzz` was **not** used: it needs a `cargo-fuzz` install plus a nightly
libFuzzer target and produces coverage-guided corpora, while the requirement here
is a deterministic, seed-replayable run. Both harnesses are deterministic; with
`--seconds` only the prefix length depends on machine speed, and `--iters` gives
an exactly replayable prefix.

### 6.1 JSON parser — `experiments/quality/fuzz/json_fuzz.rs`

```
rustc --edition 2021 -O experiments/quality/fuzz/json_fuzz.rs -o target/quality-tools/json_fuzz.exe
target/quality-tools/json_fuzz.exe --out experiments/quality/fuzz --seconds 60   # seed 20261110
```

| Metric | Value |
| --- | --- |
| Seed | **20261110** |
| Target | `crates/flow/src/json.rs::parse` (crate's own parser via `#[path]`) |
| Runtime | 60.000 s |
| Iterations | **8 704 339** (1 088 661 accepted, 7 614 769 refused) |
| Mutations | char insert/delete/replace, token injection, truncate, duplicate, cross-corpus splice, swap |
| Invariants | no panic; empty error message refused; accepted ⇒ print+reparse round-trips (compact and pretty) |
| Findings | **909 occurrences, 2 distinct classes** — G9 FAILED |
| Corpus | 16 seeds + 200 generated accepted inputs (`fuzz/corpus/`) |
| Crashing inputs | `fuzz/crashes/crash-*.json` (input bytes), stats in `fuzz/stats-json.json` |

**Finding F1 (JSON round-trip contract, CLI-reachable).** `parse("1e999")` returns
`Ok(Json::Num(inf))` (Rust's `f64` parser accepts overflow), and the printer then
emits the bare token `inf`, which is not JSON:

```
$ echo -n 1e999 | target/quality-tools/json_fuzz.exe --file /tmp/inf.json
input="…" chars=5
result=PARSED value=inf          # to_string() == "inf"
```

Reachable through the CLI on `.flow` state: a `.flow/proposals/<id>.json` whose
`patch_bytes` is `1e999` makes `flow proposals show <id>` print
`"patch_bytes": inf` and exit 0 (`MEASURED` on a disposable fixture), i.e. the
CLI's "machine-readable JSON for every command" contract is broken for
non-finite numbers. First fuzz hit at iteration 32 237 (input `999991e999`),
artefact `fuzz/crashes/crash-00032237-round-trip-error.json`; the 60 s run
reached the same defect class 909 times. Severity: low (requires corrupted or
forged local state), but the projection is invalid JSON and deserves a guard
(reject non-finite numbers at parse time, or print `null`).

### 6.2 CLI option parser — `experiments/quality/fuzz/cli_fuzz.py`

```
python experiments/quality/fuzz/cli_fuzz.py --seconds 60   # seed 20261110
```

| Metric | Value |
| --- | --- |
| Seed | **20261110** |
| Target | `crates/flow/src/main.rs` option grammar + dispatch, driven as a black box |
| Runtime | 60.4 s, **273 invocations** (71 exit 0, 195 exit 2 usage, 7 exit 1 refusal) |
| Invariants | no panic on stderr; exit ∈ {0,1,2}; stdout empty or valid JSON (usage text allowed for `help`/no args — documented exception); no run exceeds 20 s |
| Findings | **0** — G10 PASSED |
| Corpus | 25 seed vectors + 59 accepted vectors under `fuzz/corpus/`; stats in `fuzz/stats-cli.json` |

Coverage of the two harnesses is **not** claimed as code coverage: the 60 s runs
are behavioural, and the only coverage numbers in this document are §3.

---

## 7. Flakiness

* Full suite on the delivered head: 163/163 Windows, 162/162 Linux (this repo's own evidence, `test-run-full.log`).
* Concurrency suite, re-run 3× back to back after the full suite, same binary:

| Run | Result | Duration |
| --- | --- | --- |
| 1 | 8 passed, 0 failed | 6.94 s |
| 2 | 8 passed, 0 failed | 7.97 s |
| 3 | 8 passed, 0 failed | 7.02 s |

`concurrency-run-1.log` … `concurrency-run-3.log`. No retry, no sleep, no
`--test-threads` change. **MEASURED: no flake observed in 3 runs** — 3 runs cannot
prove absence of flakiness, only its absence in this sample; the timing-sensitive
fixtures (`timeout_kills_the_child_…`, `eight_concurrent_publishes_…`) remain the
most likely future flakes.

---

## 8. What this matrix does NOT cover

| Not measured / not run | Reason |
| --- | --- |
| Whole-crate mutation score (960 mutants) | ≈ 24 h serial, ≈ 6 h at `--jobs 4`; time box. Only `paths.rs` + `lock.rs` (65) measured. |
| `commands.rs` (177 mutants), `proposal.rs` (123), `state.rs` (145), `json.rs` (138), `gitcli.rs` (98), `sha256.rs` (98), `main.rs` (62), `scratch.rs` (54) mutation | same time box; listed in `mutation/all-mutants-list.txt` as NOT_RUN |
| Branch coverage of the coverage instrumentation itself | by construction |
| `cargo-fuzz`/libFuzzer coverage-guided corpora | needs `cargo-fuzz` + nightly sanitizer target; hand-rolled deterministic fuzzers used instead |
| Miri, ASan/UBSan, concurrency stress beyond 8 writers | not attempted: no `unsafe`, and the mutation/coverage budget was prioritised |
| Linux behaviour | this evidence is Windows-only; `flow-ci.yml` still runs ubuntu-latest + windows-latest |
| GitHub Actions result for `f237d57` | branch not pushed from this worktree (NOT_RUN here; the PR is the source of truth) |
| HTML coverage report | deliberately left out: text summary + LCOV + JSON are the artefacts; the HTML tree is multi-MB and adds no evidence |

## 9. Artefacts

```
experiments/quality/
├── coverage/
│   ├── summary-997f983.txt  instrumented coverage report (163 passed)
│   ├── lcov.info            raw LCOV (line + BRDA branch records)
│   ├── coverage.json        raw llvm-cov JSON export
│   ├── summary.txt          per-file table (source of §3)
│   ├── report-lcov.log, report-json.log
│   └── run-branch.log
├── mutation/
│   ├── run-paths-lock.log   13 m 09 s bounded run, per-mutant outcomes
│   ├── all-mutants-list.txt 960 mutants (full-crate population, NOT_RUN)
│   ├── caught.txt, missed.txt (19 survivors), timeout.txt, unviable.txt
│   ├── outcomes.json, mutants.json    cargo-mutants machine-readable results
│   └── install.log
├── properties/
│   ├── props.rs             standalone seeded harness (7 properties)
│   └── run.log              seed + per-property results
├── fuzz/
│   ├── json_fuzz.rs         seeded JSON-parser mutation fuzzer
│   ├── cli_fuzz.py          seeded CLI option-parser fuzzer
│   ├── run-json.log, run-cli.log, run-json-a-first-finding.log, run-json-b-capped-findings.log
│   ├── stats-json.json       JSON fuzz stats (60 s run)
│   ├── stats-cli.json        CLI fuzz stats (60 s run)
│   ├── corpus/               16 seeds + 200 accepted JSON inputs; 59 accepted CLI vectors
│   └── crashes/             crashing JSON inputs (F1)
├── test-run-full.log        full suite (163 on Windows, 162 on Linux)
├── concurrency-run-1..3.log flakiness re-runs
├── test-inventory.txt       executed test names per binary
└── inventory.py             inventory extractor
```

`mutants.out/` (cargo-mutants scratch, ~1.2 MB of per-mutant stdout) is **not**
committed: its summary files (`caught/missed/timeout/unviable/outcomes/mutants`)
were copied into `experiments/quality/mutation/`. No binaries or HTML trees are
committed.

## 10. Verdict

* The suite is green and the concurrency suite did not flake in 3 runs.
* Line coverage on the delivered head is **83.19 %** and branch coverage **68.64 %** overall
  (`997f983`); the weakest modules (`scratch.rs` 55.56 % branches, `proposal.rs` 55.48 %,
  `state.rs` 66.07 %) are also the ones with no mutation measurement, so the two signals still
  agree on where the pressure is missing.
* Mutation testing found **19 survivors in 76 testable mutants (75.0 % killed)** on the two modules
  measured. The survivors that mattered — the containment check `canonical_within`, the lock
  `nonce`/`lock budget`/`pid_alive` logic and the lock error rendering — were test gaps, not product
  defects, and this wave closed every one of them with a test (the remaining 19 are classified in §4:
  10 are `#[cfg]`-excluded on Windows, 3 are equivalent at the `Ok`/`Err` level, 6 need fault
  injection).
* Fuzzing found one real contract defect (**F1**, non-finite numbers printed as
  `inf`) that is reachable through `flow proposals show` on corrupted state.
* Everything that was not measured is labelled `NOT_RUN`/`NOT_MEASURED` with its
  reason; no number in this document is estimated.
