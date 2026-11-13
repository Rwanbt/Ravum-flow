# S9 benchmark protocol — DRAFT, NOT RUN
**Status: `DRAFT`.** No S9 campaign has been executed in this repository. This file is the **single naming authority** for S9 arms (`X-06` resolved here); the protocol itself is still `DRAFT` and blocked on `H4`.

## Canonical arms (A–G)
| Arm | Mechanism | Notes |
|---|---|---|
| **A** | private full copies of the repository + Git integration/merge | Baseline used by S8; no shared Git directory |
| **B** | real `git worktree add` / `remove`, serialized integration | Creation and removal cost measured separately; shares refs and Git directory with the source repo |
| **C** | one shared directory + optimistic guard | Only one writer may hold the directory; guard is advisory against unmanaged tools |
| **D** | sequential execution, one agent at a time | Reference lower bound on parallelism cost |
| **E** | instrumented O0: native Git + native hooks/CI + light coordination + receipts | "Build nothing" candidate, measured with the same harness |
| **F** | existing tool (jj / GitButler / Scion or equivalent) | `NOT_AVAILABLE` if not installed; never simulated |
| **G** | Flow managed/hybrid | **Must not be simulated.** Measured only once a Flow prototype exists and has passed its own gates |

**Rule:** an arm letter means exactly the row above, in every document and artifact. A run using a different mechanism must publish its local labels in the alias table below; it may not reuse A–G silently.

## Alias table — historical letters mapped to canonical arms
| Old label | Where it appeared | Canonical |
|---|---|---|
| `S8-A` (private copies) | S8 artifact `evidence/mvp0c-s8-live-summary-2026-11-09.json`, prior text of this file | **A** |
| `S8-B` (shared directory + optimistic guard) | S8 artifact, prior text of this file | **C** — *not* B; S8 never used `git worktree` |
| `S8-C` (sequential) | S8 artifact, prior text of this file | **D** |
| `V6 A/B/C` | V6 archived protocol | A / B / C |
| `V6 D` = "Git classique" | V6 archived protocol | **E** |
| `V6 E` = "Flow" | V6 archived protocol | **G** |
| `V6 F` = "outil existant" | V6 archived protocol | F |
| `V6 G` = "séquentiel" | V6 archived protocol | **D** |
| old `E` = jj/GitButler/Scion | this file before `X-06` | **F** |
| old `F` = minimal orchestration O0 | this file before `X-06` | **E** |

## S8 — what the historical medians do and do not establish
S8 (`n = 3` per arm, arms `S8-A/S8-B/S8-C` → canonical A/C/D) produced elapsed totals of 100 664 / 111 110 / 208 694 ms (A), 126 224 / 237 674 / 286 275 ms (C) and 269 594 / 528 481 / 611 003 ms (D).

- These are **medians of three runs per arm**, not a ranking: the A and C ranges **overlap** (100.7–208.7 s vs 126.2–286.3 s), so no ordering between copies and shared directory is established.
- `+37 %` (hub summary) and `+193 %` (review) do **not** describe the same comparison: the first is a ratio of marginal medians, the second a median of **paired** A→D ratios (×3.93). A paired ratio must always cite its exact pairs, never an aggregate alone.
- The sign test 3/3 gives **p = 0.125 one-sided**; it is not a bilateral result and does not reach any conventional threshold.
- `total_ms` includes preparation and orchestration, with a 2 %–82.5 % residual attributable to LLM orchestration: S8 measures mostly orchestration, not VCS mechanics.
- **No `git worktree` was measured.** The product premise (arm B) has never been measured by S8.

Do not cite S8 as a hierarchy, as evidence for Flow, or as a paired effect without its pairs. S8 is `MEASURED_HISTORICAL`, fixture-specific, and superseded for decision purposes by an actual R0-c pilot.

## Hypothesis and primary outcome
Can a safe local-first workflow materially reduce **time to verified deliverable** (`T_verified`) vs O0 while remaining within CPU/RAM and human supervision budgets? Primary metric: elapsed time from task assignment until the exact final tree passes every required check and the publish gate, including retries, merge, intervention, E2E and selected CI. Report **paired differences/ratios** with their pairs and uncertainty; do **not** require marginal confidence intervals to be non-overlapping.

## Workloads
Disjoint files, same file/hunk, same symbol, API break impacting an unchanged downstream consumer, renamed/deleted files, generated files, binary/mode/symlink/case, deliberately failing E2E, stale build, dirty/untracked user modifications, interrupted process, hook side effects. Rust + TS/Python where toolchains are available. Each workload is a fixture with a pinned SHA.

## Control
Freeze fixture SHA, OS/toolchain/adapter version, task definition, model/provider and parameters, warm/cold cache regime, order/seeds, repetition count, per-strategy human supervision and LLM tokens. No scoring change after results are seen. Never execute on the Ravum live workspace.

## Metrics
`T_verified`, paired median difference and ratio, p95 only with adequate samples, CPU, peak **aggregate** RAM, disk IO and footprint, measured tokens, conflicts/attempts, E2E completeness, human supervision elapsed time, functional correctness, security stop gates.

## Elimination and stopping rules
**Immediate NO-GO** for silent data loss, unpublished/untested tree marked passed, required E2E omitted/masked, secret leak, or non-recoverable transactional state. **O0 stop:** do not create new VCS machinery if O0 matches or beats the candidate on total verified outcome and maintenance cost. Any numeric effect threshold, experiment budget, confidence protocol, sample size and stopping rule are **H4 HUMAN DECISION**; placeholder percentages from prior reviews are not ratified.

## Gate
`D3` closes only when a human signs off a versioned protocol, effect threshold, sample sizes, resource budget and stop rules **before** a confirmatory run. An exploratory pilot may measure noise and orders of magnitude; it does not close `D3`.
