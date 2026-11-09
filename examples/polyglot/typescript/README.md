# TypeScript fixture — `tsc --noEmit` and `node --test` on `.ts`

A dependency-free TypeScript project. Two required obligations, both executed
inside Flow's materialised copy of the verified tree:

1. `tsc --noEmit` — the type gate. It reads `tsconfig.json` (`strict`,
   `noEmit`) and type-checks `src/` **and** `test/`: the tests are inside the
   compile scope, so a type error in a test is a failed obligation.
2. `node --test test/math.test.ts` — the execution gate. Node ≥ 22.6 strips
   the type annotations and runs the real test file through the built-in test
   runner. Type stripping erases annotations, it does not check them: `tsc` is
   what makes them meaningful.

Nothing is installed at run time and no network is used.

## 1. What is here

| Path | Purpose |
|---|---|
| `src/math.ts` | typed `add(left, right)` and `sum(values)` |
| `test/math.test.ts` | three `node:test` cases, type-checked and executed |
| `types/node-shims.d.ts` | hand-written declarations for the two built-ins used (see §6) |
| `tsconfig.json` | `strict`, `noEmit`, `allowImportingTsExtensions`, includes `src`, `test`, `types` |
| `package.json` | private ESM package, no dependencies, no lockfile |
| `.flow/config.json` | the declared policy (`flow-config/1`, `local-trusted`) |
| `.flow/.gitignore` | `*` — Flow's own state never enters the verified tree |

## 2. Declared policy

```json
{
  "schema": "flow-config/1",
  "policy_version": "FLOW-POLICY-1",
  "execution_boundary": "local-trusted",
  "obligations": [
    { "id": "tsc",          "program": "tsc",  "args": ["--noEmit"],                        "required": true, "timeout_s": 120 },
    { "id": "node-ts-test", "program": "node", "args": ["--test", "test/math.test.ts"],     "required": true, "timeout_s": 120 }
  ],
  "disabled": false
}
```

Both are required: a `PASSED` receipt is publishable only when every one of them
ran and passed. If `tsc` is not on `PATH`, Flow reports the `tsc` obligation as
`UNSUPPORTED` and the run is not `PASSED`.

## 3. Exact commands

Run the obligations directly (this is what Flow executes inside the verified
tree, in the same order):

```bash
tsc --noEmit
node --test test/math.test.ts
```

Through Flow from a disposable copy:

```bash
pwsh -NoProfile -File examples/polyglot/run-demo.ps1 -Fixture typescript
./examples/polyglot/run-demo.sh typescript
```

`-Fixture all` / `all` includes this fixture.

## 4. Prerequisites (offline after a one-time install)

| Requirement | Version used here | Why |
|---|---|---|
| `tsc` on `PATH` | 5.8.2 | the first required obligation; there is no compiler shipped in the fixture |
| `node` | 26.13.0 | `--test` (Node ≥ 18) plus type stripping for `.ts` (Node ≥ 22.6) |

The TypeScript compiler is the one prerequisite the fixture cannot provide
itself. Install it once, outside the demo, when you have network access:

```bash
npm i -g typescript@5.8.2
tsc --version        # Version 5.8.2
```

After that the fixture is offline: no `npm install`, no `node_modules`, no
lockfile, no download at run time. The driver does **not** skip a host without
`tsc` — it prints `UNSUPPORTED` with that exact prerequisite and exits non-zero.

## 5. The explicit test path, and why it is not a bare `node --test`

`node --test` with no path uses Node's default discovery patterns
(`*.test.{js,cjs,mjs}`, …). Node ≥ 22.6 also discovers `.test.ts`, so a bare
`node --test` **passes on Node 26**. It is still not what this fixture declares,
because on a host with Node < 22.6 the same command discovers no file at all and
reports:

```
1..0
# tests 0
# pass 0
```

with **exit 0** — a false green: a test obligation that executed nothing would
be recorded as `PASSED`. Naming the file (`node --test test/math.test.ts`) fails
closed there instead: Node 20 exits 1 with `ERR_UNKNOWN_FILE_EXTENSION`, so the
obligation is `FAILED` and nothing can be published. The exact path is a
deliberate choice, not a missing glob.

The type gate is separate: on Node 20, `tsc --noEmit` still type-checks the
same files, so the pair fails for the right reason (execution unavailable) and
never for the wrong one (no test ran).

## 6. Why `types/node-shims.d.ts`

`@types/node` is not vendored and the fixture must type-check offline, so the
two built-in module surfaces that are actually imported are declared by hand in
`types/node-shims.d.ts`. It is not a copy of the Node typings and makes no claim
to be one; it exists only so `strict` can check the fixture's own code. If you
add an import that needs more of `node:*`, extend that file — or install
`@types/node` once and drop the shim, which changes the policy's inputs and
therefore requires `flow approve` again.

## 7. Measured results (this host, 2026-11-09)

Windows 11, `Node v26.13.0`, `tsc 5.8.2`, `git 2.51.1.windows.1`:

| Command | Result |
|---|---|
| `tsc --noEmit` | PASS (exit 0), no diagnostics |
| `node --test test/math.test.ts` | PASS (exit 0): 3 tests, 0 failures |
| `flow run` on the fixture index tree | `PASSED`, exit 0, `tsc=PASSED`, `node-ts-test=PASSED` |
| `flow publish --expected-old <head>` | `PUBLISHED`, exit 0, published tree = receipt tree |
| `flow run` after the red edit | `FAILED`, exit 1, `tsc=PASSED`, `node-ts-test=FAILED` (exit 1) |
| `flow publish` after the red edit | `REFUSED`, exit 1, code `REQUIRED_NOT_PASSED`, ref unchanged |

Linux (WSL2 Ubuntu, `node v20.20.2`, no `tsc`): **NOT_RUN** as a pass. The two
blockers are the ones this fixture is built to surface: `tsc` is absent, and
Node 20 cannot execute `.ts`. `run-demo.sh typescript` prints the `UNSUPPORTED`
line and exits 1 there rather than skipping. The same bash driver was exercised
to a full `PASSED` → `PUBLISHED` → `FAILED` → `REFUSED` sequence under Git Bash
(MSYS, `node v26.13.0`, `tsc 5.8.2`); that is a local measurement, not CI
evidence and not a Linux-run claim.

A type error is also red, independently of the test: changing a call to
`add("2", 3)` fails `tsc` with `TS2345` while `node --test` still passes. That
is the split the two obligations exist to show.

## 8. Red path (deliberately available)

`test/math.test.ts:9`:

```ts
assert.equal(add(2, 3), 5);
```

Change `5` to `6`, then `git add -A` and run Flow:

```bash
flow run          # FAILED, exit 1, obligation "node-ts-test" FAILED (node exit 1)
flow publish      # REFUSED, exit 1, code REQUIRED_NOT_PASSED; the ref does not move
```

`tsc` stays green on that edit: it is a wrong expectation, not a type error.
That is intentional — the red path proves execution really happened.

## 9. Notes

- No network is used and no package manager runs at demo time.
- `node_modules/`, `dist/` and `*.tsbuildinfo` are ignored; `noEmit` means no
  build output is produced, so the verified tree stays source-only.
- `package.json` sets `"type": "module"` because the test imports `.ts` by its
  full extension (required by Node's ESM resolver). Without it Node emits a
  `MODULE_TYPELESS_PACKAGE_JSON` warning and reparses the file as ESM.
- The fixture is a verification target, not part of the Flow crate workspace.
