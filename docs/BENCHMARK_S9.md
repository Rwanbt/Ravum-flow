# S9 benchmark protocol — DRAFT, NOT RUN
S8 real historical **27 agents / 9 campaigns / 3 repetitions each**: A private copies + integration median ~111.110 s; B shared directory + optimistic guard ~237.674 s; C sequential ~528.481 s. These are fixture-specific. A was **not** a Git worktree; B was **not** transactional Flow. Earlier scripted micro-pilots measured different workloads and must never be pooled with S8.

## Hypothesis and primary outcome
Can a safe local-first workflow materially reduce **time to verified deliverable** vs O0 while remaining within CPU/RAM and human supervision budgets? Primary metric: elapsed time from task assignment until the correct candidate/merge tree passes all required checks and publish gate, including retries/merge, intervention, E2E and selected CI. Report paired differences and confidence bounds; do **not** require marginal confidence intervals to be non-overlapping.

## Comparators (availability must be recorded)
A: private copies + merge; B: real Git worktrees + serialized integration; C: shared disk + optimistic guard; D: sequential; E: existing jj/GitButler/Scion as available; F: minimal orchestration O0; G: prospective Flow managed/hybrid when independently implemented and authorized. `NOT_AVAILABLE` is not a failure; do not construct fake results.

## Workloads
Disjoint files, same file/hunk, same symbol, API break impacting unchanged downstream consumer, renamed/deleted files, generated files, binary/mode/symlink/case, deliberately failing E2E, stale build, dirty/untracked user modifications, interrupted process, hook side effects. Rust + TS/Python in scope where toolchains available.

## Control
Freeze fixture SHA, OS/toolchain/adapter version, task definition, model/provider and parameters, warm/cold cache regime, order/seeds, repetition count, per-strategy human supervision and LLM tokens. No scoring changes after results are seen. Do not execute on Ravum live workspace.

## Metrics
Median paired time, p95 (only with adequate samples), CPU, peak **aggregate** RAM, disk IO and footprint, tokens measured, number of conflicts/attempts, E2E completeness, elapsed human supervision, functional correctness, security stop gates.

## Elimination and stopping rules
**Immediate NO-GO** for silent data loss, unpublished/untested tree marked passed, required E2E omitted/masked, secrets leaked or non-recoverable transactional state. O0 stop: recommend not creating new VCS machinery if it matches or beats candidate across total verified outcome and maintenance cost. S9 numerical effect threshold, experiment budget, significance/confidence protocol, sample size and stopping rule are **H4 HUMAN DECISION**; placeholder percentages from prior reviews are not ratified.

## Gate
`D3` closes only when a human signs off versioned protocol, effect threshold, sample sizes, resource budget and stop rules **before** executing R0.