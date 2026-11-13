# Node fixture — `node --test` obligation

A plain JavaScript fixture using only Node's built-in test runner. There is no
`npm install` step, no lockfile and no dependency.

## 1. What is here

| Path | Purpose |
|---|---|
| `src/math.js` | `add(left, right)`, exported through CommonJS |
| `test/math.test.js` | two `node:test` cases |
| `package.json` | private package, no dependencies (`test` script is optional) |
| `.flow/config.json` | the declared policy (`flow-config/1`, `local-trusted`) |
| `.flow/.gitignore` | `*` — Flow's own state never enters the verified tree |

## 2. Declared policy

```json
{
  "schema": "flow-config/1",
  "policy_version": "FLOW-POLICY-1",
  "execution_boundary": "local-trusted",
  "obligations": [
    { "id": "node-test", "program": "node", "args": ["--test"], "required": true, "timeout_s": 120 }
  ],
  "disabled": false
}
```

`node --test` with no path argument uses Node's default discovery in the current
directory (`test/*.test.js` is one of the patterns). The run happens in Flow's
materialised copy of the verified tree, so `node_modules` is never needed.

## 3. Exact commands

```bash
node --test
```

Through Flow from a disposable copy:

```bash
pwsh -NoProfile -File examples/polyglot/run-demo.ps1 -Fixture node
./examples/polyglot/run-demo.sh node
```

## 4. Measured results (this host, 2026-11-09)

Windows 11, `node v26.13.0`, `git 2.51.1.windows.1`:

| Command | Result |
|---|---|
| `node --test` | PASS (exit 0): 2 tests, 0 failures |
| `flow run` on the fixture index tree | `PASSED`, exit 0, `node-test=PASSED` |
| `flow publish --expected-old <head>` | `PUBLISHED`, exit 0 |

Linux (WSL2 Ubuntu), `node v20.20.2`: the same obligation ran through Flow and
the demo reported `PASSED` and `PUBLISHED`. This is a local WSL measurement, not
CI evidence.

Node's test runner is available from Node 18; an older Node reports
`bad option: --test` and the obligation becomes `FAILED` (never `PASSED`).

## 5. Red path (deliberately available)

`test/math.test.js:10`:

```js
assert.equal(add(2, 3), 5);
```

Change `5` to `6`, then `git add -A` and run Flow:

```bash
flow run          # FAILED, exit 1, obligation "node-test" FAILED (node exit 1)
flow publish      # REFUSED, exit 1, code REQUIRED_NOT_PASSED; the ref does not move
```

## 6. Notes

- No network is used and no package manager runs.
- `node_modules/` is ignored, but it is never created.
- The fixture uses CommonJS (`require`) so no `"type": "module"` decision is
  needed; the test runner is the only dependency, and it is built in.
