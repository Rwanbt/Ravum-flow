# Flow production candidate — baseline (G0)

**Date:** 2026-11-10 (Europe/Paris) · **Read-only inventory** · no state was written by this step.

## 1. Repository actually opened

| Fact | Value | How |
|---|---|---|
| Toplevel | `D:/App/Ravum-flow` | `git rev-parse --show-toplevel` |
| Remote | `origin https://github.com/Rwanbt/Ravum-flow.git` | `git remote -v` |
| Branch at G0 | `feat/flow-mvp-0.1` @ `ea181d4f112df4deef01016a041fcdfdac154264` | `git log -1` |
| Working tree | clean (`git status --porcelain` empty) | idem |
| Submodules | none (`.gitmodules` absent) | `ls` |
| Other worktrees | `D:/App/Ravum-flow-wt/{polyglot,qa,safety,prod-security,prod-quality,prod-polyglot}` | `git worktree list` |

`Rwanbt/Ravum` was **not** opened, built, branched or tested during this wave.

## 2. Remote refs and PRs (verified with `gh`, not quoted from the mandate)

| Ref | SHA | State |
|---|---|---|
| `origin/main` | `9d5170a7de0a1bac5e1a8caf74827afc43122251` | unchanged |
| `origin/dev` | `2c8cb62b4ff6e2c1403ae38fc47a3d86dcdf3b8b` | unchanged |
| PR #51 | `feat/flow-verified-foundation` @ `832f4995618a28c3e2bae25654e59651210a0faa` → `dev` | draft |
| PR #52 | `fix/flow-foundation-proof-gates` @ `d884e29c825cad40678c56702361f4e403a769f7` → `feat/flow-verified-foundation` | draft |
| PR #53 | `feat/flow-mvp-0.1` @ `ea181d4` → `fix/flow-foundation-proof-gates` | draft, MERGEABLE |
| PR #33 | `dev` → `main` | draft (documents) |

CI at `ea181d4`: runs `42698150980` (pull_request) and `42698141703` (push), both `success`,
4 jobs each (test + polyglot, Ubuntu + Windows).

## 3. Test counts reconciled (the historical numbers differ, and that is expected)

| Source | Count | Head | Meaning |
|---|---|---|---|
| P0 hardening report | 83 | `3b51052` | 21 unit + 37 CLI + 25 isolation |
| MVP 0.1 verification report | 132 | `ea181d4` | 24 unit + 21 bounds + 41 CLI + 8 concurrency + 25 isolation + 13 proposals |
| this branch, G0→G4 | 142 | `f237d57` | 24 unit + 21 bounds + 47 CLI + 8 concurrency + 25 isolation + 17 proposals |
| this branch, after the fuzz/property evidence | **148** | `HEAD` (see the gate report) | 25 + 22 + 49 + 9 + 25 + 18 (the `json` unit tests are also collected by the `cli` and `bounds` binaries via `#[path]`) |

No earlier number was rewritten: each is an **historical state at its own SHA**, and the CI of each
wave counted the tests that actually ran. The 128/129 figures that appear in intermediate notes are
intermediate heads of the same wave, not a contradiction.

## 4. Licence and decisions

MPL-2.0 (`LICENSE`, 17 100 bytes) is present since PR #49/#50. No right to extract Ravum or
third-party code is granted. `H1`, `H3`, `H4`, `H6`, `H7`, `H8` remain open; `H2` is decided;
`H5` and `GO_MVP_0_1` are consumed.

## 5. Production contract this wave assumes (local-trusted)

1. One **local trusted user** runs Flow on a Git repository they own.
2. **Offline**: no network, no provider, no daemon, no database. Git is required.
3. **Obligations are declared and approved**: `.flow/config.json` is a claim; `flow approve` (or
   `flow init --adopt-policy`) is consent, and it is versioned by digest.
4. **No OS sandbox**: obligations run with the user's rights. Untrusted code execution is
   `UNSUPPORTED` and out of scope; that is `H3`, a different product.
5. **No hooks executed by Flow**: the Git plumbing it calls may run `post-checkout` and
   `reference-transaction`; a blocking one fails the command. Measured, not assumed.
6. **Publication moves refs only**: HEAD/index/worktree may diverge afterwards, by design.
7. **Receipts are evidence, not signatures**: a writer who can rewrite `.flow/` can forge one.
8. Supported OS: **Windows (MSVC) and Linux**, x86_64. macOS is `NOT_TESTED_MACOS`.

## 6. Baseline re-executed before any change

```
cargo fmt --all -- --check                     PASS
cargo clippy --locked --all-targets -- -D warnings   PASS
cargo test --locked --all-targets              132 executed / 0 failed / 0 ignored
cargo build --locked --release                 PASS (binary 1 013 248 bytes)
pwsh -File examples/polyglot/run-demo.ps1 -Fixture all   PASS (3 fixtures)
```

## 7. Ownership and blockers at G0

| Owner | Paths |
|---|---|
| Lead | `crates/flow/src/**`, `crates/flow/tests/{cli,proposals,concurrency,bounds,isolation}.rs`, CI, README, `docs/reviews/FLOW-PRODUCTION-*`, release artefacts |
| security | `experiments/mvp-qa/**`, `docs/reviews/FLOW-PRODUCTION-ADVERSARIAL.md` |
| quality | `experiments/quality/**`, `docs/reviews/FLOW-PRODUCTION-TEST-MATRIX.md` |
| tsx | `examples/polyglot/typescript/**`, demo drivers, `docs/QUICKSTART-10-MINUTES.md` |

Ordered blockers carried into G1: PRF-06 (path traversal), PRF-04 (tampered receipt display),
PRF-05 (doctor exit), FIL-15 (policy provenance), PRC-10 (crash hook in release), PRF-02 (missing
logs), FIL-08 residual (byte fidelity), FIL-11 (junction/hardlink), then G3 crash points, G4
scenarios, G5 TypeScript, G6 measurement, G7 release artefacts.

**Verdict G0: `G0_PASS`** — baseline understood, reproduced and owned; no destructive action taken.
