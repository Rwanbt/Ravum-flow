# Polyglot fixtures — Rust, Node, Python and TypeScript

Four self-contained repositories that Flow can verify. They exist to make the
claim testable on a machine with no network access and no package downloads:
each fixture declares only the obligations its own language ships.

These fixtures are **verification targets, not part of the Flow crate
workspace**. The Rust fixture carries its own `[workspace]` table so `cargo`
never tries to join the Flow workspace; the Node, Python and TypeScript fixtures
have no build system to install.

The TypeScript fixture is the one exception to "no prerequisite": a TypeScript
compiler (`tsc`) cannot be shipped inside a source-only fixture and is required
on `PATH`. It is a **one-time** global install (`npm i -g typescript@5.8.2`),
never an install at demo time; there is still no `node_modules`, no lockfile and
no network access during a run. Without `tsc` the fixture fails closed — Flow
reports `UNSUPPORTED` and both demo drivers print an explicit `UNSUPPORTED`
line and exit non-zero; nothing is skipped silently.

## 1. Layout

| Fixture | Obligation | Test count | Red path |
|---|---|---|---|
| [`rust/`](rust/README.md) | `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --locked --offline` | 2 unit + 1 doc test | [`rust/src/lib.rs:24`](rust/src/lib.rs#L24) |
| [`node/`](node/README.md) | `node --test` | 2 tests | [`node/test/math.test.js:10`](node/test/math.test.js#L10) |
| [`python/`](python/README.md) | `python -m unittest discover -v` | 2 tests | [`python/tests/test_math_util.py:13`](python/tests/test_math_util.py#L13) |
| [`typescript/`](typescript/README.md) | `tsc --noEmit`, `node --test test/math.test.ts` | 3 type-checked tests | [`typescript/test/math.test.ts:9`](typescript/test/math.test.ts#L9) |

Each fixture ships:

- `.flow/config.json` — schema `flow-config/1`, execution boundary
  `local-trusted`, only the native obligations for that language;
- `.flow/.gitignore` — `*`, so Flow's journal, receipts and logs never enter the
  verified tree;
- `README.md` — the exact commands, the measured results on this host, and the
  one-line red edit;
- a `.gitignore` for the language's build output.

The TypeScript fixture additionally ships `tsconfig.json`, `package.json` (no
dependencies) and `types/node-shims.d.ts`, and its README documents the two
deliberate design choices: the explicit test path (a bare `node --test` is a
false green on Node < 22.6) and the hand-written built-in module declarations
(`@types/node` cannot be fetched offline).

The four `.flow/config.json` files are force-added to the Flow repository
(`git add -f`): `flow init` writes `.flow/.gitignore` with `*`, which ignores the
configuration as well. In a user repository the config is local policy and is
not part of the verified tree; here it must be committed so the fixture is
reproducible.

## 2. Prerequisites (all offline)

| Fixture | Toolchain used here (Windows / Linux) |
|---|---|
| rust | `cargo` 1.95.0 / 1.94.1, with `rustfmt` and `clippy` components |
| node | `node` 26.13.0 / 20.20.2 (built-in `--test`, no npm) |
| python | `python` 3.13.7 / 3.12.3 (standard library only) |
| typescript | `node` 26.13.0 and a global `tsc` 5.8.2 (`npm i -g typescript@5.8.2`, one-time; the Windows and Git Bash runs) — **no Linux pass**: WSL2 has `node` 20.20.2 and no `tsc` |

No dependency is downloaded. `cargo test --locked --offline` and `node --test`
and `python -m unittest` all run from the materialised copy of the tree. The
TypeScript fixture needs the compiler on `PATH`; it never installs it.

## 3. The demo drivers

[`run-demo.ps1`](run-demo.ps1) (PowerShell 7) and [`run-demo.sh`](run-demo.sh)
(bash) perform the same sequence on a fresh temporary copy:

1. locate the `flow` binary (`$FLOW_BIN` / `$env:FLOW_BIN`), or build it once
   with `cargo build --manifest-path <repo>/Cargo.toml` and an external
   `--target-dir`, so **nothing is written into this repository**;
2. copy one fixture into a new temporary directory and turn it into a Git
   repository (`git init`, local identity, `core.autocrlf=false`);
3. adopt the policy (`flow init --adopt-policy`);
4. apply a small real change and `git add -A`, then `flow run` -> `PASSED`;
5. `flow publish --expected-old <head>` -> `PUBLISHED`; assert that the moved
   ref carries exactly the receipt's tree;
6. apply the fixture's documented red edit, `git add -A`, then `flow run` ->
   `FAILED` and `flow publish` -> `REFUSED` with a machine code, and assert that
   the ref did **not** move;
7. print a compact summary and remove the temporary tree.

Usage:

```bash
pwsh -NoProfile -File examples/polyglot/run-demo.ps1                       # rust
pwsh -NoProfile -File examples/polyglot/run-demo.ps1 -Fixture all
pwsh -NoProfile -File examples/polyglot/run-demo.ps1 -Fixture python -KeepTemp
pwsh -NoProfile -File examples/polyglot/run-demo.ps1 -Fixture typescript

./examples/polyglot/run-demo.sh                                            # rust
FLOW_BIN=./target/debug/flow ./examples/polyglot/run-demo.sh all
KEEP_TEMP=1 ./examples/polyglot/run-demo.sh node
./examples/polyglot/run-demo.sh typescript
```

`all` now runs all four fixtures: `rust`, `node`, `python`, `typescript`.

Both drivers fail loudly (`$ErrorActionPreference='Stop'`, `set -euo pipefail`)
and never use `|| true`. Neither script needs network access.

The **TypeScript fixture requires a TypeScript compiler on `PATH`** (`tsc`,
tested with 5.8.2; one-time `npm i -g typescript@5.8.2`) and Node ≥ 22.6. The
drivers check for `tsc` before copying anything and fail closed with an explicit
`UNSUPPORTED` message and exit 1 — they never skip the fixture or report it as
passed. Under `-Fixture all` / `all`, a missing `tsc` therefore fails the whole
run (after the other fixtures) rather than producing a green summary.

### Policy adoption compatibility

The final CLI gates execution on an explicit adoption: `flow init --adopt-policy`
on a repository that already ships `.flow/config.json`, and `flow approve` after
an intentional edit. The binary built from the branch this fixture set was
developed on predates that gate: it refuses `flow init` when `.flow/` already
exists and uses a shipped config implicitly.

The drivers therefore detect the capability and, when the flag is absent, print
one explicit line saying the shipped config is used as-is. That fallback is a
pre-adoption behaviour, recorded here as a limitation, not a claim that the
adoption gate has been exercised.

**Integration check (added later).** The integrated CLI at `17c0156` was built
from a throwaway `git archive` copy and both drivers were run against it with
`-Fixture all` / `all`: all three fixtures took the real
`init --adopt-policy` path and passed the `PASSED` → `PUBLISHED` → `FAILED` →
`REFUSED` sequence. The fallback text above still applies to the pre-adoption
binary in this worktree.

## 4. Measured results (2026-11-09, local)

Windows 11 (PowerShell 7, `git 2.51.1.windows.1`):

| Fixture | Direct obligation | `flow run` | `flow publish` | Red `flow run` | Red `flow publish` |
|---|---|---|---|---|---|
| rust | PASS (fmt, clippy, test) | PASSED, exit 0 | PUBLISHED, exit 0 | FAILED, exit 1 (`test`) | REFUSED, exit 1, `REQUIRED_NOT_PASSED` |
| node | PASS, 2 tests | PASSED, exit 0 | PUBLISHED, exit 0 | FAILED, exit 1 (`node-test`) | REFUSED, exit 1, `REQUIRED_NOT_PASSED` |
| python | PASS, 2 tests | PASSED, exit 0 | PUBLISHED, exit 0 | FAILED, exit 1 (`unittest`) | REFUSED, exit 1, `REQUIRED_NOT_PASSED` |
| typescript | PASS: `tsc --noEmit` + 3 tests | PASSED, exit 0 (`tsc`, `node-ts-test`) | PUBLISHED, exit 0 | FAILED, exit 1 (`node-ts-test`; `tsc` stays PASSED) | REFUSED, exit 1, `REQUIRED_NOT_PASSED` |

The TypeScript row was measured with `node 26.13.0` and `tsc 5.8.2`, through
both drivers (`run-demo.ps1 -Fixture typescript` and, under Git Bash,
`run-demo.sh typescript`). Its `UNSUPPORTED` path was also exercised: with the
`tsc` directory removed from `PATH`, both drivers printed the `UNSUPPORTED`
line and exited 1.

Linux (WSL2 Ubuntu, bash): `run-demo.sh rust`, `run-demo.sh node` and
`run-demo.sh python` each reported `PASSED` -> `PUBLISHED` -> `FAILED` ->
`REFUSED`, with the ref unchanged after the refusal. `run-demo.sh typescript`
is **not** a Linux pass here: WSL2 has `node 20.20.2` (no `.ts` execution) and
no `tsc`, so the driver stops with `UNSUPPORTED` and exit 1 — which is the
documented, intended outcome for that host. The WSL run is a local measurement;
it is not CI evidence and is not claimed as such.

The demo builds the flow binary from the repository under test; no release
artifact is downloaded.

## 5. Running a fixture by hand

```bash
cp -R examples/polyglot/rust /tmp/rust-fix
cd /tmp/rust-fix
git init -b main
git config user.name demo
git config user.email demo@example.invalid
git add -A && git commit -m baseline
flow init --adopt-policy
printf '\n// verified change\n' >> src/lib.rs
git add -A
flow run
flow publish --expected-old "$(git rev-parse HEAD)"
```

Then apply the red edit from the table above and repeat `git add -A`, `flow run`,
`flow publish`. A refused publication exits non-zero and leaves the ref where it
was.

## 6. Boundaries

These fixtures demonstrate a **local-trusted** verification path. They do not
demonstrate, and must not be read as claiming:

- an OS sandbox: obligations run with the invoking user's rights;
- hook execution: Flow dispatches no hook of its own;
- a completeness proof of the build context;
- signature-grade receipt integrity: the digest detects edits, it is not a
  signature;
- CI results: the Ubuntu/Windows CI matrix is owned by the integration branch.

See [the user guide](../../docs/MVP-USER-GUIDE.md) and
[CLI JSON reference](../../docs/CLI-JSON.md).
