# S9 benchmark protocol — FROZEN 2026-11-15, CAMPAIGN NOT RUN
**Status: `PROTOCOL FROZEN` by the human decision `H4` (2026-11-15); the campaign itself is `NOT RUN`,
so no productivity number is claimed.** This file is the **single naming authority** for S9 arms
(`X-06` resolved here). The frozen protocol is in [Protocol v1.0](#protocol-v10--frozen-2026-11-15).

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

## Protocol v1.0 — frozen 2026-11-15

Frozen by the owner's `H4` answer before any confirmatory run. Changing any field below after results
are seen invalidates the campaign and requires a new version.

| Field | Frozen value |
|---|---|
| Arms | **G** = Flow managed (this CLI: `propose`/`assemble`/`run`/`publish`) vs **E** = instrumented O0 (plain Git + the same obligations run by CI/hooks + the same evidence discipline, no Flow). No other arm in the confirmatory campaign; F stays `NOT_AVAILABLE` unless installed. |
| Design | **Paired**: each repetition runs both arms on the same fixture, the same task and the same machine, with alternating arm order (G first on odd repetitions, E first on even) to cancel warm-cache drift. |
| Repetitions | **10 pairs**. Not 3, not 5; the sign test has a minimum of 10. |
| Metric | `T_verified`: elapsed wall-clock from task start until the exact final tree passes every required check and the publish (G) or the equivalent gate (E). Paired differences and ratios are reported with their pairs. |
| Success criterion | **≥ 30 % less wall-clock for G than E** (paired ratio ≤ 0.70) with **p < 0.05**. |
| Statistics | One-sided paired sign test over the 10 ratios plus the paired median ratio; report every pair. No marginal confidence interval is required to be non-overlapping. |
| Resources | **2 CPU cores, 8 GB RAM** per run, no other heavy Cargo campaign concurrently (same host, idle otherwise). Peak RAM and disk footprint recorded. |
| Workloads | The pinned fixtures of the workload list below; each run records the fixture SHA, the task text and the exact obligation set. At least: disjoint files, same file/hunk, stale base, failing E2E, interrupted process, dirty/untracked user files. |
| Stop rules | Immediate `NO-GO` on silent data loss, a tree marked passed without its obligations, a secret leak, or non-recoverable state. Campaign stops early if G is **> 2× slower** on the first 3 pairs or if the variance cannot be explained; the reason is recorded. |
| Environment | Windows 11 reference host (16 logical CPUs, 16 GB) and Linux (CI runner or WSL) recorded separately; OS, `rustc`, `git`, `node`/`python` versions pinned per run. No scoring change after results. |
| Result handling | A run that does not meet the success criterion is reported as such; the exploratory `n=5` pilot stays exploratory and is never merged into the confirmatory sample. |

**Campaign state: NOT RUN.** Until it runs, productivity remains `UNPROVEN`; `D3` closes only when the
campaign is executed and its artifact is stored under `experiments/quality/`.

## Gate
`D3` closes only when the frozen protocol above is executed and its artifact is stored. An exploratory
pilot may measure noise and orders of magnitude; it does not close `D3`.

