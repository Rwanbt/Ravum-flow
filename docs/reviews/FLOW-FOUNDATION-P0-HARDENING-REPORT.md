# Flow foundation — P0 hardening report

**Date:** 2026-11-09 (Europe/Paris) · **Branch:** `fix/flow-foundation-proof-gates` · **PR:** #52 (draft, base `feat/flow-verified-foundation`)
**Code SHA under test:** `9b96011` · **Baseline:** `832f499`
**Verdict:** **`READY_FOR_REVIEW`** — the three blocking defects are reproduced, closed and re-tested; every gate fails closed on the attempts made against it. `FOUNDATION_PARTIAL` remains the verdict for the foundation as a whole, and the non-guarantees in §9 are unchanged.

This report is deliberately narrow. It is not a new architecture study, not an ADR ratification, not a release, and not a merge authorization. PR #51 stays a draft and is untouched.

---

## 1. What was in scope

| In scope | Out of scope |
|---|---|
| Correcting the existing `crates/flow` core | A new VCS/VFS/CRDT/daemon/scheduler |
| Reproducing the three blocking defects before fixing them | A new confirmatory S9 campaign |
| Tests on disposable fixtures, Windows first, Linux via CI | Any write, build, branch or PR in `Rwanbt/Ravum` |
| CI honesty, documentation, GitHub/vault traceability | Merging #51 or #52, publishing a release, ratifying ADRs, choosing a licence, changing branch protection |

`Rwanbt/Ravum` was never opened for write, never built and never modified during this wave. Its `HEAD` was not read for this report, so no earlier Ravum finding is restated as still present.

## 2. Status vocabulary

Every claim below carries one of these, and they are not interchangeable.

| Label | Meaning |
|---|---|
| `CODE_READ` | the behaviour was read in the source, not executed |
| `REPRODUCED` | observed by running it on this host, with the command recorded |
| `FIXED_AND_TESTED` | a test that failed on the baseline now passes, and the test is committed |
| `NOT_REPRODUCED` | attempted and did not happen |
| `NOT_RUN` | not attempted on this host/OS |
| `UNSUPPORTED` | deliberately not provided; the tool refuses instead |
| `UNKNOWN` | no faithful claim is possible |

`CI green` means the named commands ran and passed. It never means "safe against malicious tests".

## 3. The baseline had three blocking defects

Thirteen tests were written against the observable CLI contract (JSON + exit codes) and run **before** any fix, on a throwaway worktree of `832f499` with the final test file copied in.

```
cargo test --locked --test cli
test result: FAILED. 20 passed; 13 failed
```

Verbatim output and the per-test explanation: [`experiments/p0/baseline/`](../experiments/p0/baseline/). Four further tests pass on both sides by design and are labelled guards there, not reproductions.

## 4. P0-A — a partial execution was recorded as a complete proof

**Root cause (`CODE_READ` at `832f499`).** `cmd_run` filtered the declared obligations down to `--obligation ID` (`commands.rs:349-355`), initialised the overall status to `"PASSED"` unconditionally (`:388`), and stored the receipt under the tree OID alone. `cmd_publish` then checked only `receipt.status == "PASSED"` and the same tree OID (`:533-557`).

**Reproduced (`REPRODUCED`).** With two required obligations (`green` passing, `red` failing):

```
flow run --obligation green   -> exit 0, "status":"PASSED",
                                 obligations: [green] only,
                                 receipt: <tree>.json
flow publish                  -> PUBLISHED
```

The published ref carried a tree for which one required obligation had never run.

**Correction (`FIXED_AND_TESTED`).**

- The receipt carries `verification_scope`: `mode` (`complete` / `selected`), `declared`, `required`, `selected`, `executed`, `missing_required`.
- A selective run is `PARTIAL` with exit 1 and is written to `<tree>.partial.<context>.json`, so it can never occupy or overwrite the complete slot.
- `publish` refuses anything but `mode == complete` (`SCOPE_INCOMPLETE`), then re-checks **every currently required obligation by id** for `PASSED` (`MISSING_REQUIRED`, `REQUIRED_NOT_PASSED`) instead of trusting the global status.
- Unknown ids, duplicated ids in the request, duplicate ids in the policy, unsafe ids, and a policy with no required obligation are refused.

An obligation that is optional is traced with `required: false` and does not mask a green required set.

## 5. P0-B — a receipt stayed valid after the policy changed

**Root cause (`CODE_READ`).** `build_context_id` (`commands.rs:497-520`) hashed the tree, a `policy_version` **constant**, the Git version, OS/architecture and each obligation's id/program/args. It omitted `required`, `timeout_s`, the configured `policy_version`, the execution boundary and the environment the obligations inherit. `cmd_publish` never recomputed or compared it at all.

**Reproduced (`REPRODUCED`).** Each of these published successfully on the baseline:

| Change after a green run, without touching the tree | Baseline | Now |
|---|---|---|
| add a required obligation | `PUBLISHED` | `REFUSED` / `STALE_POLICY` |
| change `timeout_s` 30 → 1 | `PUBLISHED` | `REFUSED` / `STALE_POLICY` |
| change `policy_version` | `PUBLISHED` | `REFUSED` / `STALE_POLICY` |
| `run --timeout 1` then publish | `PUBLISHED` | `REFUSED` / `STALE_POLICY` |
| edit `"FAILED"` → `"PASSED"` in the receipt file | `PUBLISHED` | `REFUSED` / `RECEIPT_INTEGRITY` |
| duplicate `required` key in the config | first value silently wins | refused, `ERROR` |
| `publish --ref HEAD` | moves the branch behind `HEAD` | `REFUSED` / `REF_NOT_ALLOWED` |

**Correction (`FIXED_AND_TESTED`).**

- An **effective policy** object — `policy_version`, `execution_boundary`, every obligation's `id/program/args/required/timeout_s`, and the obligation environment allowlist — is hashed into `policy_digest` and into the `context_id` under a versioned codec. The receipt embeds the policy it proves.
- The receipt carries an `integrity` digest over its own content, checked before use.
- `publish` rebuilds the policy from the **current** configuration and refuses on any difference; it also refuses an outdated receipt schema, a receipt that does not name this tree, a commit that does not carry exactly the qualified tree, and a required obligation that is absent or not `PASSED`.
- The destination must be a full `refs/heads/...` ref. A tree the ref already carries is `ALREADY_PUBLISHED`.

**Honest boundary.** The `integrity` digest is integrity, not authentication. It stops accidental corruption and a casual edit of one file. It does **not** stop a writer who can rewrite the configuration and the receipt together and recompute the digest — that would need a key Flow does not have. `doctor` lists this as a non-guarantee.

## 6. P0-C — a corrupt journal was reported as a successful reconciliation

**Root cause (`CODE_READ`).** `pending_intents` contained `let Ok(records) = state::read_journal(root) else { return Vec::new(); };` (`commands.rs:695-717`). Every read or parse error became an empty intent list, and `cmd_reconcile` then returned `"status":"RECONCILED"` with exit 0.

**Reproduced (`REPRODUCED`).**

| Situation | Baseline | Now |
|---|---|---|
| truncated journal with a pending intent | `RECONCILED`, exit 0, `resolved: []` | `CORRUPT_JOURNAL`, exit 1, `manual_required: true` |
| two intents sharing one id | both "resolved" | `NEEDS_RECONCILIATION`, exit 1, left open |
| ref moved to a third commit | `NEEDS_RECONCILIATION` inside exit 0 | `NEEDS_RECONCILIATION`, exit 1 |
| `flow status` on an unreadable journal | `pending_intents: 0` | exit 1, error names the line |

**Correction (`FIXED_AND_TESTED`).** `read_journal` returns a typed error (`Io` vs `Corrupt`) and it is propagated: an unreadable file is `UNKNOWN_OUTCOME`, a torn one `CORRUPT_JOURNAL`, both with `manual_required: true`, `journal_preserved: true`, no ref movement and no journal rewrite. A duplicated intent id, an incomplete intent, an absent ref, or an observation matching neither `old` nor `new` is `NEEDS_RECONCILIATION` and is **not** closed by appending an outcome. `status` and `doctor` no longer report an unreadable journal as empty.

A local advisory lock now serialises `publish` and `reconcile`. It is a lock file, not a lease: a stale lock is recovered only by observing that its owner is gone (pid liveness, or age beyond 120 s), never by stealing on ignorance. It does not fence an external `git` writer — that stays a documented non-guarantee.

## 7. P1 hardening

| Item | State | Evidence |
|---|---|---|
| Bounded obligation output (1 MiB per stream + truncation marker, surplus drained) | `FIXED_AND_TESTED` | `obligation_output_is_bounded_and_drained`, `read_capped_*` |
| A surviving grandchild can no longer hang the CLI after a timeout | `FIXED_AND_TESTED` | `surviving_grandchild_does_not_hang_the_runner` |
| Timeout kills the process tree | `FIXED_AND_TESTED` (Windows `taskkill /T`, Unix process group) | `timeout_kills_the_child_and_returns_in_bounded_time` |
| A descendant that calls `setsid`/`setpgid` escapes the group | `UNSUPPORTED`, documented | doc comment on `run_obligation` |
| OS sandbox for untrusted code | `UNSUPPORTED`, refused-by-claim | `doctor` non-guarantees |
| Owned scratch directory, no delete-before-check, `.git` link removed, RAII cleanup | `FIXED_AND_TESTED` | `scratch_*` (6 tests) |
| Git environment scrubbed (`GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, object/config redirection, askpass, external diff/editor) | `FIXED_AND_TESTED` | `git_ignores_hijacked_repository_environment` |
| Duplicate JSON keys refused at any depth | `FIXED_AND_TESTED` | parser unit test + `p0b_an_ambiguous_config_*` |
| Flow's own state kept out of the verified tree (`.flow/.gitignore`) | `FIXED_AND_TESTED` | `run_passing_obligation_binds_the_index_tree` |
| Git hooks executed | `NOT_RUN` by design | `doctor` |
| Completeness proof of the build context | `UNKNOWN` | `doctor` |

## 8. Verification matrix

| Scenario | Baseline (`832f499`) | Corrected (`9b96011`) | OS | Command | Artefact |
|---|---|---|---|---|---|
| selective run must not publish | FAIL (published) | PASS | Windows | `cargo test --locked --test cli` | `experiments/p0/baseline/cli-tests-baseline.log` |
| policy change after run | FAIL (published) | PASS | Windows | idem | idem |
| tampered receipt | FAIL (published) | PASS | Windows | idem | idem |
| corrupt journal | FAIL (exit 0) | PASS | Windows | idem | idem |
| duplicate / unknown obligation ids | FAIL (accepted) | PASS | Windows | idem | idem |
| `--ref HEAD` | FAIL (moved branch) | PASS | Windows | idem | idem |
| ambiguous config | FAIL (guessed) | PASS | Windows | idem | idem |
| timeout override unpublishable | FAIL (published) | PASS | Windows | idem | idem |
| duplicate intent id | FAIL (auto-resolved) | PASS | Windows | idem | idem |
| ref moved elsewhere | FAIL (exit 0) | PASS | Windows | idem | idem |
| status on corrupt journal | FAIL (`pending 0`) | PASS | Windows | idem | idem |
| concurrent publish | PASS (guard) | PASS | Windows | idem | idem |
| bounded output / grandchild / scratch / git env | n/a | PASS | Windows | `cargo test --locked --test isolation` | — |
| full suite on Linux | n/a | PASS | Ubuntu (CI) | `cargo test --locked --all-targets` | CI artefact `flow-test-output-ubuntu-latest` |
| full suite on Windows | n/a | PASS | Windows (CI) | idem | CI artefact `flow-test-output-windows-latest` |
| R0-b risk corpus (25 scenarios) | 25 `REPRODUCED` | replayed, same verdicts | Windows | `python experiments/r0b/{git,security,verification}/run_tests.py` | `experiments/r0b/*/RESULTS.json` |
| R0-a harness self-tests | 9 OK | 9 OK | Windows | `python -m unittest discover -s experiments/r0a/tests -v` | — |

R0-a/R0-b are **not** tests of this patch. They describe Git and OS risks measured on this host; the replay confirms the corpus still reproduces and that no verdict was rewritten.

**Tests executed, not merely discovered (`REPRODUCED`):** 80 tests, 0 ignored — 21 unit, 34 CLI, 25 isolation.

**CI on the final code SHA (`REPRODUCED`):** runs `42680405452` (push) and `42680418661` (pull request) at `9b96011`, both `success`: `ubuntu-latest` 18–20 s, `windows-latest` 1 m 50 s – 1 m 55 s. The job refuses any `ignored` test, requires at least 80 executed passes, and requires all 17 named gate tests to appear as `... ok`, so deleting or filtering one fails the build. The raw output is uploaded as `flow-test-output-{ubuntu,windows}-latest`.

**Independent review.** The P1 isolation lot was reviewed separately; that review found two real defects in the merged test (a probe that spawned a second copy of the test binary and held its image open on Windows, and an assertion too loose to detect a join-based runner). Both are fixed in `f209408`. A targeted adversarial review of the P0 diff — forged receipts, policy replay, journal corruption, ref games — was run in a separate throwaway worktree; its outcome and any accepted finding are recorded in PR #52 rather than restated here as if self-certified.

**Resource envelope on this host (16 GB), measured:** CLI suite 7.92 s wall, peak working set ≈ 464 MB (34 tests in parallel, each spawning `flow` and `git`); isolation suite 2.11 s, ≈ 32 MB; unit suite < 0.1 s. Peak memory per stream is bounded by construction at 1 MiB. The suite needs no `-j1`; nothing approaches the machine's limits.

## 9. What remains unproven

These are not caveats in passing; they are the boundary of the claim.

1. **No OS isolation.** Obligations run with the user's rights. Untrusted code execution is `UNSUPPORTED`, and the `doctor` report says so.
2. **No hook execution.** A repository whose real verification lives in hooks is not verified by Flow.
3. **No completeness proof of the context.** `context_id` covers declared inputs only; an undeclared input (an ignored cache, an environment variable, a toolchain version not in the receipt) can still change the result. `R0-b` `CACHE-13`/`CONTEXT-03` remain standing risks.
4. **Receipt integrity is not a signature** (§5).
5. **The lock is local and advisory**; a foreign `git` writer bypasses it entirely (`FENCE-07b` unchanged).
6. **Linux evidence is CI-only** for the Rust suites. The R0/S9 corpus was executed on Windows; it is `NOT_RUN` on Linux and is not claimed there.
7. **`publish` moves refs only.** `HEAD`, the index and the worktree can diverge afterwards. There is no checkout, by design.
8. **A file type Git cannot represent faithfully on the host** (case collision, reserved name, symlink outside the tree) is reported by `doctor`; Flow refuses rather than normalising, but it does not repair the repository.

## 10. Decisions and S9 are unchanged

- `H2` (licence) was already decided by the owner: MPL-2.0 (`dev` `2c8cb62`, `main` `9d5170a`). This wave records it, it does not re-decide it. Extraction rights from Ravum/third parties remain to be examined separately.
- `H5` was granted **once**, bounded, on 2026-11-09. This wave is a continuation of corrective work, not a standing authorization.
- `H4` (S9 threshold, absolute floor, budget, confirmatory `n`) is **open**. No `n = 8` run, no confirmatory campaign and no new workload was started.
- `H1`, `H3`, `H6`, `H7`, `H8` remain open. No ADR/RFC is ratified.
- The S9 pilot (`n = 5`, exploratory) is unchanged. Its statistics are stated as they are: B/E median ratio ≈ 0.9555, and the one-sided p = 1.0 for "B slower than E" does **not** prove equivalence. The arms ran sequentially, without agent thinking latency and without CPU contention, so nothing here supports "Flow is faster in practice".

## 11. Verdict

**`READY_FOR_REVIEW`.** The three blocking defects have a committed failing test on the baseline and a passing test after the fix. The publication gate fails closed on selection changes, policy changes, receipt edits, ambiguous JSON, unbounded output, hijacked Git environments and an unreadable journal. No user file was lost or modified in any scenario exercised. CI on Ubuntu and Windows counts executed tests and refuses a skipped one.

**`FOUNDATION_PARTIAL` stands** for the foundation as a whole: a useful, honest core whose unqualified capabilities are refused rather than simulated.

The next bounded gate, if one is wanted, is not more code: it is `H4` (a numeric S9 decision) plus a Linux run of the R0 corpus, which would convert the largest remaining `NOT_RUN` into evidence. Neither is authorized by this mission.
