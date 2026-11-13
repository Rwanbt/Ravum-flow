# S9 — exploratory pilot harness

Implements the arms of [docs/BENCHMARK_S9.md](../../docs/BENCHMARK_S9.md) and the pre-registration in [PREREGISTRATION.md](PREREGISTRATION.md).

```bash
python experiments/s9/run_s9.py --calibration 2 --runs 5 --out out/pilot.json
```

Options: `--arms ABCDE`, `--agents 3`, `--think-ms 0`, `--seed 20261109`.

## What it does
1. Builds a synthetic Rust crate (3 modules + integration test) under `work/base` and records its commit SHA.
2. For each run and each arm (randomized order, seeded), resets the base to that exact SHA, then executes the arm protocol end to end.
3. Measures `T_verified` (setup → work → verify → integrate → final verify), plus each phase separately.
4. Verifies the outcome: every agent edit present in the final tree, final check exit 0, zero conflicts.
5. Writes paired statistics: per-arm median/min/max, paired differences and ratios, one-sided sign test.

## Scope and honesty
- Measures **workflow mechanics only**: agent thinking time is a constant (default 0 ms). LLM latency and human supervision are out of scope and must not be inferred.
- Arms run **sequentially**; CPU/RAM contention between truly concurrent agents is not measured here (that is `RESOURCE-14`'s job).
- Arms F (jj/GitButler/Scion) and G (Flow) are reported `NOT_AVAILABLE` / not run. A study without G never shows that G wins.
- `n = 5` is exploratory. No p95, no confirmatory claim, no ranking when ranges overlap or p ≥ 0.05.
- A lost edit or a red final check makes the run `FAILED`; failures are never dropped silently from the report.

## Files
- `out/pilot.json` — full records, environment, tools, summary, limitations.
- `PREREGISTRATION.md` — frozen protocol and stop rules.
