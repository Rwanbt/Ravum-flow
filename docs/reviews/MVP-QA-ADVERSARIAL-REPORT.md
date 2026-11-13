# Ravum Flow MVP 0.1 — independent adversarial QA report

**Date:** 2026-11-09 (Europe/Paris) · **Branch under test:** `work/mvp-qa` at **`d884e29c825cad40678c56702361f4e403a769f7`** · **Binary under test:** `target/debug/flow.exe` built from that commit, sha256 `a973efd815490e8a…`
**Harness:** [`experiments/mvp-qa/attacks.py`](../experiments/mvp-qa/attacks.py) — one entry point, stdlib only, disposable fixtures only:
`cargo build --manifest-path Cargo.toml && python experiments/mvp-qa/attacks.py --flow target/debug/flow.exe --out experiments/mvp-qa/results.json --markdown experiments/mvp-qa/verdict-tables.md`
**Runs:** two complete runs, **identical id/verdict/severity for all 102 scenarios** (deterministic). Total wall time 104 s; slowest scenario 3.4 s; peak Flow working set (sampled via `GetProcessMemoryInfo`) 8.0 MB in the 10 MiB-flood scenario.

> **Scope of this report.** These results are for the **pre-0.1 surface at `d884e29`** (the branch in this worktree). The Lead later reported that `feat/flow-mvp-0.1` (PR #53) adds `approve`, `propose`, `proposals`, `assemble`, `--candidate`, `--dry-run`, `flow-lock/2` and a bounded Git layer. **That head was NOT tested here** — the Lead's checkout was not used and the new commands are absent from the binary under test. The seven `FUT-*` scenarios are `NOT_RUN` for that exact reason, not because they are believed safe. Re-running against the new binary is the first thing to do after this report.

## 1. Verdict

| | |
|---|---|
| **Attacks executed** | **93 of 102** declared (`REPRODUCED` + `REFUTED`); 9 `NOT_RUN`; 0 harness errors |
| **REPRODUCED** | **17** — 4 × **P0**, 4 × P1, 7 × P2, 2 × P3 |
| **REFUTED** | 76 — the publication gate itself held in every identity, scope and durability scenario |
| **Overall** | **NOT READY at `d884e29`** — the P0 class is real and reachable from a cloned candidate; the gate code (identity, scope, durable recovery) is sound on every attempt made. |

The four P0s share one root cause: **Flow's own state paths (`.flow/journal.jsonl`, `.flow/receipts`) are used without checking that they are regular, unlinked paths inside the workspace.** An absolute symlink planted locally, or shipped in a candidate tree and materialised by `git clone` on a symlink-capable host, makes `flow run` (`receipts/`) or `flow publish` (`journal.jsonl`) write **outside the repository**, into an attacker-chosen file or directory. This contradicts the product's own boundary ("no write outside the allowed workspace", "no destructive overwrite of user files") and is the only P0 class found.

The second finding class (P1) is the **proof's content** rather than its identity: obligations see checkout-*converted* bytes (`.gitattributes eol`, `core.autocrlf`), not the Git blob the receipt names (`FIL-08`); unknown options are silently ignored, so `publish --dry-run` and `publish --ref` (no value) **really publish** (`PRC-11`); a clonable repository can supply `.flow/config.json` and have `flow run` execute its program with no runtime warning (`FIL-15`); and a symbolic ref under `refs/heads/` can make `publish` move a **tag** (`GIT-10`).

## 2. What was attacked, per axis

| Axis | Declared | Executed | REPRODUCED | REFUTED | NOT_RUN | P0s |
|---|---|---|---|---|---|---|
| 1. Identity / receipt scope | 20 | 20 | 0 | 20 | 0 | 0 |
| 2. Scope (portée) | 11 | 11 | 0 | 11 | 0 | 0 |
| 3. Git refs / CAS | 12 | 12 | 2 | 10 | 0 | 0 |
| 4. Durable journal / recovery | 11 | 10 | 0 | 10 | 1 | 0 |
| 5. User files / hostile paths | 15 | 14 | 7 | 7 | 1 | 4 |
| 6. Process containment | 13 | 13 | 2 | 11 | 0 | 0 |
| 7. Proof quality | 6 | 6 | 5 | 1 | 0 | 0 |
| 8. Compatibility | 7 | 7 | 1 | 6 | 0 | 0 |
| 9. New 0.1 commands | 7 | 0 | 0 | 0 | 7 | — |
| **Total** | **102** | **93** | **17** | **76** | **9** | **4** |

The core publication gate — a `PASSED` receipt must name the exact index tree and the exact effective policy — held on **every** attempt: index-tree change (`NO_RECEIPT`), added/changed/removed required obligation, `timeout_s`, `policy_version`, `execution_boundary`, `--timeout` override (`STALE_POLICY`), edited receipt (`RECEIPT_INTEGRITY`), stale schema (`RECEIPT_SCHEMA_STALE`), copied receipt (`TREE_MISMATCH`), partial run (`PARTIAL` + no complete slot), red/exception/timeout/missing-tool obligations (`FAILED`/`UNSUPPORTED`/`TIMEOUT` + `REQUIRED_NOT_PASSED`), empty policy (`UNKNOWN`), truncated journal (`CORRUPT_JOURNAL`, preserved, exit 1), duplicate intent id (`NEEDS_RECONCILIATION`, no outcome appended), crash after intent/after CAS (`RECONCILED` with the right resolution, never double publication), stale `--expected-old` and an external `git update-ref` between run and publish (`STALE_REF`, no journal intent, ref kept).

## 3. Verdict tables per axis

### Identity / receipt scope

| ID | Attack | Command (excerpt) | Expected refusal | Observed | Verdict | Severity |
|---|---|---|---|---|---|---|
| IDN-01 | Index-tree change after a green run | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | publish refuses; the receipt does not name the new index tree | exit 1 status=REFUSED code=NO_RECEIPT ref_moved=False | **REFUTED** | P0 |
| IDN-02 | Worktree-only change after a green run | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | publish is defined on the INDEX tree: a dirty worktree does not change the verified tree | PUBLISHED; worktree bytes untouched | **REFUTED** | P2 |
| IDN-03 | Required obligation added after the run | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | publish refuses: a required obligation was added after the run | exit 1 code=STALE_POLICY | **REFUTED** | P0 |
| IDN-04 | Required obligation args changed after the run | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | publish refuses: a required obligation's args changed after the run | exit 1 code=STALE_POLICY | **REFUTED** | P0 |
| IDN-05 | Required obligation removed after the run | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | publish refuses: a required obligation was removed after the run | exit 1 code=STALE_POLICY | **REFUTED** | P0 |
| IDN-06 | timeout_s changed after the run | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | publish refuses: timeout_s changed 30 -> 1 after the run | exit 1 code=STALE_POLICY | **REFUTED** | P0 |
| IDN-07 | policy_version changed after the run | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | publish refuses: policy_version changed after the run | exit 1 code=STALE_POLICY | **REFUTED** | P0 |
| IDN-08 | execution_boundary changed after the run | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | publish refuses: execution_boundary changed after the run | exit 1 code=STALE_POLICY | **REFUTED** | P0 |
| IDN-09 | `--timeout` override used at run time | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | publish refuses a run made under a `--timeout` override | exit 1 code=STALE_POLICY | **REFUTED** | P0 |
| IDN-10 | Receipt edited by hand (no re-seal) | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | publish refuses an edited receipt with RECEIPT_INTEGRITY | exit 1 code=RECEIPT_INTEGRITY | **REFUTED** | P0 |
| IDN-11 | Cosmetic config edit after the run | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | cosmetic config edits do not invalidate the proof (digest is over the effective policy) | PUBLISHED | **REFUTED** | P2 |
| IDN-12 | Partial run after the tree changed | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | a selective run never occupies the complete receipt slot, so publish finds no receipt | exit 1 code=NO_RECEIPT complete_receipt=False partial=['3316702f576753acec1a3e105499fd65c31eeabc.partial.42d52af2a2a9.json'] | **REFUTED** | P0 |
| IDN-13 | Partial run after a complete proof of the same tree | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | an earlier complete proof of the same tree remains valid after a diagnostic partial run | publish exit 0 status=PUBLISHED | **REFUTED** | P2 |
| IDN-14 | Duplicated `--obligation` id | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run --obligation green --obligation green` | a duplicated --obligation id is refused | exit 1 code=DUPLICATE_OBLIGATION_ID | **REFUTED** | P2 |
| IDN-15 | Unknown `--obligation` id | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run --obligation does-not-exist` | an unknown --obligation id is refused | exit 1 code=UNKNOWN_OBLIGATION_ID | **REFUTED** | P2 |
| IDN-16 | Receipt schema edited | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | an outdated receipt schema is refused before integrity is trusted | exit 1 code=RECEIPT_SCHEMA_STALE | **REFUTED** | P0 |
| IDN-17 | Receipt copied to another tree's filename | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | a receipt copied under another tree's filename is refused | exit 1 code=TREE_MISMATCH | **REFUTED** | P0 |
| IDN-18 | Receipt integrity digest removed | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | a receipt without an integrity digest is refused | exit 1 code=RECEIPT_INTEGRITY | **REFUTED** | P0 |
| IDN-19 | Obligation order changed after the run | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | publish refuses: obligation order changed after the run | exit 1 code=STALE_POLICY | **REFUTED** | P2 |
| IDN-20 | Obligation `required` flag flipped true -> false | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | publish refuses: an obligation's `required` flag flipped true -> false | exit 1 code=STALE_POLICY | **REFUTED** | P0 |

### Scope (portee)

| ID | Attack | Command (excerpt) | Expected refusal | Observed | Verdict | Severity |
|---|---|---|---|---|---|---|
| SCP-01 | Empty policy: run | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | an empty policy yields UNKNOWN, never PASSED | exit 1 status=UNKNOWN | **REFUTED** | P0 |
| SCP-02 | Empty policy: publish | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | an empty policy is not publishable | exit 1 code=INVALID_POLICY | **REFUTED** | P0 |
| SCP-03 | Policy with no required obligation | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | a policy with no required obligation is refused | exit 1 code=INVALID_POLICY | **REFUTED** | P0 |
| SCP-04 | Red required obligation | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | a red required obligation is FAILED and never publishable | run=FAILED publish=REQUIRED_NOT_PASSED | **REFUTED** | P0 |
| SCP-05 | Obligation raises a Python exception | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | a Python exception makes the run FAILED and unpublishable | run=FAILED publish=REFUSED/REQUIRED_NOT_PASSED | **REFUTED** | P0 |
| SCP-06 | Required E2E obligation is red | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | a red required E2E obligation is never green, never publishable | run=FAILED publish=REFUSED/REQUIRED_NOT_PASSED | **REFUTED** | P0 |
| SCP-07 | Partial run that skips a required obligation | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | a partial run that skips a required obligation is PARTIAL and publishable never | run=PARTIAL publish=NO_RECEIPT | **REFUTED** | P0 |
| SCP-08 | Optional red obligation does not mask green | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | an optional red obligation does not mask a green required set | PASSED + PUBLISHED | **REFUTED** | P2 |
| SCP-09 | Required obligation whose program is missing | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | a required obligation that cannot start is UNSUPPORTED and unpublishable | run=UNSUPPORTED publish=REQUIRED_NOT_PASSED | **REFUTED** | P0 |
| SCP-10 | Required obligation times out | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | a timed-out required obligation is TIMEOUT and unpublishable | run=TIMEOUT publish=REQUIRED_NOT_PASSED | **REFUTED** | P0 |
| SCP-11 | Disabled repository | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe enable` | disabled repositories refuse run and publish | run=DISABLED publish=DISABLED | **REFUTED** | P2 |

### Git refs and CAS

| ID | Attack | Command (excerpt) | Expected refusal | Observed | Verdict | Severity |
|---|---|---|---|---|---|---|
| GIT-01 | Stale --expected-old | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish --expected-old 0000000000000000000000000000000000000000` | a stale --expected-old is refused with STALE_REF | exit 1 code=STALE_REF | **REFUTED** | P0 |
| GIT-02 | External git update-ref between run and publish | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish --expected-old 1a75341377317fc4c4188537f0e8b6901e919b0b` | an external `git update-ref` between run and publish is not overwritten: STALE_REF, ref kept, no journal intent | exit 1 code=STALE_REF ref_is_external=True | **REFUTED** | P0 |
| GIT-03 | `--ref HEAD` | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish --ref HEAD` | `--ref HEAD` is refused | exit 1 code=REF_NOT_ALLOWED | **REFUTED** | P0 |
| GIT-04 | `--ref refs/tags/x` | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish --ref refs/tags/v1` | `--ref refs/tags/x` is refused | exit 1 code=REF_NOT_ALLOWED | **REFUTED** | P0 |
| GIT-05 | Invalid ref names (`git check-ref-format`) | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish --ref refs/heads/a b` | every invalid/non-branch ref name is refused before any mutation (codes vary: REF_NOT_ALLOWED before resolution, UNKNOWN_REF after rev-parse) | 6 names refused, refs unchanged | **REFUTED** | P2 |
| GIT-06 | Missing destination ref | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish --ref refs/heads/does-not-exist` | a missing destination ref is UNKNOWN_REF | exit 1 code=UNKNOWN_REF | **REFUTED** | P0 |
| GIT-07 | Non-OID --expected-old | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish --expected-old not-an-object-id` | a non-OID --expected-old cannot reach `git update-ref`: STALE_REF | exit 1 code=STALE_REF | **REFUTED** | P0 |
| GIT-08 | Detached HEAD | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish --ref refs/heads/main` | a detached HEAD is refused unless `--ref` names the branch; an explicit branch still publishes | no_ref=ERROR/'detached HEAD: pass --ref explicitly' explicit=PUBLISHED | **REFUTED** | P2 |
| GIT-09 | Symbolic ref under refs/heads/ (branch target) | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish --ref refs/heads/sym` | a symbolic ref is refused, or the tool states that the target moved | PUBLISHED under the name refs/heads/sym while the real target refs/heads/target moved to a39d1698a62d (tree aaca6f6b8a7a = verified tree); symref unchanged | **REPRODUCED** | P2 |
| GIT-10 | Symbolic ref dereferencing to a tag | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish --ref refs/heads/sym` | publish must move a branch only: a symref must not make it move a tag | PUBLISHED reported ref=refs/heads/sym; refs/tags/v1 moved d81ed3f7e9d2 -> 19f40a69f5f7 | **REPRODUCED** | P1 |
| GIT-11 | Blocking reference-transaction hook at publish | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | a blocking reference-transaction hook aborts publish with a typed refusal, ref untouched, intent closed as refused | exit 1 code=UPDATE_REF_REFUSED ref_unchanged=True journal_outcome=REFUSED | **REFUTED** | P2 |
| GIT-12 | Tree already carried by the ref | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | publishing a tree the ref already carries is refused, no empty commit | exit 1 code=ALREADY_PUBLISHED | **REFUTED** | P2 |

### Durable journal and recovery

| ID | Attack | Command (excerpt) | Expected refusal | Observed | Verdict | Severity |
|---|---|---|---|---|---|---|
| DUR-01 | Truncated journal line | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe status` | a truncated journal is CORRUPT_JOURNAL (exit 1), preserved, and status refuses to guess | reconcile=CORRUPT_JOURNAL/JOURNAL_CORRUPT status_exit=1 journal_preserved=True | **REFUTED** | P0 |
| DUR-02 | Duplicated intent id, one outcome | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe reconcile` | a duplicated intent id cannot be closed by one outcome: NEEDS_RECONCILIATION (exit 1), journal and ref untouched | exit 1 status=NEEDS_RECONCILIATION journal_unchanged=True | **REFUTED** | P0 |
| DUR-03 | One outcome cannot close two intents (ref moved) | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe reconcile` | one outcome cannot close two intents even when the ref matches: NEEDS_RECONCILIATION (exit 1) | exit 1 status=NEEDS_RECONCILIATION | **REFUTED** | P0 |
| DUR-04 | Crash after intent | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | crash after intent: no ref move, reconcile records NOT_PUBLISHED once, the next publish publishes exactly once | crash_exit=89 reconcile=RECONCILED outcomes=2 publish=PUBLISHED | **REFUTED** | P0 |
| DUR-05 | Crash after CAS (after-ref) | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | crash after CAS: reconcile observes PUBLISHED, a second publish is ALREADY_PUBLISHED and no commit is added | crash_exit=89 reconcile=RECONCILED second=ALREADY_PUBLISHED commits=2 | **REFUTED** | P0 |
| DUR-06 | Reconcile idempotence | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe reconcile` | reconcile is idempotent: a second run appends nothing | RECONCILED twice, journal lines 2 == 2 | **REFUTED** | P2 |
| DUR-07 | Ref moved to a third commit after a crash | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe reconcile` | an observation matching neither old nor new is NEEDS_RECONCILIATION (exit 1), no outcome invented | exit 1 status=NEEDS_RECONCILIATION journal_unchanged=True | **REFUTED** | P0 |
| DUR-08 | Journal deleted after a crash | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe status` | a deleted journal is indistinguishable from a fresh repository: reconcile reports RECONCILED with nothing pending | reconcile=RECONCILED pending_intents=0 | **REFUTED** | P2 |
| DUR-09 | Crash before intent | `(no command)` |  | no injection point exists for a crash before the intent append: FLOW_TEST_CRASH supports only after-intent and after-ref, and killing the process between commit-tree and append-journal is not deterministic from outside | **NOT_RUN** | P2 |
| DUR-10 | Valid-JSON non-object journal line | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe status` | a valid-JSON but non-object journal line is silently ignored: reconcile says RECONCILED | reconcile=RECONCILED status_exit=0 | **REFUTED** | P3 |
| DUR-11 | Crafted two intents with two outcomes | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe reconcile` | a hand-crafted journal with two intents and two outcomes is accepted as reconciled | reconcile=RECONCILED | **REFUTED** | P3 |

### User files and hostile paths

| ID | Attack | Command (excerpt) | Expected refusal | Observed | Verdict | Severity |
|---|---|---|---|---|---|---|
| FIL-01 | Dirty/untracked/read-only/index files preserved | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | `run` modifies no user file, index, status or ref; `publish` modifies no user file or index | run=0 publish=1 changed=[] | **REFUTED** | P0 |
| FIL-02 | Verification reads the index, not the dirty worktree | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | the obligation sees the INDEX content while the dirty worktree is untouched | PASSED on index content; worktree bytes unchanged | **REFUTED** | P1 |
| FIL-03 | Flow state stays out of the verified tree | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | Flow state is gitignored and never enters the verified tree | receipt tree == HEAD tree 5da9bb037560; no .flow entry | **REFUTED** | P1 |
| FIL-04 | Symlink tree entry checked out as a regular file | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | a symlink tree entry checked out as a regular file is refused, not certified | exit 1 code=UNSUPPORTED_CHECKOUT | **REFUTED** | P1 |
| FIL-05 | Case-colliding tree | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | a case-colliding tree is refused (the host cannot represent it faithfully) | exit 1 code=UNSUPPORTED_CHECKOUT | **REFUTED** | P1 |
| FIL-06 | Windows-reserved and `..` tree paths | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe init` |  | Git itself refuses to place a Windows-reserved name or a `..` path in the index (update-index: Invalid path; read-tree refuses the raw tree), so no verified index tree can contain one. Fixture impossible on this host; paths are still valida | **NOT_RUN** | P1 |
| FIL-07 | Obligation id traversal | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | an obligation id with `..` is refused at policy validation | run=INVALID_POLICY publish=INVALID_POLICY | **REFUTED** | P2 |
| FIL-08 | EOL conversion: obligation does not see tree bytes | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | the obligation must see the bytes of the verified tree (the Git blob), or the run must be refused as unfaithful | PASSED: the obligation asserted CRLF and succeeded, while the tree blob for payload.txt is LF (11 bytes: 'line1\nline2'); the receipt names the tree whose blob was never tested. The checkout conversion is driven by a committed .gitattribute | **REPRODUCED** | P1 |
| FIL-09 | Journal symlink -> write outside the workspace | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | Flow must not follow a symlinked `.flow/journal.jsonl` to write outside its workspace | PUBLISHED appended 514 bytes of journal records to 'outside-victim.txt' OUTSIDE the repository (14 -> 528 bytes, sha256 1fc87ad022bb -> 7241e0afe7df) | **REPRODUCED** | P0 |
| FIL-10 | Receipts symlink -> write outside the workspace | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | Flow must not follow a symlinked `.flow/receipts` to write outside its workspace | run exited 0 and wrote receipt ['5da9bb037560f218b886f3b5bd3bf89d312259aa.json'] into 'outside-receipts' OUTSIDE the repository | **REPRODUCED** | P0 |
| FIL-11 | Logs junction -> write outside `.flow` (inside repo) | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | logs must stay under `.flow`, not be redirected into the user's source tree | run wrote ['green.stderr.log', 'green.stdout.log'] into the junction target `src/`; only the repository boundary is checked, not the `.flow` boundary | **REPRODUCED** | P2 |
| FIL-12 | Cloned repo ships a receipts symlink | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | a cloned repository must not be able to redirect Flow's receipt write outside the checkout | clone materialised `.flow/receipts` as a symlink; `flow run` wrote ['baf41f8b26ef7f8cec53bdecdfde3c1b986a38fa.json'] into 'clone-receipts-out' outside the clone | **REPRODUCED** | P0 |
| FIL-13 | Cloned repo ships a journal symlink | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | a cloned repository must not be able to redirect Flow's journal append outside the checkout | clone materialised `.flow/journal.jsonl` as a symlink; `flow publish` appended 514 bytes of journal to a file OUTSIDE the clone (16 -> 530 bytes) | **REPRODUCED** | P0 |
| FIL-14 | Cloned repo ships a journal symlink (default clone) | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | on a host where Git cannot materialise the symlink, the fidelity check refuses the tree instead of certifying it | clone wrote a regular file; run refused UNSUPPORTED_CHECKOUT | **REFUTED** | P0 |
| FIL-15 | Cloned repo ships `.flow/config.json` | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | `flow run` must not execute a program supplied by the repository without saying so | the cloned repo's `.flow/config.json` was used by `flow run` and its obligation executed (config-executed.txt written); run status=PASSED | **REPRODUCED** | P1 |

### Process containment

| ID | Attack | Command (excerpt) | Expected refusal | Observed | Verdict | Severity |
|---|---|---|---|---|---|---|
| PRC-01 | 10 MiB stdout + stderr | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | 10 MiB per stream is drained and truncated with a marker; the run returns in bounded time and memory | PASSED in 492 ms; logs {'flood.stderr.log': 1048629, 'flood.stdout.log': 1048629}; truncated=True; peak_ws=7.9 MB | **REFUTED** | P2 |
| PRC-02 | Obligation ignores termination | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | an obligation ignoring SIGTERM is killed with the tree and the run returns TIMEOUT in bounded time | TIMEOUT in 2353 ms | **REFUTED** | P2 |
| PRC-03 | Detached grandchild holds the pipe | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | a detached grandchild holding the inherited pipe cannot hang the CLI: the reader grace bounds the call | PASSED in 2339 ms with a detached grandchild holding stdout | **REFUTED** | P2 |
| PRC-04 | Very short timeout | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | a 1 s timeout is honoured; the 30 s obligation is killed and the run returns TIMEOUT | TIMEOUT in 1327 ms | **REFUTED** | P2 |
| PRC-05 | GIT_DIR/GIT_WORK_TREE/GIT_INDEX_FILE hijack | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | a hijacked GIT_DIR/GIT_WORK_TREE/GIT_INDEX_FILE does not redirect Flow: it verifies repository A and leaves B untouched | receipt tree == A index tree aaca6f6b8a7a; B untouched | **REFUTED** | P0 |
| PRC-06 | GIT_CONFIG_GLOBAL/GIT_CONFIG_SYSTEM hijack | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | a hostile GIT_CONFIG_GLOBAL/GIT_CONFIG_SYSTEM is scrubbed before every `git` child | PASSED with the real repository | **REFUTED** | P2 |
| PRC-07 | Missing toolchain in an obligation | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | a missing program is UNSUPPORTED, typed and bounded | UNSUPPORTED in 279 ms | **REFUTED** | P2 |
| PRC-08 | post-checkout hook: what runs today | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | documented: Flow dispatches no hook of its own, but `git worktree add` runs a `post-checkout` hook; it executes with cwd = the scratch checkout and user files are untouched | post-checkout ran; cwd='/tmp/flow-scratch-18876-0' | **REFUTED** | P2 |
| PRC-09 | reference-transaction hook: what runs today | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | documented: the Git plumbing runs `reference-transaction` during `worktree add` and `update-ref`; a blocking hook aborts publish with a typed refusal and no ref move | phases during run: ['aborted', 'committed', 'prepared']; blocking publish -> UPDATE_REF_REFUSED, ref unchanged | **REFUTED** | P2 |
| PRC-10 | FLOW_TEST_CRASH in the shipped binary | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | a test-only crash switch is honoured unconditionally by the shipped binary and exits 89, outside the documented exit-code contract (0/1/2) | FLOW_TEST_CRASH=after-intent -> exit 89; after-ref -> exit 89; unrecognised value -> exit 1 | **REPRODUCED** | P2 |
| PRC-11 | Unknown options / missing option values | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run --obligation` | unknown options and options missing their value must be usage errors (exit 2), never a live database- or ref-mutating action | `publish --dry-run` -> PUBLISHED (ref moved: True); `publish --ref` with no value -> PUBLISHED (ref moved: True); `run --obligation` with no value -> FAILED (both obligations executed: True) | **REPRODUCED** | P1 |
| PRC-12 | Host secret canary and log redaction | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | the host canary is not inherited by the obligation and is redacted before persistence | obligation env=None; log and receipt preview contain [REDACTED] | **REFUTED** | P2 |
| PRC-13 | Scratch cleanup | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | the owned scratch checkout is removed on drop: no residue | no new flow-scratch-* directory under PRC-13 | **REFUTED** | P2 |

### Proof quality

| ID | Attack | Command (excerpt) | Expected refusal | Observed | Verdict | Severity |
|---|---|---|---|---|---|---|
| PRF-01 | Re-sealed forged receipt | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | a receipt must not be forgeable by a writer who can also edit and re-seal the file | PUBLISHED a commit for a tree whose required obligation was FAILED: status and obligation entries edited, integrity digest recomputed (0bc37d63ec15...) | **REPRODUCED** | P2 |
| PRF-02 | Log files deleted after the run | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | publish claims evidence, but the evidence files it names can be deleted without any check | PUBLISHED with all 2 log files deleted; publish output reports receipt_logs=2 | **REPRODUCED** | P3 |
| PRF-03 | File changed after run, before publish | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | a worktree edit does not invalidate the index-tree proof (by design); a staged edit changes the tree and is NO_RECEIPT | worktree -> PUBLISHED, staged -> NO_RECEIPT | **REFUTED** | P1 |
| PRF-04 | Tampered receipt displayed by status/explain | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | `status` and `explain` must not present a receipt whose integrity digest no longer matches | status reports latest_receipt.status=PASSED from a receipt that publish refuses with RECEIPT_INTEGRITY; explain echoes the same tampered receipt | **REPRODUCED** | P2 |
| PRF-05 | Doctor honesty and exit code | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe doctor` | `doctor` must fail the process (or at least not exit 0) when its own journal check is FAILED, and its disclosures must match the code | journal check FAILED, overall=FAILED, but exit=0; forge disclosed=True, eol/filter in static non_guarantees=False (only the conditional `gitattributes` check mentions it) | **REPRODUCED** | P2 |
| PRF-06 | `explain --tree` path traversal | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe explain --tree ../../../outside-secret` | `explain --tree` must validate the tree argument, not read an arbitrary JSON file by path traversal or absolute path | explain --tree 'C:\\Users\\barat\\AppData\\Local\\Temp\\flow-mvp-qa-qfgi0q8d\\PRF-06\\outside-secret' returned the outside file content {"secret": "CANARY-FAKE-SECRET-001"}; traversal '../../../outside-secret' returned {"secret": "CANARY-FA | **REPRODUCED** | P2 |

### Compatibility

| ID | Attack | Command (excerpt) | Expected refusal | Observed | Verdict | Severity |
|---|---|---|---|---|---|---|
| CMP-01 | JSON is the default and parseable | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe help` | the usage text promises machine-readable JSON for every command | these invocations printed non-JSON text: ['no-args', 'help'] (exit codes [('no-args', 2), ('help', 0)]) | **REPRODUCED** | P3 |
| CMP-02 | `--json` flag is a no-op | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run --json` | `--json` is accepted (as a no-op) and the output stays JSON: JSON is the default | status exit=0, run exit=0 | **REFUTED** | P2 |
| CMP-03 | Exit-code contract stability | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe publish` | the documented exit-code contract (0 success, 1 refusal, 2 usage) holds for the exercised matrix | all 8 cases match | **REFUTED** | P2 |
| CMP-04 | Two concurrent publishes | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe status` | two concurrent publishes publish at most once: one PUBLISHED, the other refused, one commit, no pending intent | results=[{'exit': 1, 'status': 'REFUSED', 'code': 'ALREADY_PUBLISHED', 'reason': 'refs/heads/main already points at a commit whose tree is aaca6f6b8a7a9c0aa154e87614ed60b93bf38750: nothing to publish'}, {'exit': 0, 'status': 'PUBLISHED', 'c | **REFUTED** | P0 |
| CMP-05 | Concurrent read-only processes | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe init` | concurrent read-only CLI processes do not interfere | [{'exit': 0, 'status': 'REPORT'}, {'exit': 0, 'status': 'REPORT'}, {'exit': 0, 'status': 'VERSION'}] | **REFUTED** | P2 |
| CMP-06 | Two concurrent runs | `<concurrent flow run>` | two concurrent runs must not corrupt the receipt or report a false PASSED | passed=1 failed=1 receipts=['aaca6f6b8a7a9c0aa154e87614ed60b93bf38750.json'] errors=['git ["write-tree"] failed (128): fatal: Unable to create \'C:'] | **REFUTED** | P2 |
| CMP-07 | Unknown configuration keys | `D:\App\Ravum-flow-wt\qa\target\debug\flow.exe run` | unknown configuration keys are ignored (no silent effect), so the policy digest is unchanged | `sandbox: true` and an obligation `env` map have no effect; obligation env=None | **REFUTED** | P3 |

### Future commands (absent at this commit)

| ID | Attack | Command (excerpt) | Expected refusal | Observed | Verdict | Severity |
|---|---|---|---|---|---|---|
| FUT-01 | propose -> proposals -> assemble -> run --candidate -> publish --candidate lifecycle | `(no command)` |  | command/option absent at branch d884e29: propose, proposals, assemble (the new commands do not exist in this worktree; scenario is declared and will be enabled when the binary provides it) | **NOT_RUN** | P1 |
| FUT-02 | `run --candidate` verifies the assembled candidate, not the index | `(no command)` |  | command/option absent at branch d884e29: --candidate (the new commands do not exist in this worktree; scenario is declared and will be enabled when the binary provides it) | **NOT_RUN** | P1 |
| FUT-03 | `publish --candidate` publishes the approved candidate only | `(no command)` |  | command/option absent at branch d884e29: --candidate (the new commands do not exist in this worktree; scenario is declared and will be enabled when the binary provides it) | **NOT_RUN** | P1 |
| FUT-04 | `--dry-run` has no side effect | `(no command)` |  | command/option absent at branch d884e29: --dry-run (the new commands do not exist in this worktree; scenario is declared and will be enabled when the binary provides it) | **NOT_RUN** | P1 |
| FUT-05 | `approve` records a human approval bound to a candidate | `(no command)` |  | command/option absent at branch d884e29: approve (the new commands do not exist in this worktree; scenario is declared and will be enabled when the binary provides it) | **NOT_RUN** | P1 |
| FUT-06 | `proposals` output is machine-readable and stable | `(no command)` |  | command/option absent at branch d884e29: proposals (the new commands do not exist in this worktree; scenario is declared and will be enabled when the binary provides it) | **NOT_RUN** | P2 |
| FUT-07 | `assemble` is deterministic (same inputs, same tree) | `(no command)` |  | command/option absent at branch d884e29: assemble (the new commands do not exist in this worktree; scenario is declared and will be enabled when the binary provides it) | **NOT_RUN** | P2 |

## 4. Findings — every REPRODUCED defect with a minimal repro

All fixtures are disposable temp repositories. `$FLOW` is the binary above; `python` is the obligation interpreter. Nothing here touches a real project.

### P0-1 — `FIL-09`: `flow publish` follows `.flow/journal.jsonl` outside the repository

**Severity P0. Invariant broken:** a user file outside the repository changes bytes (journal appended); the tool writes outside its workspace.

```powershell
$r = Join-Path $env:TEMP ("qa-p0-" + [guid]::NewGuid()); New-Item -ItemType Directory $r | Out-Null
cd $r; git init -q -b main; "v1" > data.txt; git add -A; git -c user.name=q -c user.email=q@x commit -qm init
& $FLOW init; '{"schema":"flow-config/1","policy_version":"FLOW-POLICY-1","execution_boundary":"local-trusted","obligations":[{"id":"g","program":"python","args":["-c","import sys;sys.exit(0)"],"required":true,"timeout_s":30}],"disabled":false}' | Set-Content .flow\config.json
"feature" > feature.txt; git add feature.txt          # a tree that is not already published
"USER CONTENT" | Set-Content (Join-Path $r "victim.txt")
Remove-Item .flow\journal.jsonl
python -c "import os; os.symlink(r'$r\victim.txt', r'.flow\journal.jsonl')"
& $FLOW run; & $FLOW publish                          # exit 0 PUBLISHED
Get-Content (Join-Path $r "victim.txt")               # 514 bytes of journal JSON appended
```

Observed: `PUBLISHED`, `victim.txt` 14 → 528 bytes (sha256 before/after differ). Root cause: `state::append_journal` opens `.flow/journal.jsonl` with `append` without canonicalising it or requiring a regular file inside the workspace. A **relative** symlink target fails on Windows with `ERROR` (os error 123) because of `\\?\` path semantics, but an **absolute** target succeeds. That asymmetry is incidental, not a defence.

### P0-2 — `FIL-10`: `flow run` follows `.flow/receipts` outside the repository

**Severity P0. Invariant broken:** receipt written outside the workspace.

```powershell
# same fixture as P0-1, then:
Remove-Item -Recurse .flow\receipts; $out = Join-Path $r "outside"; New-Item -ItemType Directory $out | Out-Null
python -c "import os; os.symlink(r'$out', r'.flow\receipts')"
& $FLOW run                                           # exit 0 PASSED
Get-ChildItem $out                                    # <tree>.json receipt written OUTSIDE the repo
```

Root cause: `state::save_receipt` calls `create_dir_all(parent)` then `write_atomic` into the resolved directory; no containment check, no regular-file check.

### P0-3 — `FIL-12`: a cloned candidate ships `.flow/receipts` as an absolute symlink

**Severity P0. Invariant broken:** a repository the operator only cloned makes the first `flow run` write outside the clone. This is the supply-chain form of P0-2 and needs no local prank.

```powershell
$src = Join-Path $env:TEMP ("qa-cand-" + [guid]::NewGuid()); New-Item -ItemType Directory $src | Out-Null
cd $src; git init -q -b main; "v1" > data.txt; git add -A; git -c user.name=q -c user.email=q@x commit -qm init
New-Item -ItemType Directory .flow | Out-Null
'{"schema":"flow-config/1","policy_version":"FLOW-POLICY-1","execution_boundary":"local-trusted","obligations":[{"id":"g","program":"python","args":["-c","import sys;sys.exit(0)"],"required":true,"timeout_s":30}],"disabled":false}' | Set-Content .flow\config.json
"!config.json`n!receipts" | Set-Content .flow\.gitignore
$out = Join-Path $env:TEMP ("qa-out-" + [guid]::NewGuid()); New-Item -ItemType Directory $out | Out-Null
python -c "import os; os.symlink(r'$out', r'.flow\receipts')"
git add -f .flow\config.json .flow\.gitignore .flow\receipts; git -c user.name=q -c user.email=q@x commit -qm cand
cd (Join-Path $env:TEMP ("qa-clone-" + [guid]::NewGuid())); git clone -q -c core.symlinks=true $src .
& $FLOW run                                           # exit 0 PASSED
Get-ChildItem $out                                    # receipt exists outside the clone
```

Reproduced on this Windows host with `-c core.symlinks=true` (Python can create symlinks here; Git can materialise them when asked). On Linux/macOS this is the **default** clone behaviour. `flow init` is not needed because the candidate ships the config.

### P0-4 — `FIL-13`: a cloned candidate ships `.flow/journal.jsonl` as a symlink; `flow publish` appends outside

**Severity P0.** Same fixture shape as P0-3 with `.flow/journal.jsonl` pointing at an absolute outside file; then stage any change in the clone, `flow run`, `flow publish` → the outside file gains 514 bytes of journal records. This is the supply-chain form of P0-1 and is the worse of the two (it appends to an **existing** file, i.e. corrupts it).

### P1-1 — `FIL-08`: obligations see checkout-converted bytes, not the verified tree's blobs

**Severity P1. Invariant weakened:** a `PASSED` receipt says the tree was checked; the obligation actually tested a converted projection of it.

```powershell
cd $r; "line1`nline2" | Set-Content -NoNewline payload.txt
"*.txt text eol=crlf" | Set-Content .gitattributes
git add -A; git -c user.name=q -c user.email=q@x commit -qm eol
& $FLOW init
# obligation passes iff payload.txt is CRLF
'{"schema":"flow-config/1","policy_version":"FLOW-POLICY-1","execution_boundary":"local-trusted","obligations":[{"id":"crlf","program":"python","args":["-c","import sys;d=open(\"payload.txt\",\"rb\").read();sys.exit(0 if d==b\"line1\\r\\nline2\\r\\n\" else 9)"],"required":true,"timeout_s":30}],"disabled":false}' | Set-Content .flow\config.json
& $FLOW run                                           # exit 0 PASSED
git cat-file -p HEAD:payload.txt                      # blob is LF, never tested
```

`verify_checkout_fidelity` checks presence/type but not bytes; `.gitattributes` (committed, so shipped by a candidate) drives the conversion. `doctor` warns only when `.gitattributes` is present, and its static `non_guarantees` list does not mention checkout byte conversion at all (`PRF-05` records this). Fix direction (not applied): compare the checkout's `git hash-object --no-filters` output with `ls-tree`, or refuse files whose attributes disable fidelity.

### P1-2 — `FIL-15`: a cloned candidate's `.flow/config.json` executes an arbitrary program on `flow run`

**Severity P1.** Documented residual (§8 of the hardening report: "a repository can ship `.flow/config.json` and have it used without `flow init`"), but the operational consequence is not surfaced at run time and contradicts the workflow of verifying an untrusted candidate.

Repro: commit `.flow/config.json` with an obligation whose program writes a marker outside the repo (and a `.flow/.gitignore` that un-ignores it); `git clone`; `flow run` → status `PASSED`, marker written. `doctor` names the execution boundary but not that the policy came from the repository, and `run` prints no provenance line.

### P1-3 — `GIT-10`: a symbolic ref under `refs/heads/` moves a tag

**Severity P1. Invariant broken:** "publish to a branch only; a tag is the wrong object" is bypassed; a signed/release tag can be moved while `publish` reports a branch name.

```powershell
git tag v1; git symbolic-ref refs/heads/sym refs/tags/v1
& $FLOW run; & $FLOW publish --ref refs/heads/sym    # exit 0 PUBLISHED, ref: refs/heads/sym
git rev-parse refs/tags/v1                            # tag moved to the new commit
```

The same mechanism moves `refs/heads/target` (`GIT-09`, documented residual, P2); the tag case is materially worse and not distinguished in the report. Fix direction: resolve the symbolic ref, require the final target to be under `refs/heads/`, or pass `--no-deref`-style validation.

### P1-4 — `PRC-11`: unknown options are silently ignored; `publish --dry-run` really publishes

**Severity P1. Invariant broken:** a usage error must exit 2 and do nothing; safety-seeking flags must not execute the mutating action.

| Invocation | Observed | Expected |
|---|---|---|
| `flow publish --dry-run` | exit 0, `PUBLISHED`, ref moved | refuse as unknown option (exit 2) |
| `flow publish --ref` (value missing) | exit 0, `PUBLISHED` to the current branch | usage error, exit 2 |
| `flow run --obligation` (value missing) | full run executed (red required obligation made it `FAILED`, proving both ran) | usage error, exit 2 |

Cause: `main.rs` `flag_value` returns `None` for a dangling flag and the `run` parser ignores every unknown token. With PR #53 the surface now advertises `--dry-run` and `--candidate`; if the *new* parser still ignores unknown options, the same class applies, so the first check on the new binary should be `publish --dry-ru` (typo) and `publish --dry-run` side effects.

### P2-1 — `FIL-11`: logs are redirected by a junction inside `.flow/logs/<tree>`

`canonical_within(root, log_dir)` enforces the **repository** boundary, not the `.flow` boundary: a pre-existing junction `.flow/logs/<index-tree>` → `src/` makes `flow run` write `green.stdout.log` / `green.stderr.log` into the user's source tree. Requires a local link (not cloneable as a junction), hence P2. Repro: `cmd /c mklink /J .flow\logs\<tree> src`, then `flow run`, then `dir src`.

### P2-2 — `GIT-09`: a symbolic branch ref moves the target while reporting the symref name

`git symbolic-ref refs/heads/sym refs/heads/target; flow publish --ref refs/heads/sym` → `PUBLISHED` with `"ref": "refs/heads/sym"`, but `refs/heads/target` is the ref that moved. Documented residual; the commit still carries the verified tree, and `git for-each-ref` shows the symref unchanged.

### P2-3 — `PRC-10`: `FLOW_TEST_CRASH` is live in the shipped binary and exits 89

`FLOW_TEST_CRASH=after-intent flow publish` → exit **89** after the journal intent; `=after-ref` → exit 89 after the CAS. No opt-in compile flag, and 89 is outside the documented 0/1/2 contract. Impact is denial/forced recovery, not a bypass, but it is a test backdoor in production and a documented-exit-code violation.

### P2-4 — `PRF-01`: a re-sealed forged receipt publishes

`doctor` and the report say integrity is not a signature. Measured: take a `FAILED` receipt, edit `status` and the `red` obligation entry to `PASSED`, recompute `sha256(rust_json_canonical(body))` (the harness reproduces the Rust serializer and self-checks it against an unedited receipt), write it back → `flow publish` → exit 0 `PUBLISHED`. Severity P2 because it is disclosed; only signing (or binding a key) closes it. The **casual** edit is refused with `RECEIPT_INTEGRITY` (`IDN-10`).

### P2-5 — `PRF-04`: `status` and `explain` display a tampered receipt with no integrity check

Edit a `FAILED` receipt's `status` to `PASSED` without re-sealing. `flow status` reports `latest_receipt.status = "PASSED"` (exit 0), `flow explain --tree <tree>` echoes the tampered object; `flow publish` correctly refuses with `RECEIPT_INTEGRITY`. The gate holds, the human-facing report shows a false green.

### P2-6 — `PRF-05`: `doctor` exits 0 while its own report is `overall: FAILED`

With a genuinely torn journal: `journal` check `FAILED`, `overall: FAILED`, exit **0** (because `status` is `REPORT`). A CI job gating on the exit code treats a corrupt journal as success. The forge disclosure is honest; the checkout byte-conversion limit is only mentioned by the conditional `gitattributes` check, not in the static `non_guarantees`.

### P2-7 — `PRF-06`: `explain --tree` reads arbitrary JSON by path traversal or absolute path

`flow explain --tree 'C:\...\outside-secret'` and `--tree ../../../outside-secret` both return the outside file's parsed content as the `receipt` (`{"secret": "CANARY-FAKE-SECRET-001"}`). `receipt_path` joins the unvalidated argument. Read-only, same-user, but it is the PATH-08 class the project claims to validate.

### P3-1 — `CMP-01`: the usage text promises JSON for every command; `help` and a bare invocation print text

`flow help` exits 0 with usage text; `flow` with no args exits 2 with usage text. Both are documented as outputs of the tool and contradict "Every command prints machine-readable JSON on stdout". No gate impact.

### P3-2 — `PRF-02`: evidence files listed in the receipt can be deleted; publish still succeeds

Delete both `*.log` files after a green run, then `flow publish` → `PUBLISHED` and the output still reports `receipt_logs: 2`. The receipt's own content is intact (tree + policy still proven), so this is not a false green, but "evidence" is not verified to exist.

## 5. What I could not attack, and why

| Item | Status | Reason |
|---|---|---|
| `FUT-01..07` — `propose`/`proposals`/`assemble`/`run --candidate`/`publish --candidate`/`--dry-run`/`approve` | `NOT_RUN` | Commands absent from the binary under test (`d884e29`). Declared in one `FUTURE_SCENARIOS` list; each detects its command and reports the reason instead of a verdict. **Not evidence of safety.** |
| `DUR-09` — crash *before* the intent append | `NOT_RUN` | No injection point: `FLOW_TEST_CRASH` supports only `after-intent`/`after-ref`; killing between `commit-tree` and the journal append cannot be made deterministic from outside. |
| `FIL-06` — Windows-reserved name / `..` in a tree path | `NOT_RUN` | Fixture impossible: `git update-index` refuses `CON`/`AUX.txt`/`x.`/`../evil.txt` ("Invalid path"), and though `git hash-object -t tree` writes such a tree, `git read-tree` refuses to place it in an index, so no verified index tree can contain one. A cloned hostile tree is likewise materialised by Git itself first; the validation that matters (obligation ids) is covered by `FIL-07`. |
| Linux/macOS behaviour | `NOT_RUN` | Host is Windows 11 only. The two P0 clone scenarios are **more** likely on Linux (symlinks are default there); they should be replayed on Ubuntu. |
| Real race into the CAS window (external writer between re-check and `update-ref`) | partially covered | `GIT-02` covers the deterministic "external move between run and publish"; `GIT-11` covers an in-flight abort via a blocking `reference-transaction` hook (`UPDATE_REF_REFUSED`, ref kept). A truly simultaneous external `git` writer remains the documented `FENCE-07b` non-guarantee; the harness does not claim to have raced it. |
| OS sandbox / coded execution containment | not attempted | `doctor` declares no OS isolation; obligations run with the user's rights. Testing escape from a non-existent sandbox would be theatre. |
| Disk full, GC of receipt objects, pathological filesystem (AV/FUSE) | `NOT_RUN` | Out of budget/host; belongs to `RECOVERY-16`/`OS-18` on disposable images. |
| New binary `feat/flow-mvp-0.1` (PR #53) | `NOT_RUN` | Not built or executed by this reviewer: worktree is `work/mvp-qa` at `d884e29`; the Lead's checkout was explicitly out of bounds. The time-boxed handoff did not allow a rebuild. **All 17 findings above are statements about `d884e29`.** Some may be closed or changed by the 0.1 merge; the harness is ready for one command re-run. |
| Ravum source, real credentials, live projects | never touched | Disposable fixtures only, fictional canary `CANARY-FAKE-SECRET-001`. |

## 6. Doctor claim audit (measured against this harness)

| Claim | Observed | Verdict |
|---|---|---|
| receipts bind an exact tree OID and one effective policy digest | all policy-mutation scenarios refused `STALE_POLICY`; copied/edited receipts refused | **holds**, except the content-conversion caveat (`FIL-08`) |
| publication uses ref CAS with a durable intent/outcome journal | `STALE_REF` on external move, crash scenarios reconcile honestly, no double publication | **holds** |
| a required obligation that cannot run makes the result `UNSUPPORTED`, never `PASSED` | missing program → `UNSUPPORTED`, publish refused | **holds** |
| a selective run is `PARTIAL` and never publishable | `PARTIAL` written to a separate slot; `NO_RECEIPT` on publish | **holds** |
| no OS-level isolation | obligations run with user rights; no confinement claimed | **honest** |
| no hook dispatch "of its own", but Git plumbing may run `post-checkout`/`reference-transaction` | both observed (`PRC-08`, `PRC-09`); blocking hook aborts publish/run | **honest** |
| no completeness proof of the build context | `FIL-08` shows checkout-conversion is one such undeclared input | **honest, under-specified** |
| no cryptographic protection: anyone who can write receipts can forge one | `PRF-01` forges and publishes | **honest** |
| a checkout that cannot reproduce the tree is refused, not repaired | case collision and symlink-as-file refused (`UNSUPPORTED_CHECKOUT`) | **holds for types, not bytes** (`FIL-08`) |
| (absent from `doctor`) Flow does not follow `.flow` state paths | P0-1..4 | **missing disclosure / defect** |

## 7. Reproduction and artefacts

```
python experiments/mvp-qa/attacks.py --flow <binary> --out results.json [--filter AXIS|ID]
                                       [--markdown verdict-tables.md] [--keep]
```
* Exit 0 = no reproduced P0; 1 = at least one reproduced P0; 3 = harness error (never used as evidence).
* `--keep` (or `FLOW_QA_KEEP=1`) preserves fixtures under `%TEMP%\flow-mvp-qa-*`.
* Everything is stdlib Python 3.13; no network; no non-disposable repository; `git clean`/`reset --hard` are never used.
* Evidence: [`results.json`](../experiments/mvp-qa/results.json) (full commands, JSON results, evidence per scenario), [`verdict-tables.md`](../experiments/mvp-qa/verdict-tables.md) (the tables above, machine-generated).

## 8. Recommended next checks (no product code touched)

1. Re-run the whole harness against the `feat/flow-mvp-0.1` binary; the 93 already-implemented scenarios should keep their verdicts unless the gate changed.
2. Fix, then re-test, the P0 class: canonicalise `.flow/journal.jsonl` and `.flow/receipts` and require regular, non-link paths inside the workspace (or keep state outside the tree). The harness's `FIL-09/10/12/13` are the regression tests.
3. Add `publish --dry-ru` (typo) and `publish --dry-run` to the new surface's tests; unknown options should exit 2 before any effect.
4. Extend `doctor`'s `non_guarantees` with checkout byte conversion and the `FLOW_TEST_CRASH` switch (or compile it out).
5. Make `status`/`explain` verify `integrity` before displaying a receipt, and make `doctor` exit nonzero when its own report is `FAILED`.
