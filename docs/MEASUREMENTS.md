# Measurements — the single source of the numbers

**Rule.** Every other document cites this table instead of restating a count. A number that is not
here is not a claim; a number that is here names its head, its platform and its evidence. Historical
reports keep their own dated numbers on purpose (a record that follows the code is not a record).

**Head of the merged line:** `447b0c6` (2026-11-13), plus the documentation commits on top.
**Toolchain:** `rustc 1.95.0`, pinned by `rust-toolchain.toml`; crate `flow 0.2.0-rc.1`, zero runtime
dependencies.

## 1. Tests executed

| Platform | Executed | Failed | Ignored | Evidence |
|---|---|---|---|---|
| Windows x86_64 (MSVC) | **482** | 0 | 0 | CI run `43484905763`, job `test windows-latest` (`executed_passed=482`) |
| Linux x86_64 (GNU) | **479** | 0 | 0 | CI run `43484905763`, job `test ubuntu-latest` (`executed_passed=479`) |
| macOS arm64 | **479** | 0 | 0 | CI run `43484905763`, job `test macos-latest` (`executed_passed=479`; the job then failed on the *packaging* step, which macOS does not have yet) |
| Windows, local | 482 | 0 | 0 | `cargo test --locked --offline --all-targets` on this host |

Differences are platform gates, not skips: one Windows-only `PATH`/`PATHEXT` gate, and the
non-UTF-8 checkout gate that skips on APFS because the filesystem refuses such a name.
**144 named gates** are required by name in CI; each one was verified to resolve to a real
`fn <name>` in `crates/flow`.

Reproduce: `cargo test --locked --offline --all-targets` (the CI floors are 482 / 479 / 479 and
any `ignored` count fails the build).

## 2. Coverage (product-only, branch counters)

| Platform | Gate | Result | Evidence |
|---|---|---|---|
| Linux (CI runner) | lines ≥ 95 %, branches ≥ 90 % | **green, no waiver** | CI job `coverage (product-only gate)`, run `43484905763` |

The measurement is `cargo-llvm-cov --all-targets --branch` with
`--ignore-filename-regex '[\/]tests[\/]'`, then `analyze_lcov.py --product-only`. Test code never
enters the product view. The last quoted figures on an earlier head were **95.91 % lines /
93.68 % branches** (`00f87b7`, see [wave 9](reviews/FLOW-QUALITY-WAVE9.md)); the gate — not the
decimal — is what CI enforces.

## 3. Mutation

| Scope | Result | Evidence |
|---|---|---|
| Module subset, hand-measured (Windows/Linux) | `state` 140/140 · `commands` 96.7 % · `gitcli` 88.6 % · `main` 94.2 % (97.1 % adjudicated) · aggregate **95.4 %** | [wave 9 §3.5](reviews/FLOW-QUALITY-WAVE9.md) |
| Per module on Linux, in CI | **armed, first run pending** | `.github/workflows/flow-mutation.yml` (scheduled, report mode) |

Scope is `--bins` (the unit suite): every rate above is a **lower bound**, never a crate-wide
score. Mutation is measured, not yet enforcing a threshold — the threshold has to come from the
first full matrix.

## 4. Real-project pilot (2026-11-13)

Windows 11, 16 logical CPUs, release binary built from `447b0c6`. Raw results:
[`experiments/pilot-2026-11-13/results.json`](../experiments/pilot-2026-11-13/results.json).

| Scenario | Input | Result | Time |
|---|---|---|---|
| A — this repository | 2 705 tracked files, real Rust workspace | `run` **PASSED**, `publish` **PUBLISHED** | `run` 23.3 s (2 obligations: `cargo metadata`, `cargo fmt --check`), `publish` 0.55 s |
| A — one real, slow obligation | the same tree, `cargo test --bin flow` + `cargo metadata` | `run` **PASSED** | 18.8 s |
| A — two agents, disjoint files | two proposals from one base, assembled | candidate → `run` **PASSED** → `publish` **PUBLISHED** | propose 0.89 s / 0.32 s, assemble 0.74 s, candidate run 12.5 s |
| A — two agents, same path | two proposals creating the same file | **REFUSED**, `PATH_OPERATION_CONFLICT`, exit 1 | — |
| B — above the old envelope | **25 000 files**, one no-op obligation | `run` **PASSED**, `publish` **PUBLISHED** | `run` 185.7 s (≈ 7.4 ms/file) |

Scenario B is the point: the same operation was a **refusal** before this branch (`20 000`-file
budget). Scenario A is the collaboration claim measured, not asserted: two attributed proposals on
one immutable base produce one verified tree, and a real collision is refused by name.

## 5. Performance (descriptive, not a benchmark)

| Command | Measured | Gate |
|---|---|---|
| `flow version` | < 1 s | CI fails above 2 s (Linux) |
| `flow doctor` | < 5 s | CI fails above 5 s (Linux) |

The CI budgets are regression tripwires, not performance claims. Productivity against other
workflows stays `UNPROVEN` until `H4` fixes the S9 thresholds ([BENCHMARK_S9.md](BENCHMARK_S9.md)).

## 6. Security and supply chain

| Item | State |
|---|---|
| Runtime dependencies | none (`Cargo.lock` = 1 package) |
| `unsafe`, `TODO`/`FIXME`, `#[ignore]` in product code | none (grep-verified) |
| GitHub Actions | pinned to 40-hex commits; `permissions: contents: read` |
| GitHub secret scanning / push protection | enabled |
| Branch protection / required checks | **absent** ([verdict](reviews/FLOW-PRODUCTION-VERDICT-2026-11-13.md) §3) |
| Code signing | **none** (`"unsigned": true`) |

## 7. Maintenance of this table

Update it when a head changes the numbers, and cite the CI run id. Do not copy a number from this
table into prose without its platform and its evidence: that is how four contradictory test counts
appeared three times (R2-06, waves 5 and 8, then again on 2026-11-13).
