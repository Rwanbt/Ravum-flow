# Quickstart — verify and publish a tree in 10 minutes

This walks one complete production-candidate flow on a **disposable** project:
clone and build the CLI, adopt a policy, admit an agent's patch as a proposal,
assemble it into an immutable candidate, verify the candidate (not the working
tree), plan the publication, publish it with a compare-and-swap, and inspect the
receipt with `flow explain`.

**Measured** on 2026-11-09 against `feat/flow-production-candidate` at
`44db6fc60c265c0116a7a7e84390e28b9f588372` (`flow 0.2.0-rc.1`, `flow-receipt/2`),
on Windows 11 (PowerShell 7.6.9) and on WSL2 Ubuntu (bash). Both hosts produced
the **same candidate tree** for the same demo bytes:
`03f518fc57694acc5601ab1b5fd4c5e32fbda96d`. Commit and candidate ids differ
between the two runs because a commit includes its timestamp; the tree does not.

This is a local, `local-trusted` demonstration. It is **not** CI evidence and it
does not demonstrate an OS sandbox, hook execution, a signed receipt, or
publication of any ref other than a local branch.

## 1. What you need

| Requirement | Used here | Why |
|---|---|---|
| `git` | 2.51.1 (Windows) / 2.43.0 (WSL) | repositories, worktrees, ref CAS |
| Rust toolchain (`cargo`) | 1.95.0 (Windows) / 1.94.1 (WSL) | build the CLI; it has **zero** crate dependencies |
| Node.js | 26.13.0 (Windows) / 20.20.2 (WSL) | the demo project's `node --test` obligation |
| network | once, for the clone | nothing is downloaded after that |
| time | ~10 minutes | measured: clone+build 13–17 s, the rest seconds |

`cargo build --release --offline` works because the Flow crate has no
dependencies. The demo project installs nothing and uses only Node's built-in
test runner.

## 2. Clone and build

PowerShell (Windows):

```powershell
Set-Location $HOME
git clone --depth 1 --branch feat/flow-production-candidate https://github.com/Rwanbt/Ravum-flow.git flow-quickstart-src
Set-Location flow-quickstart-src
cargo build --release --offline
```

bash (Linux):

```bash
cd ~
git clone --depth 1 --branch feat/flow-production-candidate https://github.com/Rwanbt/Ravum-flow.git flow-quickstart-src
cd flow-quickstart-src
cargo build --release --offline
```

Expected: `Finished \`release\` profile [optimized] target(s)` and exit **0**
(measured 12.6 s on Windows, 17.0 s on WSL).

Put the binary on `PATH` for the rest of the session:

```powershell
$env:PATH = "$HOME\flow-quickstart-src\target\release;$env:PATH"
flow version
```

```bash
export PATH="$HOME/flow-quickstart-src/target/release:$PATH"
flow version
```

Expected `flow version` — status `VERSION`, exit **0**:

```json
{
  "status": "VERSION",
  "tool": "flow 0.2.0-rc.1",
  "policy_version": "FLOW-POLICY-1",
  "schema": "flow-receipt/2"
}
```

> `--json` is accepted for compatibility; every command already prints JSON on
> stdout. A JSON status is a machine contract: do not parse the human prose.

## 3. Create the disposable demo project

Create `~/flow-quickstart-demo` (PowerShell: `$HOME\flow-quickstart-demo`) and
save these three files. The directory is disposable; §12 removes it.

`src/math.js`:

```js
'use strict';

// Dependency-free demo project: integer helpers exercised by node:test.
function add(left, right) {
  return left + right;
}

module.exports = { add };
```

`test/math.test.js`:

```js
'use strict';

// Run with `node --test` from the repository root.
const test = require('node:test');
const assert = require('node:assert/strict');

const { add } = require('../src/math.js');

test('adds positive numbers', () => {
  assert.equal(add(2, 3), 5);
});

test('adds negative numbers', () => {
  assert.equal(add(-2, 3), 1);
});
```

`.flow/config.json` — the policy. It is a **claim about what should run**; it is
not consent to run it (that is `flow init --adopt-policy` in §4):

```json
{
  "schema": "flow-config/1",
  "policy_version": "FLOW-POLICY-1",
  "execution_boundary": "local-trusted",
  "obligations": [
    {
      "id": "node-test",
      "program": "node",
      "args": ["--test"],
      "required": true,
      "timeout_s": 120
    }
  ],
  "disabled": false
}
```

Initialise the repository and commit the baseline. Save the files as LF: the two
hosts then verify **byte-identical trees**. (A CRLF save also works — the policy
does not inspect line endings — but it yields different tree OIDs.)

PowerShell:

```powershell
New-Item -ItemType Directory -Force "$HOME\flow-quickstart-demo" | Out-Null
Set-Location "$HOME\flow-quickstart-demo"
git init -q -b main
git config user.name "Flow Quickstart"
git config user.email "quickstart@example.invalid"
git config commit.gpgsign false
git config core.autocrlf false        # verify the bytes you saved
New-Item -ItemType Directory -Force src, test, .flow | Out-Null
# ... save the three files above ...
git add -A
git commit -q -m "baseline: a small Node project with a declared policy"
$base = (git rev-parse HEAD).Trim()
$base
```

bash:

```bash
mkdir -p "$HOME/flow-quickstart-demo"
cd "$HOME/flow-quickstart-demo"
git init -q -b main
git config user.name "Flow Quickstart"
git config user.email "quickstart@example.invalid"
git config commit.gpgsign false
git config core.autocrlf false        # verify the bytes you saved
mkdir -p src test .flow
# ... save the three files above ...
git add -A
git commit -q -m "baseline: a small Node project with a declared policy"
base="$(git rev-parse HEAD)"
printf '%s\n' "$base"
```

Expected: commit exit **0**; `$base` is a 40-hex commit id.

## 4. Adopt the policy

`flow init` refuses an unadopted policy: a repository that arrives with a
`.flow/config.json` does not execute it until someone adopts it explicitly.

```bash
flow init --adopt-policy
```

Expected — status `INITIALIZED`, exit **0**:

```json
{
  "status": "INITIALIZED",
  "flow_dir": ".flow",
  "policy_version": "FLOW-POLICY-1",
  "adopted": true,
  "adoption_source": "init --adopt-policy",
  "policy_digest": "<12-hex prefix>",
  "journal_preserved": true
}
```

`init` also writes `.flow/.gitignore` containing `*`: Flow's journal, receipts,
logs, proposals and candidates never enter the tree that is verified.

## 5. Produce the first proposal

An agent never edits the shared working tree. It works in its own Git worktree
at the explicit base, and hands over a patch (`git diff --cached --binary`) that
Flow admits as an immutable, attributed proposal.

Create `src/multiply.js`:

```js
'use strict';

// Added by agent-a in its own worktree and proposed as a patch.
function multiply(left, right) {
  return left * right;
}

module.exports = { multiply };
```

Create `test/multiply.test.js`:

```js
'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');

const { multiply } = require('../src/multiply.js');

test('multiplies', () => {
  assert.equal(multiply(4, 5), 20);
});
```

PowerShell:

```powershell
git worktree add --detach .agent-a $base
Set-Location .agent-a
# ... save the two files above ...
git add -A
git diff --cached --binary --output="$HOME\flow-quickstart-agent-a.patch" $base
Set-Location ..
git worktree remove --force .agent-a
```

bash:

```bash
git worktree add --detach .agent-a "$base"
cd .agent-a
# ... save the two files above ...
git add -A
git diff --cached --binary --output="$HOME/flow-quickstart-agent-a.patch" "$base"
cd ..
git worktree remove --force .agent-a
```

Both steps exit **0**; the patch is ~700–800 bytes and touches only the two new
files. Admit it:

```bash
flow propose --agent agent-a --task T1 --base "$base" --patch "$HOME/flow-quickstart-agent-a.patch" --key k1
```

PowerShell (capture the id):

```powershell
$text = flow propose --agent agent-a --task T1 --base $base --patch "$HOME\flow-quickstart-agent-a.patch" --key k1
if ($LASTEXITCODE -ne 0) { throw "propose failed: $text" }
$proposalId = ($text | ConvertFrom-Json).proposal_id
$proposalId
```

bash (capture the id):

```bash
proposal="$(flow propose --agent agent-a --task T1 --base "$base" --patch "$HOME/flow-quickstart-agent-a.patch" --key k1)"
if [ $? -ne 0 ]; then printf '%s\n' "$proposal" >&2; exit 1; fi
proposal_id="$(printf '%s' "$proposal" | sed -n 's/.*"proposal_id": "\([^"]*\)".*/\1/p')"
printf '%s\n' "$proposal_id"
```

Expected — status `ADMITTED`, exit **0**:

```json
{
  "schema": "flow-proposal/1",
  "status": "ADMITTED",
  "proposal_id": "<32-hex>",
  "agent_id": "agent-a",
  "task_id": "T1",
  "idempotency_key": "k1",
  "base_commit": "<base>",
  "tree_oid": "<proposal-tree>",
  "changes": [
    { "path": "src/multiply.js", "status": "A", "old_mode": "000000", "new_mode": "100644" },
    { "path": "test/multiply.test.js", "status": "A", "old_mode": "000000", "new_mode": "100644" }
  ]
}
```

Re-submitting the same `--key` with the same bytes returns the same record with
`"idempotent": true`; the same key with different bytes is refused
(`IDEMPOTENCY_CONFLICT`). Optional check: `flow proposals list` → status
`RECORD`, exit **0**, `"count": 1`.

## 6. Assemble the candidate

```bash
flow assemble --proposal "$proposal_id"
```

PowerShell:

```powershell
$text = flow assemble --proposal $proposalId
if ($LASTEXITCODE -ne 0) { throw "assemble failed: $text" }
$candidateId = ($text | ConvertFrom-Json).candidate_id
$candidateId
```

bash:

```bash
candidate="$(flow assemble --proposal "$proposal_id")"
if [ $? -ne 0 ]; then printf '%s\n' "$candidate" >&2; exit 1; fi
candidate_id="$(printf '%s' "$candidate" | sed -n 's/.*"candidate_id": "\([^"]*\)".*/\1/p')"
printf '%s\n' "$candidate_id"
```

Expected — status `ASSEMBLED`, exit **0**:

```json
{
  "schema": "flow-candidate/1",
  "status": "ASSEMBLED",
  "candidate_id": "<32-hex>",
  "base_commit": "<base>",
  "tree_oid": "<candidate-tree>",
  "proposals": ["<proposal-id>"],
  "attribution": [
    { "proposal_id": "<proposal-id>", "agent_id": "agent-a", "task_id": "T1", "base_commit": "<base>" }
  ]
}
```

No ref, index or worktree was touched: the candidate exists only as a Git tree
plus a record under `.flow/candidates/`. Multiple proposals may be assembled at
once (`--proposal` repeated); they must share a base, and a path collision is
refused with a code rather than merged silently.

## 7. Verify the candidate

```bash
flow run --candidate "$candidate_id"
```

Expected — status `PASSED`, exit **0**:

```json
{
  "schema": "flow-receipt/2",
  "tree_oid": "<candidate-tree>",
  "status": "PASSED",
  "candidate_id": "<candidate-id>",
  "verification_scope": { "mode": "complete", "missing_required": [] },
  "obligations": [
    { "id": "node-test", "status": "PASSED", "required": true, "exit": 0 }
  ]
}
```

The receipt is bound to the candidate's **tree OID**, the effective policy digest
and the execution context — not to the working tree. Flow materialised the
candidate tree in a directory it owns and ran `node --test` there; the three
tests (both baseline tests and the proposed one) passed.

An obligation that cannot start (a missing program) is `UNSUPPORTED`; a failing
one is `FAILED`. Neither is `PASSED`, and neither is publishable.

## 8. Plan the publication

A dry run performs the same preflight as the real publication and stops before
creating a commit, moving a ref or appending a journal record.

```bash
flow publish --candidate "$candidate_id" --expected-old "$base" --dry-run
```

Expected — status `PLAN`, exit **0**:

```json
{
  "status": "PLAN",
  "schema": "flow-plan/1",
  "dry_run": true,
  "ref": "refs/heads/main",
  "old": "<base>",
  "tree": "<candidate-tree>",
  "candidate_id": "<candidate-id>",
  "obligations": ["node-test"],
  "note": "plan only: no commit was created, no ref was moved, no journal record was appended"
}
```

Check it yourself — the branch still points at the base afterwards:

```bash
git rev-parse refs/heads/main        # == $base
```

## 9. Publish

```bash
flow publish --candidate "$candidate_id" --expected-old "$base"
```

Expected — status `PUBLISHED`, exit **0**:

```json
{
  "status": "PUBLISHED",
  "candidate_id": "<candidate-id>",
  "ref": "refs/heads/main",
  "old": "<base>",
  "commit": "<new-commit>",
  "tree": "<candidate-tree>",
  "note": "only refs were moved; the working tree was not modified and may now differ"
}
```

`--expected-old` is the compare-and-swap: if the branch no longer points at
`$base`, the publication is refused (`STALE_REF`, exit **1**) and the ref does
not move. Verify the invariant that matters:

```bash
git rev-parse refs/heads/main            # == the PUBLISHED "commit"
git rev-parse 'refs/heads/main^{tree}'   # == the ASSEMBLED "tree_oid"
```

## 10. Explain the result

```bash
flow explain --candidate "$candidate_id"
```

Expected — status `EXPLAIN`, exit **0**:

```json
{
  "status": "EXPLAIN",
  "tree": "<candidate-tree>",
  "candidate_id": "<candidate-id>",
  "candidate": { "status": "ASSEMBLED" },
  "proposals": [
    { "proposal_id": "<proposal-id>", "agent_id": "agent-a", "task_id": "T1" }
  ],
  "receipt": { "status": "PASSED" },
  "receipt_trust": "INTEGRITY_OK",
  "evidence_complete": true,
  "missing_logs": [],
  "journal": [ { "event": "intent", "ref": "refs/heads/main", "old": "<base>", "new": "<new-commit>" } ]
}
```

This is the whole story for one tree: who proposed what, which candidate was
assembled, which receipt proves it, and the ref intent that moved it.

## 11. Sync your working tree (expected, not an error)

Publication moves **refs only**. Your index and working tree were never touched,
so Git now reports the difference between the index (base) and the new `HEAD`
(candidate). That is the documented behaviour, not a failed publish.

```bash
git status --short
# D  src/multiply.js
# D  test/multiply.test.js
```

Sync to the verified commit when you are ready:

```bash
git restore --source=HEAD --staged --worktree .
git status --short        # empty
```

PowerShell: the same commands (`git restore --source=HEAD --staged --worktree .`)
exit **0** and leave an empty status.

## 12. Clean up

```bash
rm -rf "$HOME/flow-quickstart-src" "$HOME/flow-quickstart-demo" "$HOME/flow-quickstart-agent-a.patch"
```

```powershell
Remove-Item -Recurse -Force "$HOME\flow-quickstart-src", "$HOME\flow-quickstart-demo"
Remove-Item -Force "$HOME\flow-quickstart-agent-a.patch"
```

Nothing in the cloned Flow repository was modified by the demo: the demo lives
in its own directory, and the CLI was built inside the clone's `target/`.

## 13. What can go wrong

Every row below was **measured** on this host (Windows, `flow 0.2.0-rc.1`).
`flow` prints the status and, for refusals, a machine-readable `code`.

| Symptom | Cause | Command that shows it | Measured result |
|---|---|---|---|
| `STALE_REF` | `--expected-old` is not the current tip | `flow publish --candidate <id> --expected-old <old-base> --dry-run` | `REFUSED`, exit 1 |
| `ALREADY_PUBLISHED` | the ref already carries the verified tree | `flow publish --candidate <id> --expected-old <current-commit> --dry-run` | `REFUSED`, exit 1 |
| `NO_RECEIPT` | the index tree changed after the last `flow run` | edit a file, `git add -A`, `flow publish --expected-old <head>` | `REFUSED`, exit 1 |
| `UNKNOWN_CANDIDATE` | typo in the candidate id | `flow run --candidate 0123456789abcdef0123456789abcdef` | `REFUSED`, exit 1 |
| `BASE_INVALID` | `--base` is not a commit in this object store | `flow propose ... --base deadbeef ...` | `REFUSED`, exit 1 |
| `PATCH_REFUSED` | the patch does not apply to its base (e.g. re-proposed after publication) | `flow propose ... --base <new-head> --patch <old.patch> --key k3` | `REFUSED`, exit 1 |
| `NO_PROPOSAL` | `assemble` without `--proposal` | `flow assemble` | `REFUSED`, exit 1 |
| `POLICY_NOT_APPROVED` | `.flow/config.json` was edited after adoption | edit the config, `flow run` | `REFUSED`, exit 1 — fix with `flow approve` |
| `FAILED` | an obligation really failed | change an assertion, `git add -A`, `flow run` | `FAILED`, exit 1; the failing obligation names itself |
| `REQUIRED_NOT_PASSED` | publication attempted on a non-`PASSED` tree | `flow publish` after the failing run above | `REFUSED`, exit 1; ref unchanged |
| `ERROR`, exit 2 | usage error, unknown flag | `flow run --bogus` | `ERROR`, exit **2** (no mutation) |
| `UNSUPPORTED` | a required program is not on `PATH` | the TypeScript fixture with `tsc` removed from `PATH`, then `flow run` | run `UNSUPPORTED`, exit 1 (`tsc` `UNSUPPORTED`, the other obligation `PASSED`); `flow publish` → `REFUSED` / `REQUIRED_NOT_PASSED`, exit 1 |
| `D`/`M` in `git status` after publish | refs moved, index/worktree did not | `git status --short` after §9 | expected; §11 syncs it |
| receipt's `worktree_diff` is non-empty | the local worktree differs from the verified tree | any `flow run` while your worktree is behind | informational only; the receipt still describes the verified tree |

Two things that are **not** errors and are easy to misread: a non-empty
`worktree_diff` in the receipt (the local checkout may be behind — the receipt
describes the tree, not your files), and a `D`/`M` status after publication
(refs only). Neither changes a status to `FAILED`.

## 14. Measured evidence and what was not run

Measured at `feat/flow-production-candidate` = `44db6fc`, `flow 0.2.0-rc.1`:

| Step | Command | Status | Exit | Windows | Linux (WSL2) |
|---|---|---|---|---|---|
| build | `cargo build --release --offline` | — | 0 | 12.6 s | 17.0 s |
| version | `flow version` | `VERSION` | 0 | 0.2.0-rc.1 | 0.2.0-rc.1 |
| adopt | `flow init --adopt-policy` | `INITIALIZED` | 0 | yes | yes |
| propose | `flow propose ...` | `ADMITTED` | 0 | yes | yes |
| list | `flow proposals list` | `RECORD` | 0 | count 1 | not run |
| assemble | `flow assemble --proposal ...` | `ASSEMBLED` | 0 | yes | yes |
| verify | `flow run --candidate <id>` | `PASSED` | 0 | yes | yes |
| plan | `flow publish --candidate <id> --dry-run` | `PLAN` | 0 | yes | yes |
| publish | `flow publish --candidate <id> --expected-old <base>` | `PUBLISHED` | 0 | yes | yes |
| explain | `flow explain --candidate <id>` | `EXPLAIN` | 0 | yes | yes |
| sync | `git restore --source=HEAD --staged --worktree .` | — | 0 | yes | yes |

Both runs assembled the same candidate tree
`03f518fc57694acc5601ab1b5fd4c5e32fbda96d` from the same bytes. The Windows run
also exercised every refusal in §13. The `UNSUPPORTED` row was measured
separately against the TypeScript fixture (`tsc` removed from `PATH`:
`tsc=UNSUPPORTED`, `node-ts-test=PASSED`, run status `UNSUPPORTED` exit 1;
`flow publish` → `REFUSED` / `REQUIRED_NOT_PASSED`, exit 1).

**NOT_RUN** in this quickstart (do not read the absence as a claim):

- `flow doctor`, `flow disable`/`enable`, `flow reconcile`, `flow status`.
- `flow init` **without** `--adopt-policy` (the unadopted-policy refusal).
- Publication to any ref other than a local branch, and tags.
- Two proposals in one candidate (assembly of multiple `--proposal` arguments).
- A real path collision between proposals, and its resolution.
- Signed receipts: `integrity` detects edits; it is not a signature.
- A native Linux host: the measurement is WSL2, not bare metal; macOS and ARM
  Windows were not tried.

## 15. Where to go next

- [MVP user guide](MVP-USER-GUIDE.md) — the full command and lifecycle model.
- [CLI JSON reference](CLI-JSON.md) — every status, code and schema field.
- [Polyglot fixtures](../examples/polyglot/README.md) — Rust, Node, Python and
  TypeScript verified end to end, each with a documented red edit.
