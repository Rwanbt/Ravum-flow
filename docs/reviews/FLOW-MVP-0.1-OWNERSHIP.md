# FLOW MVP 0.1 — ownership matrix (published before work starts)

**Date:** 2026-11-09 (Europe/Paris) · **Integration branch:** `feat/flow-mvp-0.1`
**Baseline:** `fix/flow-foundation-proof-gates` @ `d884e29c825cad40678c56702361f4e403a769f7`
**Working root (Lead only):** `D:\App\Ravum-flow`
**Teammate worktrees:** `D:\App\Ravum-flow-wt\{safety,polyglot,qa}` (ephemeral, disposable)
**Nothing is ever written to `Rwanbt/Ravum`.**

## 1. Rules

1. One writer per path. A path not listed for an agent is read-only for that agent.
2. Teammates work **only** in their own worktree on their own branch and commit there.
   The Lead integrates by cherry-pick / merge, then re-runs every gate.
3. Shared files (`main.rs`, `commands.rs`, `state.rs`, `Cargo.toml`, CI, README,
   `docs/**` except the two files assigned below) have exactly one owner: the Lead.
4. Every worker reports: base SHA, files touched, commands actually run,
   PASS/FAIL/SKIP/NOT_RUN per command, risks, result SHA. No report, no merge.
5. A red or skipped test is never reported green. `UNSUPPORTED` is a refusal, not a pass.
6. No `git reset --hard`, `git clean -fd`, `git checkout -f`, no force push, no
   branch-protection change, no release.

## 2. Path → owner → work

| Path | Owner | Planned work |
|---|---|---|
| `crates/flow/src/main.rs` | Lead | dispatch for `propose`, `proposals`, `assemble`, `run --candidate`, `publish --candidate`, `--dry-run`, `explain --candidate`, `init --adopt-policy`, `approve` |
| `crates/flow/src/commands.rs` | Lead | candidate run/publish path, symref refusal, policy-adoption gate, dry-run plan, explain |
| `crates/flow/src/state.rs` | Lead | config/adoption record, proposal+candidate paths, schema consts (lock moved out) |
| `crates/flow/src/proposal.rs` | Lead | proposal admission, immutability, idempotency, temp-index tree build, assembly, candidate record |
| `crates/flow/tests/cli.rs` | Lead | migrate existing suite to the adoption gate, keep every P0 gate name |
| `crates/flow/tests/proposals.rs` | Lead | M1–M6 end-to-end scenarios through the real CLI |
| `Cargo.toml` (workspace) | Lead | exclude polyglot fixtures from the workspace |
| `.github/workflows/flow-ci.yml` | Lead | branch triggers, executed-test counters, polyglot job, E2E job |
| `README.md` | Lead | status, install, 2-agent + conflict + crash examples, limits |
| `docs/**` (except the two below) | Lead | ARCHITECTURE, DECISION_REGISTER, TRACEABILITY, SECURITY_TESTS, BACKLOG, ISSUE_MAP, ADR, verification report |
| `examples/two-agents/**` | Lead | reproducible M1/M2/M3 driver scripts |
| `crates/flow/src/lock.rs` | safety (seed stub by Lead) | v2 lock contract: owner identity, fail-closed attribution, no age-based stealing |
| `crates/flow/src/gitcli.rs` | safety | bounded Git invocation (memory + wall clock + descendants), explicit index/stdin plumbing |
| `crates/flow/tests/concurrency.rs` | safety | 2/3/8-process contention, live-old-lock, dead-PID recovery, corrupt lock |
| `examples/polyglot/**` | polyglot | Rust / Node / Python fixtures + offline demo scripts |
| `docs/MVP-USER-GUIDE.md` | polyglot | install, three-language walkthrough, refusal cases, limits |
| `docs/CLI-JSON.md` | polyglot | versioned JSON schemas, exit codes, examples |
| `experiments/mvp-qa/**` | qa | independent adversarial scripts (python) on disposable fixtures |
| `docs/reviews/MVP-QA-ADVERSARIAL-REPORT.md` | qa | attack matrix results: REPRODUCED / REFUTED / NOT_RUN |

## 3. Dependency order

```
Lead: proposal.rs + candidate CLI  ─┬─> Lead: M1–M6 E2E tests
safety: lock v2 + bounded git ─────┤
polyglot: fixtures + docs ─────────┴─> Lead: integration, CI, report
                                                 ▲
qa: adversarial report (independent, read-only on src) ─┘
```

`gitcli.rs` and `lock.rs` are self-contained modules; the Lead wires their call
sites at integration, so no two writers touch one file.

## 4. Shared-file policy conflicts already decided

- `.github/**` and `README.md`: Lead only. Teammates propose text in their report.
- `Cargo.toml`: Lead only (workspace `exclude` for the fixtures).
- Any new dependency: **refused** for the MVP (offline, licence, audit). The crate
  stays dependency-free; the fixtures use only their toolchain's built-ins.
