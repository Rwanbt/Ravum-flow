# S9 exploratory pre-registration (pilot)

**Status: EXPLORATORY PILOT — not a confirmatory campaign. `H4` numbers are NOT set.**

This file is written **before** the pilot results are inspected. Any change after seeing results must be recorded as a dated amendment below, and invalidates the pilot as pre-registered.

## Question
Does a managed workflow reduce the time to a **verified deliverable** compared with plain Git practice, on a small synthetic workload, by a margin that would justify building a new engine?

## Arms (canonical letters, see `docs/BENCHMARK_S9.md`)
| Arm | Mechanism |
|---|---|
| A | private full copies (`git clone --no-hardlinks`) + merge |
| B | real `git worktree add/remove`, serialized merge |
| C | one shared directory + exclusive writer lock |
| D | strictly sequential, single working copy |
| E | instrumented O0: worktrees + per-agent receipt + exact-tree gate |
| F | existing tool (jj/GitButler/Scion) — `NOT_AVAILABLE` unless installed and versioned |
| G | Flow prototype — **must not be simulated**; not run in this pilot |

## Workload
Rust crate `s9_workload` with three modules owned by three agents (`alpha`, `beta`, `gamma`), each agent edits only its own module (no textual conflict), plus an integration test. `cargo test --offline -q` is the required check. The fixture commit SHA is recorded in the result file.

Agent "thinking time" is a constant (0 ms by default): this measures **workflow mechanics**, not LLM latency. That choice is identical for every arm and is stated in the results.

## Primary metric
`T_verified` = wall time from arm setup start until the **final tree on the target branch** passes the required check, excluding post-verification cleanup. Components are recorded separately (`setup`, `work`, `verify`, `integrate`, `final_verify`).

## Design
- Paired by run index: each run applies the same fixture, same commit, same 3-agent task, same random arm order (seeded).
- Calibration runs are discarded and reported as such.
- **n = 5 paired instances per arm** (pilot budget), no p95, no confirmatory claim.
- The pilot may be extended to n ≤ 8 **only** if the human grants a numeric `H4` budget. Otherwise it stops here and one consolidated `H4` decision is requested.

## Analysis plan (fixed before results)
- Per arm: n, median, min, max, mean of `T_verified`.
- Per pair: median of **paired differences** and **paired ratios**, count of runs where the first arm is slower, exact one-sided sign test p.
- No ranking claim if n < 8 or if the sign test does not reach p < 0.05, or if ranges overlap.
- Correctness gate: all module edits present in the final tree, final `cargo test` exit 0, zero conflicts. A run with a lost edit is a **failure**, never a fast win.
- Any arm failing the correctness gate is reported as `FAILED` and excluded from timing comparisons, with the failure explained.

## Stop rules
Immediate stop and report if: a lost/overwritten edit, a published tree different from the verified tree, a false green required check, a surviving descendant process, or the 16 GB ceiling being approached.

## What would count as evidence for a Flow engine
A reproducible O0 pain point that O0 cannot address by configuration (e.g. an exact-tree receipt that O0 cannot express, or a publication race O0 cannot fence), **plus** a measured `T_verified` disadvantage of O0 that a minimal engine removes. A speed advantage alone, on a synthetic workload, is not enough.

## Amendments
*(none)*
