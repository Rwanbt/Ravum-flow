# Flow foundation — verified report

**Date:** 2026-11-09 (Europe/Paris) · **Branch:** `feat/flow-verified-foundation` · **PR:** #51 (draft, base `dev`)
**Verdict:** **`FOUNDATION_PARTIAL`** — a useful core, tested on Windows, with every unqualified capability refused safely. This is **not** production-ready and **not** a proof of general safety.

> **Correction applied afterwards (2026-11-09).** The verification core described in this report contained three blocking defects that this report did not catch: a selective run was recorded as a complete `PASSED` proof, a receipt stayed valid after the verification policy changed, and an unreadable journal was reported as a successful reconciliation. They were reproduced with failing tests on `832f499` and closed on `fix/flow-foundation-proof-gates` (PR #52). See [FLOW-FOUNDATION-P0-HARDENING-REPORT.md](FLOW-FOUNDATION-P0-HARDENING-REPORT.md).
>
> The evidence below is left exactly as it was recorded. It describes the state **before** the correction and must not be read as a statement about the corrected gate.

---

## 1. Baseline, modules and commits

| Item | Value |
|---|---|
| Start of work | `f113791` (review dossier) whose parent is `dev@1d5fddf` |
| `dev` merged in (licence decision) | `2c8cb62` — MPL-2.0 adopted by the owner (PR #50) |
| Implementation commit | `4912772` — CLI, R0-a harness, R0-b evidence, S9 pilot, CI |
| Merge commit | `7f05439` |
| Licence alignment | `69030b4` |
| Modules added | `crates/flow` (2,247 lines source, 401 lines integration tests), `experiments/r0a` (fixtures + control), `experiments/r0b` (reproduction harnesses), `experiments/s9` (pilot), `.github/workflows/flow-ci.yml` |
| Ravum | never opened for write, never built, never modified; `Rwanbt/Ravum` untouched |
| PR #33 | unchanged, still draft |

No ADR/RFC was ratified. No release was published. No merge was performed.

## 2. Decision: instrumented O0 + the smallest useful core (no new VCS engine)

The R0 and S9 evidence supports building **the receipt/publish discipline only**, not a scheduler, VFS, CRDT or merge engine.

Reasoning, in the order it was established:

1. **R0 shows the real problem is verification and publication, not concurrency.** 25/25 executed scenarios reproduced: a test can pass on dirty/untracked bytes while failing on the exported tree (`TREE-01/02`); hooks mutate the user's worktree and can reach the shared Git directory (`HOOK-02b`, `HOOK-GITDIR-19`); native `git commit` includes what `pre-commit` stages, whereas the observed Ravum plumbing writes the tree **before** hooks (`HOOK-02a`); ref CAS is correct between cooperating writers and detect-only against an external `git` (`REF-10`, `FENCE-07a/b`); the only ambiguous crash window is *after the CAS* and is resolved by re-reading the ref (`FENCE-07c`, `CRASH-11`); an ignored input poisons a declared cache key (`CACHE-13`); textual applicability is not validation (`LEGACY-06`).
2. **S9 shows O0-with-receipts is not slower than worktrees.** In the exploratory pilot, arm E (worktrees + receipt + exact-tree gate) and arm B (worktrees) are statistically indistinguishable (paired median ratio 0.955, one-sided sign p = 1.0), and both are faster than full private copies (arm A: ratio A/E = 1.259, p = 0.031). A new engine therefore buys no measured time.
3. **What O0 cannot express** is a machine-checkable receipt bound to the exact tree, a refuse-by-default publication, and a crash-safe intent/outcome journal. That is exactly what `crates/flow` implements, in ~2,250 lines with **zero dependency**.

Rejected options, with the reason: O1 existing tools (jj, GitButler, Scion) are `NOT_AVAILABLE` on this machine and cannot be adopted without a licence/behaviour study (`H1` OPEN); O2/O3 full managed layer (no measured pain point justifying a scheduler or VFS); O4 (deferred by the review, no new evidence).

## 3. R0 matrix — every executed scenario, with its outcome

All on Windows 11 / NTFS / Git 2.51.1 / Python 3.13.7, disposable synthetic repositories, fictitious canaries only. Full commands, raw output and scope per test: `experiments/r0b/*/RESULTS.json` and `logs/`.

| ID | Verdict | Fixture | Key observation |
|---|---|---|---|
| `TREE-01` | REPRODUCED | scratch `tree-imports` | green in the worktree (untracked + ignored imports), red on a clean export of the tree OID |
| `TREE-02` | REPRODUCED | scratch `tree-mutation` | tree OID `9f4f0974…` unchanged while the checked bytes changed → a receipt naming only that OID would be false |
| `HOOK-02a` | REPRODUCED | scratch `hook02a` | native `git commit` **includes** the file staged by `pre-commit`; the tree is written after hooks |
| `HOOK-02b` | REPRODUCED | scratch `hook02b` | `pre-commit` rewrites a dirty user file in the user's worktree; no isolation |
| `HOOK-02c` | REPRODUCED | scratch `hook02c` | order `pre-commit → prepare-commit-msg → commit-msg → post-commit`, cwd = worktree root |
| `HOOK-02d` | REPRODUCED | scratch `hook02d` | refusal is traceable, HEAD unchanged, index intact, no orphan object |
| `HOOK-GITDIR-19` | REPRODUCED | scratch `gitdir-*` | a linked worktree writes the shared `.git/config` and moves a ref of the main repo; a private copy does not |
| `REF-10` | REPRODUCED | scratch `refcas` | three-argument CAS refuses the stale old OID; correct CAS succeeds |
| `FENCE-07a` | REPRODUCED | scratch `fence07a` | stale cooperating writer refused by CAS alone |
| `FENCE-07b` | REPRODUCED | scratch `fence07b` | an external `git commit` succeeds between read and CAS: bypass **detected, never prevented** |
| `FENCE-07c` | REPRODUCED | scratch `fence07c` | crash after CAS: ref moved, intent only → outcome classified by re-reading the ref |
| `CRASH-11` | REPRODUCED | scratch `crash11-*` | four crash points; naive replay writes two intents and two commits for one tree → recovery guard required |
| `FILE-09` | REPRODUCED | scratch `f3-filetypes/*` | `100755` and binary bytes exact in the tree, rename detected 100 %, CRLF normalised by attributes, `120000` preserved but the worktree link is not trustworthy on this host, case collision: two tree entries, one worktree file |
| `ENV-04a` | REPRODUCED | scratch `fake-home` | 72 inherited variables by default; an 11-variable allowlist removes the canary |
| `ENV-04b` | REPRODUCED | scratch `fake-home` | the fake `~/.ssh` key is fully readable as the user: no filesystem isolation |
| `ENV-04c` | REPRODUCED | scratch logs | naive logging persists the canary twice; redaction-before-persistence leaves 0 occurrence and fails closed |
| `SANDBOX-05` | REPRODUCED (loopback) | scratch `probe.py` | loopback connect allowed (`WSAECONNREFUSED`/`WSAEWOULDBLOCK`), **no** `WSAEACCES`; Internet egress `NOT_QUALIFIED` |
| `PATH-08` | REPRODUCED | scratch `paths` | `..` accepted by naive validation and writing outside the workspace; junction (`mklink /J`) created **without elevation**; `CON`/`aux.txt` created but **not indexable by Git**; `trail.`/`trail ` silently renamed; case collision overwrites |
| `RESOURCE-14` | REPRODUCED | scratch `grandchild` | killing the direct child leaves the grandchild alive; `taskkill /T /F` kills the tree |
| `UNMANAGED-17` | REPRODUCED | scratch `writer_lock` | external `git commit` succeeds while a cooperative lock is held; detection only |
| `E2E-12` | REPRODUCED | scratch `e2e` | same consumer: green rc=0, breaking rc=1; `|| true`-style gate returns 0 with a red suite |
| `CACHE-13` | REPRODUCED | `f7-context` (rebuilt) | changing the ignored `.cache` keeps `context_id` identical while the artifact changes (fixture now consumes the ignored cache natively) |
| `CONTEXT-03` | REPRODUCED | scratch | three runs, one `context_id`, three artifacts (ignored file + undeclared env): completeness is not provable from inside |
| `LEGACY-06` | REPRODUCED | scratch | patch applies and compiles (`rc=0`), tests fail (`ImportError`): textual applicability ≠ validation |
| `RECOVERY-16` | REPRODUCED | scratch `journal` | truncated journal: naive strict crashes, naive lenient would republish, robust reader refuses (`REFUSED_CORRUPT_JOURNAL`) |

**Totals:** 25 `REPRODUCED`, 0 `DISPROVED`, 0 `NOT_REPRODUCED`, 0 `BLOCKED`, 0 `NOT_RUN` within the executed set. **Outside the executed set** (`NOT_RUN`, and never reported as pass): Internet egress, AppContainer / Windows Sandbox / WDAC, Job Objects via ctypes, Linux namespaces/bwrap, ACL or another user account, a real forge merge queue (`FORGE-15`), power-loss durability, and any multi-host scenario.

**Honest provenance note:** the R0-a fixtures were cleared mid-campaign by the `control` step of the harness (a real defect). Fix applied in `experiments/r0a/harness/cli.py` (control now cleans only its own root). The three R0-b campaigns rebuilt their own copies from the same builders; their verdicts are valid for the Git/OS behaviour measured, but they are not bit-identical replays of the R0-a seeds. This is stated in each `RESULTS.json`.

## 4. Tests and CI

Blocking gates, executed locally after the `dev` merge:

| Gate | Command | Result |
|---|---|---|
| formatting | `cargo fmt --all --check` | 0 issues |
| lints | `cargo clippy --locked --offline --all-targets -- -D warnings` | 0 warning |
| unit tests | `cargo test --locked` (bin) | **12 passed** |
| integration tests | `cargo test --locked` (`tests/cli.rs`) | **16 passed**, 3.16 s |
| release build | `cargo build --locked --release` | success, binary `target/release/flow` |
| harness self-tests | `python -m unittest discover -s experiments/r0a/tests` | **9 passed**, 1.9 s |

Integration coverage (`crates/flow/tests/cli.rs`), each on a fresh disposable repository: `init` idempotence/refusal, read-only `doctor`, `run` without obligations → `UNKNOWN`, passing obligation bound to the index tree, failing obligation → `FAILED` and publish refused, publish without receipt refused, publish CAS + journal, stale `--expected-old` refused with no ref move, crash after intent → `NOT_PUBLISHED` by reconcile, crash after ref → `PUBLISHED` by reconcile, dirty/untracked files preserved byte-for-byte (SHA-256 before/after), disable blocks mutation, unsafe obligation id refused, obligation timeout → `TIMEOUT` (skips with an explicit `SKIP` line if `python` is absent).

**Deliberately red, and they are red:** `publish` without a PASSED receipt, stale CAS, unsafe path, failing obligation, timeout, and both simulated crashes (exit code 89, never 0).

**CI:** `.github/workflows/flow-ci.yml` — Ubuntu + Windows, `fmt`/`clippy`/`test`/`release`, Python harness self-tests on Linux, a test-count guard (≥25 discovered tests) and a 20-minute timeout.

Executed on GitHub Actions for `69030b4` (branch push run `42665344871`, PR run `42665384917`) — **both runs concluded `success` on both jobs**:

| Job | Result | Observed |
|---|---|---|
| `ubuntu-latest` | success (26 s) | `cargo fmt --check` OK, clippy OK, **12 unit + 16 integration passed**, release built, 9 Python harness self-tests passed, `discovered 28 tests` |
| `windows-latest` | success (60 s) | fmt OK, clippy OK, **12 unit + 16 integration passed** (5.36 s for the integration suite), release built, `discovered 28 tests` |

What that qualifies and does not: the CLI and its tests are now exercised on Linux **and** Windows. R0-b and S9 remain Windows-only; no R0/S9 measurement was performed on Linux. The workflow is bounded (20 min timeout, two OS, no service container, no network use inside tests).

## 5. Security state

| Surface | State | Evidence |
|---|---|---|
| Git hooks | **not executed** by Flow; `doctor` warns when hooks exist | `HOOK-02a/b/c/d`; deliberate MVP scope |
| Shared Git directory | verification runs on an export whose `.git` link is removed; hooks are never run there | `cmd_run`; `HOOK-GITDIR-19` shows why a worktree alone is not a boundary |
| Exact-tree identity | receipt names `tree_oid`; `publish` re-derives the index tree and refuses any other | integration tests; `TREE-01/02` |
| Build context | `BuildContextId` = declared tree + obligations + policy + git/OS; undeclared inputs ⇒ scope `UNKNOWN` | `CONTEXT-03`, `CACHE-13`; `doctor` lists the non-guarantee |
| Dirty/untracked user files | never written; preservation asserted byte-for-byte | integration test + `worktree_diff` in the receipt |
| Journal / crash | fsynced intent, CAS, outcome; `reconcile` observes the ref; never blind replay | `CRASH-11`, `FENCE-07c`; two integration tests |
| Secrets | obligation environment is an 11-name allowlist; logs redacted **before** persistence | `ENV-04a/b/c` |
| Timeout / descendants | direct child killed by default; tree kill on Windows (`taskkill /T`); POSIX direct kill only | `RESOURCE-14`; limitation documented |
| Sandbox | **none**. Untrusted code execution is `UNSUPPORTED` | `SANDBOX-05`; `doctor.execution_boundary` = WARN |
| Windows / Linux | Windows 11 tested; Linux `NOT_RUN` locally, CI-only | this report |

No P0 (data loss, secret leak, false green receipt) was observed in the product's own tests. The R0 findings are risks of the **environment** (Git hooks, filesystems, process trees), not regressions introduced here; Flow refuses or documents each of them.

## 6. S9 exploratory pilot

Pre-registered before execution (`experiments/s9/PREREGISTRATION.md`). 2 calibration runs discarded, **n = 5 paired runs per arm**, arm order randomised per run with a fixed seed, same fixture SHA, same 3-agent task, `cargo test --offline` as the required check. Agent thinking time = 0 ms by design (workflow mechanics only).

| Arm | Median `T_verified` | Min–max | Mean |
|---|---|---|---|
| A — private copies | **3.518 s** | 3.441–3.616 | 3.525 |
| B — real worktrees | **2.678 s** | 2.636–2.745 | 2.684 |
| C — shared dir + lock | **2.078 s** | 1.980–2.150 | 2.060 |
| D — sequential | **2.087 s** | 1.980–2.192 | 2.096 |
| E — O0 + receipts + exact-tree gate | **2.786 s** | 2.768–2.833 | 2.795 |

Paired comparisons (n = 5 each, exact one-sided sign test):

| Pair | Median ratio | p |
|---|---|---|
| A / B | 1.317 | 0.031 |
| A / E | 1.259 | 0.031 |
| B / E | 0.955 | 1.000 |
| C / D | 0.981 | 0.969 |
| C / E | 0.734 | 1.000 |

Reading, with the limits that matter: full private copies are the slowest arm on this workload; **worktrees save ~24 %**; adding the receipt discipline costs ~4 % and is indistinguishable from plain worktrees (p = 1.0); the serialized arms (C/D) are a lower bound, not a competitor for parallel agents. **No confirmatory claim is made**: n = 5, one machine, one workload, no LLM latency, arms executed sequentially (no CPU/RAM contention), one OS. `H4` (threshold, budget, confirmatory n) remains **OPEN**, and this pilot is not sufficient to close `D3`.

Cost: whole campaign ≈ 3 minutes of CPU on the 16 GB reference machine; peak concurrency 1 rustc at a time.

## 7. What is usable, and how to recover

```bash
cargo build --release
./target/release/flow init
./target/release/flow doctor
git add -A && ./target/release/flow run     # receipt bound to the index tree
./target/release/flow publish               # CAS publish of a PASSED receipt
./target/release/flow reconcile             # after a crash
./target/release/flow disable               # stop mutating commands
```

Refusals are explicit and machine-readable: `UNKNOWN` (no obligation), `FAILED`, `TIMEOUT`, `UNSUPPORTED`, `REFUSED`, `RECONCILED_PUBLISHED` / `RECONCILED_NOT_PUBLISHED` / `NEEDS_RECONCILIATION` recorded in the journal. `publish` moves **refs only** — it never checks out, never overwrites a dirty file, and leaves the worktree visibly divergent. Recovery is `reconcile` (observe the ref, never move it) followed by a normal `git checkout`/merge performed by the human.

## 8. Governance

| ID | State after this work |
|---|---|
| H1 (provisional design) | **OPEN** — the evidence favours instrumented O0 + thin receipt layer, but no ratification |
| H2 (licence/rights) | **HUMAN_DECIDED 2026-11-09**: MPL-2.0 (PRs #49/#50). External code reuse remains OPEN |
| H3 (untrusted sandbox) | **OPEN** — no qualified sandbox; untrusted execution refused |
| H4 (S9 threshold/budget) | **OPEN** — pilot only, no numeric threshold granted, no confirmatory run |
| H5 (R0 fixtures / minimal core) | **HUMAN_DECIDED 2026-11-09**, bounded to this branch and synthetic fixtures |
| H6 (P0 handoff to Ravum session) | **OPEN** — no handoff was sent; findings are in this repo only |
| H7 (internal-first vs product) | **OPEN** (proposed) |
| H8 (documentary stop rule) | **OPEN** (proposed) |

No `RAVUM-ADR-*` or `FLOW-ADR-*` was marked ACCEPTED. No decision above was inferred from model output.

## 9. Vault traceability

Private vault (`Rwanbt/obsidian-dev-brain`, `projects/ravum-flow/`), pushed as `29c63f3`: dossier session `operations/sessions/2026-11-09-deepseek-d0-d4-review.md`; **execution session** `operations/sessions/2026-11-09-flow-foundation-execution.md`; initiative `work/initiatives/2026-11-09-flow-foundation-mvp.md` (`FLOW-INIT-20261109found`, `delivery: implemented`, `verification: partial`); hub `INDEX.md`; resume memory `_memory/memory.md`. `vault.py check` green (lint/links/idempotence) and `BOARD.*`/`PORTFOLIO.*` regenerated by the tool, never hand-edited. **A new session note for this execution phase is recorded separately; if the vault sync failed, the status is written here rather than claiming success.** No private vault content, path or log is copied into this public repository.

## 10. Exit decision

**`FOUNDATION_PARTIAL`.**

- The core is real, useful and tested on the platform it claims: exact-tree receipts, refuse-by-default publication, CAS + journal + reconciliation, no user-file loss in the exercised paths.
- At least one advertised capability is **refused safely** rather than faked: untrusted execution (no sandbox), Git hooks (not executed), context completeness (`UNKNOWN`), Linux (CI-only).
- No P0 was observed in the product; residual risks are the documented environment limits (external `git` bypass, junction/case filesystem hazards, descendant processes on POSIX).

**Single next objective:** obtain one human `H4` decision (numeric threshold, absolute floor, budget, confirmatory `n`) and, only then, extend this pre-registered pilot to `n = 8` on Linux as well as Windows — because the remaining questions are statistical and cross-platform, not architectural.

*Everything in this report is `MEASURED` on the stated host, or explicitly labelled `NOT_RUN` / `NOT_QUALIFIED`. A green test is not a proof of general safety; a reproduced risk is not a fixed risk.*
