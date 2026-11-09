# Ravum Flow — MVP user guide

**Status of this document:** written for the MVP 0.1 integration branch
(`work/mvp-polyglot`), against the CLI that actually exists in this worktree at
branch point `d884e29`. The final 0.1 surface (`propose`, `assemble`,
`run --candidate`, `publish --dry-run`, `init --adopt-policy`, `approve`,
`--json`) is being implemented in parallel by the Lead; every part of it that
this worktree cannot execute is marked **NOT_RUN** and listed in the
[status table](#14-status-pending-in-this-worktree). Nothing here is a promise
about code that was not read or run.

Claims are labelled where they matter: **MEASURED** (executed on this host and
recorded), **CODE_READ** (read in `crates/flow/src/**`), **NOT_RUN** (not
executable in this worktree).

Related documents: [CLI and JSON reference](CLI-JSON.md) ·
[Architecture](ARCHITECTURE.md) ·
[P0 hardening report](reviews/FLOW-FOUNDATION-P0-HARDENING-REPORT.md) ·
[polyglot fixtures](../examples/polyglot/README.md).

## 1. What Flow is

Flow is a local, offline, dependency-free command-line tool that answers one
question with evidence: *is exactly this Git tree verified by the policy this
repository declared?* It then optionally moves a branch ref to a commit carrying
that tree, behind a compare-and-swap and an append-only journal.

It is not a version control system, a remote, a build scheduler, a sandbox, an
LLM agent or a proof system. It has no daemon, no network access and no
dependencies (**CODE_READ**: `crates/flow/Cargo.toml` has an empty
`[dependencies]`).

## 2. What Flow is not — the honest limits

These are boundary conditions, not incidental caveats.

1. **Local-trusted execution, no sandbox.** Obligations run with your user
   rights. The environment is scrubbed to an allowlist (`PATH`, `HOME`,
   `USERPROFILE`, `CARGO_HOME`, `SystemRoot`, …) and `stdin` is empty
   (**CODE_READ**: `gitcli.rs`), but there is no OS isolation. Untrusted code
   execution is `UNSUPPORTED` and `flow doctor` says so.
2. **No hook execution of its own.** Flow does not dispatch repository hooks.
   The Git plumbing it calls can still run a `post-checkout` hook on
   `worktree add` and a `reference-transaction` hook on `update-ref`; a blocking
   `reference-transaction` hook can abort a publish.
3. **No completeness proof of the build context.** `context_id` covers only
   declared inputs: the tree, the effective policy, the Git version, OS and
   architecture. An undeclared input (an ignored cache file, an environment
   variable, a toolchain version not in the receipt) can still change a result.
4. **Receipt integrity is not a signature.** The `integrity` field is a SHA-256
   digest over the receipt body; it detects accidental corruption and casual
   edits. Anyone who can write `.flow/receipts` can forge a receipt.
5. **A foreign `git` writer is not fenced.** Flow serialises Flow processes with
   a local advisory lock (`.flow/lock`, stale after 120 s). Another `git`
   process is not blocked and is not serialised against Flow: the ref CAS can
   still refuse a race it observes, but the foreign writer itself is never
   fenced.
6. **`publish` moves refs only.** It does not check out, reset or clean the
   working tree. `HEAD`, the index and the worktree can diverge after a publish.
7. **No checkout of an unfaithful tree.** Case-colliding paths, symlinks when
   `core.symlinks=false`, and gitlinks are refused (`UNSUPPORTED_CHECKOUT`),
   not normalised or repaired.
8. **`PASSED` is not `PROVED`.** It means the declared obligations ran and
   passed on that tree under that policy. It says nothing about undeclared
   behaviour.

## 3. Requirements and installation

- Git 2.x on `PATH`. Flow refuses to run outside a Git work tree.
- Rust 1.78+ when you build from source (`cargo`), with `rustfmt` and `clippy`
  components if your policy declares them.
- Nothing else: no network, no package download, no service account.

Build from a checkout (offline is enough, the crate has no dependencies):

```bash
cargo build --locked --offline            # debug binary at target/debug/flow
cargo build --release --locked --offline  # target/release/flow
```

On Windows the binary is `flow.exe`; both scripts and examples work with either
path. The `examples/polyglot` drivers accept `FLOW_BIN` / `$env:FLOW_BIN` and
otherwise build the CLI with an external `--target-dir` so the repository is not
written to.

## 4. First run: initialise, inspect, approve

### 4.1 `flow init`

Creates `.flow/` with `config.json` (an empty policy), `journal.jsonl`,
`receipts/`, `logs/`, and `.flow/.gitignore` containing `*`, which keeps Flow's
own state out of every tree it later verifies.

```console
$ flow init
{
  "status": "INITIALIZED",
  "flow_dir": ".flow",
  "policy_version": "FLOW-POLICY-1",
  "note": "no obligation declared yet: `flow run` will refuse with UNKNOWN until .flow/config.json lists obligations; .flow/.gitignore keeps Flow state out of the verified tree"
}
```

**MEASURED**, exit 0. Running it a second time is refused, exit 1
(**MEASURED**):

```console
$ flow init
{
  "status": "REFUSED",
  "reason": ".flow already exists (pass --force to rewrite config, journal is preserved)"
}
```

`--force` rewrites `config.json` with the empty default and preserves the
journal (**CODE_READ**). Do not use it to "fix" a policy you edited by hand;
edit the config and adopt it.

A repository whose `.flow/config.json` already exists is the case the final CLI
gates behind `--adopt-policy`; this worktree does not implement that gate yet
(**NOT_RUN**, see §14).

### 4.2 `flow approve` (planned in this branch)

Records local approval of the current `.flow/config.json` after an intentional
edit. Until it exists, editing the config takes effect immediately and every
existing receipt becomes unpublishable (`STALE_POLICY`) by design. **NOT_RUN**
in this worktree.

### 4.3 `flow doctor`

Read-only report of the environment and of what Flow does **not** guarantee.
**MEASURED**, exit 0 (abridged: the real report has one `checks` entry per check,
including `platform`, `disabled` and `journal`):

```console
$ flow doctor
{
  "status": "REPORT",
  "overall": "OK",
  "checks": [
    { "id": "git", "status": "OK", "detail": "git version 2.51.1.windows.1" },
    { "id": "flow_initialized", "status": "OK", "detail": "config present" },
    { "id": "hooks", "status": "OK", "detail": "no hook installed. Flow dispatches no hook of its own; ..." },
    { "id": "longpaths", "status": "WARN", "detail": "core.longpaths=\"\"" },
    { "id": "case_sensitivity", "status": "WARN", "detail": "core.ignorecase=\"true\": ..." },
    { "id": "toolchains", "status": "OK", "detail": "cargo=found rustc=found node=found python=found python3=found" },
    { "id": "execution_boundary", "status": "WARN", "detail": "local-trusted: ... WITHOUT an OS sandbox ..." }
  ],
  "guarantees": [ "receipts bind an exact Git tree OID and one effective policy digest", ... ],
  "non_guarantees": [ "no OS-level isolation of obligations", ... ]
}
```

Warnings are expected on Windows: `core.ignorecase=true`, unset
`core.longpaths`, and no sandbox. `overall` is `FAILED` if any check reports
`FAILED`; an unreadable journal is one of those checks.

### 4.4 `flow status`

Repository state plus the most recent receipt and the number of journal intents
without an outcome. **MEASURED**, exit 0:

```console
$ flow status
{
  "status": "REPORT",
  "branch": "main",
  "head": "531e89e80ae0c73d7aba50f0e6fd728a634c94f0",
  "tree": "e1e3809858cf380de1106a81f6f75de9b132d091",
  "dirty_count": 0,
  "untracked_count": 0,
  "obligations_declared": 3,
  "disabled": false,
  "latest_receipt": null,
  "pending_intents": 0,
  "hint": "`flow run` verifies the INDEX tree (git add first); the worktree is never modified"
}
```

If the journal cannot be read, `status` fails closed with `status: ERROR` and a
non-zero exit rather than reporting `pending_intents: 0` (**MEASURED**; see
§13).

## 5. Verify a change: the index tree

Flow verifies the **Git index tree**, not the working tree. Stage first:

```bash
git add -A
flow run
```

`flow run` calls `git write-tree`, materialises that exact tree in a throwaway
directory outside the repository (with the worktree's `.git` link removed), and
runs the obligations declared in `.flow/config.json` with a scrubbed
environment (**CODE_READ**: `commands.rs`, `scratch.rs`). The worktree is never
modified.

A run is `PASSED` only when the **whole** declared policy ran and passed:
`verification_scope.mode` is `complete` and `missing_required` is empty.

- `run --obligation ID` is diagnostic only. It produces `PARTIAL` (exit 1), is
  written to a separate `.partial.<context>.json` slot, and is never publishable
  (**MEASURED**; see §13).
- `run --timeout S` overrides every obligation's deadline and is part of the
  effective policy: the receipt is valid evidence, but it is **not publishable**
  against the configured policy (`STALE_POLICY`) (**MEASURED**; see §13).
- A required obligation that cannot start yields `UNSUPPORTED`; a child killed
  after the deadline yields `TIMEOUT`; neither is ever `PASSED`.
- An empty policy is `UNKNOWN`, exit 1 (**MEASURED**):

```console
$ flow run
{ "status": "UNKNOWN", "reason": "no obligation declared: verification scope is undefined, nothing is publishable" }
```

`output` is bounded at 1 MiB per stream. A truncated stream is flagged in the
receipt (`stdout_truncated` / `stderr_truncated`) rather than presented as
complete evidence.

### 5.1 The receipt

Every run writes `.flow/receipts/<tree_oid>.json` (a selective run writes a
different file). The receipt names the tree OID, the verification commit, the
context digest, the effective policy and its digest, the scope, per-obligation
outcomes with previews, log paths, and a sealed `integrity` digest. The full
field list is in the [CLI and JSON reference](CLI-JSON.md#53-flow-run).

The receipt is bound to three things, and **any change to any of them
invalidates publication**:

| Bound to | Invalidated by | Refusal code |
|---|---|---|
| the tree OID | any index change after `run` | `NO_RECEIPT` (the new tree has no complete receipt, unless one was produced earlier) |
| the effective policy digest | editing `.flow/config.json`, `--timeout` at run time | `STALE_POLICY` |
| the context (`flow-context/2`: tree, policy, Git version, OS, arch) | a Git upgrade or platform change between run and publish | `CONTEXT_MISMATCH` |

The receipt also carries policy/version/boundary values that must still match
(`POLICY_VERSION_MISMATCH`, `EXECUTION_BOUNDARY_MISMATCH`). A receipt edited on
disk is refused (`RECEIPT_INTEGRITY`) (**MEASURED**; see §13).

## 6. Publish: compare-and-swap, journal, recovery

`flow publish` requires a **complete `PASSED`** receipt for the current index
tree under the current policy and context. It then:

1. acquires the local lock (`.flow/lock`);
2. re-reads the current ref and refuses unless it still equals
   `--expected-old` (or the observed ref when the flag is omitted);
3. emits a commit carrying exactly the verified tree, with the old ref OID as
   parent;
4. appends a fsynced `intent` record to `.flow/journal.jsonl`;
5. re-reads the ref and refuses if it moved in the meantime (`REF_MOVED`);
6. runs `git update-ref <ref> <new> <old>`;
7. appends an `outcome` record.

Prefix rules: `--ref` must be a full `refs/heads/...` branch ref (`HEAD` and
tags are refused). Without `--ref`, a detached HEAD is refused with an error
asking for an explicit ref (**MEASURED**, §13).

**What this does not do:** it does not fence a foreign `git` writer, does not
check out the published tree, and does not push anywhere. `publish` moves local
refs only.

`publish --dry-run` (planned in this branch) writes no ref and no commit, prints
the plan and exits 0 with status `PLAN`. **NOT_RUN** here — do not pass
`--dry-run` to the binary in this worktree: the flag is not implemented, and a
publish would be real.

`flow reconcile` resolves an interrupted publication by **observation**: it
reads the journal and the refs, records `PUBLISHED`, `NOT_PUBLISHED`, or
`NEEDS_RECONCILIATION`, and never moves a ref and never performs a second blind
publish. A corrupt journal is reported as `CORRUPT_JOURNAL`, exit 1, never as a
successful empty reconciliation (**MEASURED**, §10).

## 7. Three-language walkthrough

The complete, reproducible form of this section is
[`examples/polyglot/README.md`](../examples/polyglot/README.md). Each fixture
declares exactly its own language's obligations and ships a documented red edit.

> The `flow init --adopt-policy` line below is the final CLI. On the binary in
> this worktree the flag does not exist: `flow init` is refused because `.flow/`
> already ships, and the fixture's `.flow/config.json` is used as-is. The demo
> drivers detect this and print one explicit fallback line; they never claim the
> adoption gate was exercised.

### 7.1 Rust

Policy: `cargo fmt --check`, `cargo clippy -- -D warnings`,
`cargo test --locked --offline`.

```bash
cp -R examples/polyglot/rust /tmp/rust-fix && cd /tmp/rust-fix
git init -b main && git config user.name demo && git config user.email demo@example.invalid
git add -A && git commit -m baseline
flow init --adopt-policy           # final CLI; see the fallback note in the fixture README
printf '\n// verified change\n' >> src/lib.rs
git add -A
flow run                            # PASSED: fmt, clippy, test
flow publish --expected-old "$(git rev-parse HEAD)"   # PUBLISHED
```

**MEASURED** on Windows 11 (cargo 1.95.0): `PASSED`, exit 0, three obligations
`PASSED`; `PUBLISHED`, exit 0, and the moved ref carries the receipt's tree. The
same sequence reported `PASSED`/`PUBLISHED` on WSL2 with cargo 1.94.1.

Red path: change `assert_eq!(add(2, 3), 5);` to `6`
([`rust/src/lib.rs:24`](../examples/polyglot/rust/src/lib.rs#L24)). Then
`git add -A`, `flow run` → `FAILED` exit 1 (`test` exit 101), `flow publish` →
`REFUSED` exit 1, code `REQUIRED_NOT_PASSED`, ref unchanged. **MEASURED**.

### 7.2 Node

Policy: `node --test` (built-in runner, no npm, no dependency).

```bash
cp -R examples/polyglot/node /tmp/node-fix && cd /tmp/node-fix
git init -b main && git config user.name demo && git config user.email demo@example.invalid
git add -A && git commit -m baseline
flow init --adopt-policy
printf '\n// verified change\n' >> src/math.js
git add -A
flow run                            # PASSED: node-test
flow publish --expected-old "$(git rev-parse HEAD)"   # PUBLISHED
```

**MEASURED** on Windows 11 (Node 26.13.0) and WSL2 (Node 20.20.2).

Red path: change `assert.equal(add(2, 3), 5);` to `6`
([`node/test/math.test.js:10`](../examples/polyglot/node/test/math.test.js#L10))
→ `FAILED` / `REFUSED`, ref unchanged. **MEASURED**.

### 7.3 Python

Policy: `python -m unittest discover -v` (standard library only).

```bash
cp -R examples/polyglot/python /tmp/python-fix && cd /tmp/python-fix
git init -b main && git config user.name demo && git config user.email demo@example.invalid
git add -A && git commit -m baseline
flow init --adopt-policy
printf '\n# verified change\n' >> src/math_util.py
git add -A
flow run                            # PASSED: unittest
flow publish --expected-old "$(git rev-parse HEAD)"   # PUBLISHED
```

**MEASURED** on Windows 11 (Python 3.13.7) and WSL2 (Python 3.12.3).

Red path: change `self.assertEqual(add(2, 3), 5)` to `6`
([`python/tests/test_math_util.py:13`](../examples/polyglot/python/tests/test_math_util.py#L13))
→ `FAILED` / `REFUSED`, ref unchanged. **MEASURED**.

On a host that only provides `python3`, the declared `python` program cannot
start and the obligation is `UNSUPPORTED`; change the program and re-adopt
instead of renaming the interpreter silently (see the fixture README).

## 8. The two-agent workflow (planned in this branch)

The final 0.1 surface is designed for two writers that never share a working
tree: each proposes a patch against an explicit base, the patches are assembled
into one candidate tree, the candidate is verified, and only then published.
The command shapes are:

```text
flow propose --agent A --task T1 --base <commit> --patch <file> [--key K] [--json]
flow proposals list [--json]
flow proposals show <id> [--json]
flow assemble --proposal <id> [--proposal <id>]... [--json]
flow run --candidate <id> [--obligation ID]... [--timeout S] [--json]
flow publish --candidate <id> --dry-run [--json]
flow publish --candidate <id> --expected-old <oid> [--json]
```

Intended sequence:

1. Agent A and agent B each produce a patch against the same known base commit.
   `propose` records the proposal with its agent and task and returns
   `PROPOSED`; `--key` is the caller-supplied key carried by the proposal
   (reserved for idempotent admission by the implementation being integrated).
2. `proposals list` / `proposals show <id>` inspect what was admitted. A
   proposal is immutable once admitted.
3. `assemble --proposal <id>...` builds a candidate tree from the declared base
   and the selected proposals. The candidate tree is the only content identity
   used afterwards.
4. `run --candidate <id>` verifies the immutable candidate tree (not the index),
   producing a receipt bound to that tree.
5. `publish --candidate <id> --dry-run` prints the plan — ref, old OID, tree,
   the commit that *would* be created, and the checks that passed — and writes
   nothing, exit 0, status `PLAN`.
6. `publish --candidate <id> --expected-old <oid>` performs the same ref CAS as
   §6 after re-checking the receipt and the policy.

**NOT_RUN**: none of `propose`, `proposals`, `assemble`, `run --candidate`,
`publish --candidate`, `publish --dry-run` exists in this worktree. The field
names of `flow-proposal/1` and `flow-candidate/1` are intentionally not
documented here — they are being defined by the implementation being integrated,
and this document does not invent them. See the status table.

## 9. A controlled collision, and its resolution

### 9.1 Two publishers, one ref (measured)

The publication gate is a compare-and-swap, so a collision is refused, not
silently merged:

```console
# both publishers verified distinct trees against main at the same old OID
$ flow publish --expected-old <old>          # first writer
{ "status": "PUBLISHED", "old": "<old>", "commit": "<new1>", ... }

$ flow publish --expected-old <old>          # second writer
{ "status": "REFUSED", "code": "STALE_REF",
  "reason": "stale expected old OID: ref is Some(\"<new1>\"), expected <old>" }
```

**MEASURED** (schematic: the refusals shown were executed, OIDs abbreviated): the
second writer gets `STALE_REF`, exit 1, ref unchanged. If the ref moves between
the intent record and the CAS, the second writer gets `REF_MOVED` and the journal
records an outcome, so recovery is possible (**CODE_READ**; the crash-window
path is exercised in §10 with `FLOW_TEST_CRASH=after-intent`).

Resolution: the loser does not force anything. It re-runs `flow run` on its new
base (the tree it wants to publish may now need a rebase), then publishes again
with the **new** expected-old. There is no automatic merge, no reset and no
silent overwrite.

### 9.2 Proposal-level collisions (planned)

When several proposals are assembled into one candidate, the assembler decides
whether they apply. A candidate that cannot be built from the declared base is a
refusal, not a best-effort merge. The exact conflict reporting belongs to the
`propose`/`assemble` implementation being integrated and is **NOT_RUN** here.

## 10. Crash recovery with `flow reconcile`

A publication is two journal records: an fsynced `intent` before the ref update,
an `outcome` after. If the process dies in between, `reconcile` observes the ref
and records what actually happened. It never retries the update.

Reproduced with the in-crate crash hook (`FLOW_TEST_CRASH=after-intent`, which
exits 89 between the intent and the CAS; **CODE_READ** and **MEASURED**):

```console
$ FLOW_TEST_CRASH=after-intent flow publish --expected-old <head>; echo $?
89

$ flow reconcile
{
  "status": "RECONCILED",
  "resolved": [
    {
      "id": "2026-11-09T20:22:19Z-66d8eb8a8570-1794255739651302000",
      "ref": "refs/heads/main",
      "observed": "52c552e296a89fda8d35356a254cd1702ad24928",
      "resolution": "NOT_PUBLISHED"
    }
  ],
  "manual_required": false,
  "note": "reconciliation never moves a ref: it only records what is observable"
}
```

exit 0; the journal gained
`{"event":"outcome","status":"RECONCILED_NOT_PUBLISHED",...}` and the ref is
still where it was. The next publication is a normal one.

If the journal itself is damaged, recovery refuses to guess (**MEASURED**):

```console
$ flow reconcile
{
  "status": "CORRUPT_JOURNAL",
  "code": "JOURNAL_CORRUPT",
  "error": "journal corrupt: torn or invalid line 1: unexpected end of input",
  "journal_path": ".../.flow/journal.jsonl",
  "journal_preserved": true,
  "manual_required": true,
  "resolved": [],
  "note": "recovery refuses to guess: the journal was not rewritten and no ref was moved"
}
```

exit 1, and `flow status` also fails closed. Inspect `.flow/journal.jsonl` by
hand: a torn line is not a successful empty journal.

## 11. Windows and Linux notes

- **CRLF.** Flow verifies the Git tree, not worktree bytes. On Windows, Git's
  `core.autocrlf` conversion can make the worktree differ from the tree; the
  demo scripts pin `core.autocrlf=false` in their disposable repository so the
  checkout and the tree match byte for byte. `.gitattributes` eol/filter rules
  can create the same worktree-versus-tree difference; `flow doctor` warns when
  `.gitattributes` exists because the receipt binds the tree OID, not worktree
  bytes.
- **PowerShell.** `run-demo.ps1` requires PowerShell 7 (`#requires -Version
  7.0`), sets `$ErrorActionPreference='Stop'`, and checks `$LASTEXITCODE`
  explicitly. On 7.3+, a native non-zero exit can become a terminating error;
  the script sets `$PSNativeCommandUseErrorActionPreference = $false` for itself
  when the variable exists, and inspects codes instead.
- **bash.** `run-demo.sh` starts with `set -euo pipefail`; every expected
  non-zero exit (the red run, the refusal) is captured explicitly with `if
  cmd; then ... else code=$?; fi`, never with `|| true`.
- **PATH.** Obligations are started by program name with the scrubbed
  environment. The toolchain must be on `PATH`, and the allowlist must still let
  it run: `CARGO_HOME`, `RUSTUP_HOME` and `USERPROFILE`/`HOME` are kept for
  rustup; `SystemRoot` and `PATHEXT` are kept on Windows.
- **Case sensitivity.** On a case-insensitive filesystem a tree containing two
  paths that differ only by case cannot be materialised faithfully; Flow refuses
  it (`UNSUPPORTED_CHECKOUT`). `flow doctor` reports `core.ignorecase`.
- **Long paths.** `flow doctor` warns when `core.longpaths` is not `true` on
  Windows; a path that the host cannot create makes the checkout unfaithful.
- **Symlinks.** A tree symlink checked out as a regular file (because
  `core.symlinks=false`) is refused rather than certified.
- **Line endings in scripts.** The PowerShell driver ran on Windows; the bash
  driver ran on Linux (WSL2), and its syntax was also checked with `bash -n`
  under Git Bash.

## 12. Troubleshooting

Codes are the stable, machine-readable `code` field of a refusal (`status:
REFUSED`) or the `status` of a receipt. Exit code is 0 only for a success status
(see the [exit-code table](CLI-JSON.md#2-exit-codes)).

| Symptom / code | Meaning | Action |
|---|---|---|
| `ERROR: not inside a Git work tree` (exit 1) | the current directory is not in a work tree | `cd` into the repository, or run `flow help`/`flow version`, which do not need a repository |
| `ERROR: unknown command` (exit 2) | typo | `flow help` |
| `UNKNOWN` | the policy declares no obligation | edit `.flow/config.json` to declare at least one required obligation |
| `NO_RECEIPT` | no complete receipt for the **current index tree** | `git add -A` then `flow run` (a selective run never qualifies) |
| `SCOPE_INCOMPLETE` | a receipt in the complete slot claims a non-complete scope (a selective receipt moved there) | delete it and re-run the whole policy |
| `STALE_POLICY` | config edited, or `--timeout` used at run time | re-run `flow run` without overrides after the intentional edit |
| `CONTEXT_MISMATCH` | Git/toolchain/platform changed between run and publish | re-run `flow run` |
| `POLICY_VERSION_MISMATCH`, `EXECUTION_BOUNDARY_MISMATCH` | the receipt proves a different policy version/boundary | re-run `flow run` under the current config |
| `RECEIPT_INTEGRITY` | the receipt file was edited or corrupted | delete it and re-run `flow run`; do not hand-edit receipts |
| `RECEIPT_SCHEMA_STALE` | receipt written by another schema version | re-run `flow run` |
| `REQUIRED_NOT_PASSED` | a required obligation is `FAILED`/`TIMEOUT`/`UNSUPPORTED` | fix the failure; a non-`PASSED` receipt is never publishable |
| `MISSING_REQUIRED` | the receipt does not cover a currently required obligation | re-run `flow run` on the current policy |
| `STALE_REF` | the ref no longer equals `--expected-old` | observe the ref, rebase/re-verify, publish with the new expected-old; never retry blindly |
| `REF_MOVED` | the ref moved between the intent and the CAS | nothing was published; `flow reconcile` records the outcome, then retry with fresh state |
| `ALREADY_PUBLISHED` | the ref already carries the verified tree | nothing to do; commit a change first |
| `REF_NOT_ALLOWED` | `--ref` is not a full `refs/heads/...` ref | pass e.g. `--ref refs/heads/main` |
| `LOCKED` | another Flow process holds `.flow/lock` | wait; remove the lock only if you are certain no Flow process runs |
| `DISABLED` | `flow disable` was used | `flow enable` |
| `INVALID_POLICY` | duplicate/unsafe obligation ids, or no required obligation | fix the config (ids key logs and receipts; they must be unique) |
| `UNKNOWN_OBLIGATION_ID`, `DUPLICATE_OBLIGATION_ID` | bad `--obligation` selection | list ids from `.flow/config.json` |
| `UNSUPPORTED_CHECKOUT` | the host cannot materialise the tree faithfully | case collision, symlink/gitlink; `flow doctor` reports the cause |
| `TIMEOUT` | an obligation exceeded its deadline (default 300 s) | raise `timeout_s`, or make the check faster |
| `UNSUPPORTED` | an obligation process could not start | check `PATH` and the program name (e.g. `python` versus `python3`) |
| `PARTIAL` | diagnostic run | re-run the whole policy before publishing |
| `CORRUPT_JOURNAL` | a journal line is not valid JSON | inspect `.flow/journal.jsonl`; it was preserved, not rewritten |
| `UNKNOWN_OUTCOME` | the journal could not be read at all | check file permissions; do not assume nothing was published |
| `NEEDS_RECONCILIATION` | the ref matches neither the old nor the new OID of an intent | observe the ref by hand; a human decision is required |
| `detached HEAD: pass --ref explicitly` (exit 1, `ERROR`) | `publish` cannot infer a branch | `git switch -c <branch>` or pass `--ref refs/heads/<branch>` |

## 13. Measured evidence summary

All of the following were executed on this host (Windows 11, Git 2.51.1) with
the binary built from this worktree; Linux results are WSL2 local runs, not CI:

- three fixtures: direct obligations PASS, `flow run` `PASSED`,
  `flow publish` `PUBLISHED`, red edit `FAILED` + `REFUSED`, ref unchanged;
- `init`, `init` twice, `doctor`, `status`, `version`, `help`, `explain`
  (default and `--tree`), `reconcile` (clean, after a crash hook, corrupt
  journal), `disable`/`enable`;
- refusals: `REF_NOT_ALLOWED`, `STALE_REF`, `ALREADY_PUBLISHED`,
  `STALE_POLICY` (config edit and `--timeout`), `RECEIPT_INTEGRITY`,
  `RECEIPT_SCHEMA_STALE`, `NO_RECEIPT`, `REQUIRED_NOT_PASSED`,
  `UNKNOWN_OBLIGATION_ID`, `DISABLED`, `CORRUPT_JOURNAL`;
- receipt statuses: `PASSED`, `PARTIAL`, `FAILED`, `TIMEOUT`, `UNKNOWN`;
- usage/error paths: no arguments (exit 2), unknown command inside a work tree
  (exit 2) and outside one (exit 1), detached-HEAD publish (exit 1).

## 14. STATUS: PENDING in this worktree

Everything below is part of the 0.1 surface being integrated in parallel. None
of it exists at branch point `d884e29`; nothing in this guide claims it was run.

| Surface | Expected status / schema | State here |
|---|---|---|
| `flow init --adopt-policy` (adoption gate for a shipped config) | `INITIALIZED`; refusal without the flag | NOT_RUN (flag absent; `init` refuses an existing `.flow/`) |
| `flow approve` | `APPROVED`, exit 0 | NOT_RUN (command absent) |
| `--json` on `init`/`doctor`/`status`/`run`/`publish`/`reconcile`/`explain` | JSON is already the only output format here; the flag is accepted and ignored by the pre-0.1 parsers | NOT_RUN (flag has no effect; behaviour not verified) |
| `flow propose`, `flow proposals list`, `flow proposals show` | `PROPOSED`, schema `flow-proposal/1` | NOT_RUN (absent) |
| `flow assemble` | `ASSEMBLED`, schema `flow-candidate/1` | NOT_RUN (absent) |
| `flow run --candidate <id>` | receipt for the immutable candidate tree | NOT_RUN (absent) |
| `flow publish --candidate <id>` | `PUBLISHED` for that candidate | NOT_RUN (absent) |
| `flow publish --dry-run` | `PLAN`, exit 0, no ref/commit written, schema `flow-plan/1` | NOT_RUN (absent; the flag is ignored by the old parser, so do not use it) |
| `flow explain --candidate <id>` | `EXPLAIN` for a candidate | NOT_RUN (absent) |
| `flow status --json` / schema `flow-status/1` | status document | NOT_RUN (status exists, schema name does not) |
| `flow-proposal/1`, `flow-candidate/1`, `flow-plan/1`, `flow-status/1` | JSON schemas | NOT_RUN (not defined in this worktree) |
| exit codes for `PLAN`, `PROPOSED`, `ADMITTED`, `ASSEMBLED`, `APPROVED`, `STATUS` | 0 | NOT_RUN (not reachable here) |
| CI on Ubuntu/Windows for these fixtures | — | NOT_RUN (the CI matrix is owned by the integration branch) |

## 15. Where to go next

- [CLI and JSON reference](CLI-JSON.md) — every command, field, exit code and
  schema-versioning rule, with real examples and explicit `NOT_RUN` markers.
- [Polyglot fixtures](../examples/polyglot/README.md) — the three repositories
  used in §7 and their demo drivers.
- [Architecture](ARCHITECTURE.md) and the
  [P0 hardening report](reviews/FLOW-FOUNDATION-P0-HARDENING-REPORT.md) — why the
  gate is shaped this way and what remains unproven.
