# Flow CLI and JSON reference

**Scope:** the CLI that exists in this worktree at branch point `d884e29`
(`crate flow 0.1.0`, policy `FLOW-POLICY-1`). The final MVP 0.1 surface
(`propose`, `proposals`, `assemble`, `run --candidate`, `publish --candidate`,
`publish --dry-run`, `init --adopt-policy`, `approve`, the `--json` flags and
the `flow-proposal/1` / `flow-candidate/1` / `flow-plan/1` / `flow-status/1`
schemas) is being implemented in parallel and is **NOT_RUN** here; see §10.

**How to read the labels:**

- **MEASURED** — the command was executed on this host and the example is copied
  from its output (values such as OIDs and timestamps differ per run).
- **CODE_READ** — behaviour read in `crates/flow/src/**`, not necessarily
  exercised.
- **NOT_RUN** — not executable in this worktree; no field list is invented.

Outputs marked *(abridged)* keep the real field names and values but omit
sections for length; they are not byte-for-byte copies.

## 1. Conventions

1. Every command writes **one JSON document to stdout**, pretty-printed with
   two-space indentation, **except** `flow help` and a bare `flow` invocation,
   which print the plain-text usage block. `flow version` is JSON.
2. Three shapes exist:
   - success: an object whose first field is `status`, e.g.
     `{"status": "PASSED", ...}`;
   - refusal: `{"status": "REFUSED", "code": "<STABLE_CODE>", "reason": "..."}`
     for the gate, or `{"status": "REFUSED", "reason": "..."}` for `init` (which
     has no machine code);
   - error: `{"status": "ERROR", "error": "..."}` — bad usage, not inside a Git
     work tree, unreadable state, or an unexpected I/O failure.
3. `flow init`, `flow doctor`, `flow status`, `flow run`, `flow publish`,
   `flow reconcile`, `flow explain`, `flow disable`, `flow enable` require a Git
   work tree. `flow version` and `flow help` do not.
4. The `--json` flag is **planned in this branch**: the current binary always
   emits JSON and its parsers ignore unknown flags, so passing `--json` neither
   helps nor fails. Do not infer that the planned flag is implemented.
5. Receipt files are stored at `.flow/receipts/<tree_oid>.json`; selective runs
   at `.flow/receipts/<tree_oid>.partial.<context-prefix>.json`. Logs at
   `.flow/logs/<tree_oid>/<obligation-id>.<stdout|stderr>.log`. Journal at
   `.flow/journal.jsonl`. Lock at `.flow/lock`.

## 2. Exit codes

The dispatch maps an allowlist of status strings to 0 and **everything else to
1** (*fail closed*); 2 is reserved for usage errors (`CODE_READ`:
`main.rs::exit_code_for`).

| Exit | Statuses | Meaning |
|---|---|---|
| 0 | `VERSION`, `REPORT` (doctor/status), `INITIALIZED`, `PASSED`, `PUBLISHED`, `RECONCILED`, `EXPLAIN`, `DISABLED`, `ENABLED`; `help` text | the command did what it says |
| 1 | `REFUSED`, `FAILED`, `TIMEOUT`, `PARTIAL`, `UNSUPPORTED`, `UNKNOWN`, `ERROR`, `CORRUPT_JOURNAL`, `UNKNOWN_OUTCOME`, `NEEDS_RECONCILIATION`; `INCOMPLETE` | the operation did not succeed; details in `code` / `reason` / `error` |
| 2 | usage: no arguments, or an unknown command **inside** a work tree | command line is wrong |

All statuses above are reachable in this worktree. `INCOMPLETE`,
`UNKNOWN_OUTCOME` and `NEEDS_RECONCILIATION` are **CODE_READ** only (no example
below); every other status appears in a measured example in §5.

Two quirks, both **MEASURED**:

- `flow` with no arguments prints the usage text (not JSON) and exits **2**.
- `flow bogus` **outside** a Git work tree exits **1** with
  `{"status":"ERROR","error":"not inside a Git work tree"}`: the repository
  lookup happens before command dispatch. The same typo **inside** a work tree
  exits **2** with `{"status":"ERROR","error":"unknown command \"bogus\" (run
  `flow help`)"}`.

Planned statuses `PLAN`, `PROPOSED`, `ADMITTED`, `ASSEMBLED`, `APPROVED`,
`STATUS` are specified to exit 0; **NOT_RUN** here.

## 3. `flow version`

```text
flow version | flow --version
```

| Field | Type | Meaning |
|---|---|---|
| `status` | string | `VERSION` |
| `tool` | string | `flow <crate version>` |
| `policy_version` | string | policy vocabulary of this build (`FLOW-POLICY-1`) |
| `git` | string | first line of `git --version` |
| `schema` | string | receipt schema this build writes (`flow-receipt/2`) |

**MEASURED** (works outside a repository, exit 0):

```console
$ flow version
{
  "status": "VERSION",
  "tool": "flow 0.1.0",
  "policy_version": "FLOW-POLICY-1",
  "git": "git version 2.51.1.windows.1",
  "schema": "flow-receipt/2"
}
```

## 4. `flow help`

```text
flow help | flow --help | flow -h
```

Prints the usage block as plain text and exits 0 (**MEASURED**). A bare `flow`
prints the same block and exits 2. The block lists:

```text
  init [--force]
  doctor
  status
  run [--obligation ID]... [--timeout S]
  publish [--ref REF] [--expected-old OID] [--message M]
  reconcile
  explain [--tree OID]
  disable | enable
  version
  help
```

## 5. Commands

### 5.1 `flow init`

```text
flow init [--force]
```

Creates `.flow/` (`config.json` with an empty policy, `journal.jsonl`,
`receipts/`, `logs/`, and `.flow/.gitignore` = `*`). `--force` rewrites
`config.json` with the empty default and preserves an existing journal
(**CODE_READ**). Planned in the final surface: `--adopt-policy` for a repository
that ships a config; **NOT_RUN** here.

| Field | Type | Meaning |
|---|---|---|
| `status` | string | `INITIALIZED` or `REFUSED` |
| `flow_dir` | string | always `.flow` on success |
| `policy_version` | string | default policy vocabulary |
| `note` | string | what to do next |
| `reason` | string | refusal only: `.flow` already exists |

Exit: 0 on `INITIALIZED`; 1 on `REFUSED` and on `ERROR` (I/O failure).

**MEASURED** — first run (exit 0):

```console
$ flow init
{
  "status": "INITIALIZED",
  "flow_dir": ".flow",
  "policy_version": "FLOW-POLICY-1",
  "note": "no obligation declared yet: `flow run` will refuse with UNKNOWN until .flow/config.json lists obligations; .flow/.gitignore keeps Flow state out of the verified tree"
}
```

**MEASURED** — second run (exit 1; note: no `code` field):

```console
$ flow init
{
  "status": "REFUSED",
  "reason": ".flow already exists (pass --force to rewrite config, journal is preserved)"
}
```

### 5.2 `flow doctor`

```text
flow doctor
```

Read-only environment and guarantee report. No arguments, no side effects.
`status` is `REPORT`; `overall` is `FAILED` if any check reports `FAILED`,
otherwise `OK`.

| Field | Type | Meaning |
|---|---|---|
| `status` | string | `REPORT` |
| `overall` | string | `OK` or `FAILED` |
| `checks[]` | array | `{id, status, detail}`; `status` is `OK`, `WARN` or `FAILED` |
| `guarantees[]` | array of strings | what the gate does guarantee |
| `non_guarantees[]` | array of strings | what it explicitly does not |

Check ids: `git`, `repository`, `flow_initialized`, `hooks`, `longpaths`,
`case_sensitivity`, `gitattributes`, `toolchains`, `execution_boundary`,
`platform`, `disabled`, `journal`.

**MEASURED** — a repository without `.flow/` (exit 0; abridged):

```console
$ flow doctor
{
  "status": "REPORT",
  "overall": "OK",
  "checks": [
    { "id": "flow_initialized", "status": "WARN", "detail": "run `flow init`" },
    { "id": "hooks", "status": "OK", "detail": "no hook installed. ..." },
    ...
  ],
  "guarantees": [ ... ],
  "non_guarantees": [ ... ]
}
```

**MEASURED** — with an installed hook (exit 0; abridged):

```json
{ "id": "hooks", "status": "WARN",
  "detail": "hooks present (pre-commit): Flow dispatches none of its own, but the Git plumbing it calls runs post-checkout on `worktree add` and reference-transaction on `update-ref`, and a blocking reference-transaction hook aborts publish. Obligations are declared in .flow/config.json" }
```

**MEASURED** — Windows reference host: `longpaths` and `case_sensitivity` are
`WARN`, `execution_boundary` and `platform` are `WARN` (no sandbox; Windows
containment not qualified), `toolchains` is `OK` with
`cargo=found rustc=found node=found python=found python3=found`.

### 5.3 `flow run`

```text
flow run [--obligation ID]... [--timeout S]     # this worktree
flow run [--candidate <id>] [--obligation ID]... [--timeout S] [--json]   # planned
```

Verifies the **Git index tree** (stage first with `git add`; the worktree is
never modified). Without `--candidate` the tree is `git write-tree`; the planned
`--candidate` form verifies the immutable candidate tree instead (**NOT_RUN**
here).

The output of a completed run **is the receipt**, with one extra field,
`receipt_path`, added for the caller; the file on disk does not contain it.
Refusals and `UNKNOWN` use the shapes in §1 instead.

| Field | Type | Meaning |
|---|---|---|
| `schema` | string | `flow-receipt/2` |
| `tree_oid` | string | the verified tree (index tree here) |
| `commit_oid` | string | verification commit created to materialise the tree |
| `parent_oid` | string | repository `HEAD` at run time |
| `context_id` | string | SHA-256 over `flow-context/2` inputs: tree, effective policy, Git version, OS, architecture |
| `policy_digest` | string | SHA-256 of the effective policy object |
| `policy` | object | effective policy: `policy_version`, `execution_boundary`, `obligations[]` (`id`, `program`, `args`, `required`, `timeout_s`), `obligation_env[]` (the allowlist) |
| `policy_version` | string | `config.policy_version` |
| `execution_boundary` | string | `config.execution_boundary` (`local-trusted`) |
| `status` | string | `PASSED`, `PARTIAL`, `FAILED`, `TIMEOUT`, `UNSUPPORTED`, or `INCOMPLETE` (defined in code, not observed) |
| `verification_scope` | object | `mode` (`complete`/`selected`), `declared[]`, `required[]`, `selected[]`, `executed[]`, `missing_required[]` |
| `started_at`, `finished_at` | string | UTC ISO-8601, `...Z` |
| `obligations[]` | array | `id`, `status` (`PASSED`/`FAILED`/`TIMEOUT`/`UNSUPPORTED`), `required`, `exit` (number or null), `duration_ms`, `timed_out`, `stdout_truncated`, `stderr_truncated`, `stdout_preview`, `stderr_preview` (≤400 characters) |
| `logs[]` | array | relative paths `.flow/logs/<tree>/<id>.<stream>.log` |
| `worktree_diff[]` | array | paths reported by `git diff --name-only <tree>` at run time (usually empty) |
| `scope_note` | string | what the receipt does and does not prove |
| `integrity` | string | sealed SHA-256 over the body |
| `receipt_path` | string | CLI output only; absolute machine-local path (on Windows it can carry a `\\?\` prefix) |

Selective run output (`PARTIAL`, exit 1) is stored in a different file
(`...partial.<context>.json`) and can never shadow or replace a complete receipt.

**MEASURED** — full policy passed (exit 0; abridged):

```console
$ flow run
{
  "schema": "flow-receipt/2",
  "tree_oid": "e1e3809858cf380de1106a81f6f75de9b132d091",
  "commit_oid": "579a03bdba41326de60f9daf6f1e928675fac3f9",
  "parent_oid": "ce1ef7c4dd55262bb078c3bc9a865a550bf8e559",
  "context_id": "7eebace21cfaf5dadd352aa8d10bda53d1681b9a08a22bfb0aaa028dc748e0bb",
  "policy_digest": "0571e4d9b636a6a04d8473f90ef97b4bb49677a7f8f4c26f346e2969b6bbb6a7",
  "policy": { ... },
  "policy_version": "FLOW-POLICY-1",
  "execution_boundary": "local-trusted",
  "status": "PASSED",
  "verification_scope": {
    "mode": "complete",
    "declared": ["fmt", "clippy", "test"],
    "required": ["fmt", "clippy", "test"],
    "selected": ["fmt", "clippy", "test"],
    "executed": ["fmt", "clippy", "test"],
    "missing_required": []
  },
  "obligations": [
    { "id": "fmt", "status": "PASSED", "required": true, "exit": 0, "duration_ms": 150, "timed_out": false, ... },
    { "id": "clippy", "status": "PASSED", "required": true, "exit": 0, "duration_ms": 230, "timed_out": false, ... },
    { "id": "test", "status": "PASSED", "required": true, "exit": 0, "duration_ms": 967, "timed_out": false, ... }
  ],
  "logs": [ ... ],
  "worktree_diff": [],
  "scope_note": "verified for the declared obligations only; undeclared inputs are not covered",
  "integrity": "d1449100028c3813bc83e56d113ab4d84b4002de9174b1f5c989c6b01c465a2e",
  "receipt_path": "...\\.flow\\receipts\\e1e3809858cf380de1106a81f6f75de9b132d091.json"
}
```

**MEASURED** — selective run (exit 1; abridged):

```console
$ flow run --obligation test
{
  "schema": "flow-receipt/2",
  "status": "PARTIAL",
  "verification_scope": {
    "mode": "selected",
    "declared": ["fmt", "clippy", "test"],
    "required": ["fmt", "clippy", "test"],
    "selected": ["test"],
    "executed": ["test"],
    "missing_required": ["fmt", "clippy"]
  },
  "scope_note": "diagnostic run: only the selected obligations were executed, so this receipt proves nothing about the rest of the policy and is never publishable",
  ...
}
```

**MEASURED** — a failing obligation (exit 1; abridged):

```json
{ "status": "FAILED",
  "obligations": [
    { "id": "fmt", "status": "PASSED", "exit": 0, ... },
    { "id": "clippy", "status": "PASSED", "exit": 0, ... },
    { "id": "test", "status": "FAILED", "exit": 101, ... }
  ] }
```

**MEASURED** — empty policy and bad selection:

```console
$ flow run
{ "status": "UNKNOWN", "reason": "no obligation declared: verification scope is undefined, nothing is publishable" }

$ flow run --obligation nope
{ "status": "REFUSED", "code": "UNKNOWN_OBLIGATION_ID", "reason": "obligation \"nope\" is not declared in .flow/config.json" }
```

**MEASURED** — unstartable program → status `UNSUPPORTED`, exit 1, obligation
`exit: null`, preview `cannot start "definitely-not-installed-xyz": program not
found`.

`run` refusals (all exit 1): `DISABLED`, `INVALID_POLICY`,
`UNKNOWN_OBLIGATION_ID`, `DUPLICATE_OBLIGATION_ID`, `SCRATCH_REFUSED`,
`UNSUPPORTED_CHECKOUT`.

### 5.4 `flow publish`

```text
flow publish [--ref REF] [--expected-old OID] [--message M]     # this worktree
flow publish [--candidate <id>] [--ref refs/heads/x] [--expected-old OID] \
             [--message M] [--dry-run] [--json]                  # planned
```

Publishes the **complete `PASSED`** receipt for the current index tree (or, in
the planned form, for the candidate tree) behind a ref compare-and-swap. The
rules and refusal order are in `cmd_publish` (**CODE_READ**), and the measured
outcomes below confirm them.

Success fields:

| Field | Type | Meaning |
|---|---|---|
| `status` | string | `PUBLISHED` |
| `ref` | string | the branch ref that moved |
| `old` | string | expected old OID, now the commit's parent |
| `commit` | string | new commit OID |
| `tree` | string | verified tree carried by `commit` |
| `context_id` | string | receipt context |
| `policy_digest` | string | receipt policy digest |
| `policy_version` | string | receipt policy version |
| `execution_boundary` | string | receipt execution boundary |
| `receipt_logs` | number | number of log paths in the receipt |
| `receipt_started_at`, `receipt_finished_at` | string | receipt timestamps |
| `receipt_commit` | string | the verification commit from the receipt |
| `obligations` | array of strings | ids of the obligations the receipt covers |
| `worktree_diff` | array | copied from the receipt |
| `note` | string | "only refs were moved; the working tree was not modified and may now differ" |

Refusal shape: `{"status":"REFUSED","code":"<CODE>","reason":"..."}`, exit 1.
The commit object may already exist when the CAS refuses (for example
`REF_MOVED`), in which case it is left unreachable; no ref moved.

**MEASURED** — success (exit 0):

```console
$ flow publish --expected-old c450c8ffce2ee7c9e207038593fe1ec2c3aa4043
{
  "status": "PUBLISHED",
  "ref": "refs/heads/main",
  "old": "c450c8ffce2ee7c9e207038593fe1ec2c3aa4043",
  "commit": "1ba6c3bc24c2963e635383de6f495319ad61567f",
  "tree": "1d2d22a245481fb07e2ca9a3ebadb76473d6edcb",
  "context_id": "1ed7a1afa4d93c0d9092a2128dee6acf6472c72d241140035f1ad740b0b046ca",
  "policy_digest": "0571e4d9b636a6a04d8473f90ef97b4bb49677a7f8f4c26f346e2969b6bbb6a7",
  "policy_version": "FLOW-POLICY-1",
  "execution_boundary": "local-trusted",
  "receipt_logs": 6,
  "receipt_started_at": "2026-11-09T20:22:13Z",
  "receipt_finished_at": "2026-11-09T20:22:15Z",
  "receipt_commit": "872c6da33a1301d24e69fc2e59300fba947423f0",
  "obligations": ["fmt", "clippy", "test"],
  "worktree_diff": [],
  "note": "only refs were moved; the working tree was not modified and may now differ"
}
```

**MEASURED** — the ref moved (exit 1):

```console
$ flow publish --expected-old 0000000000000000000000000000000000000000
{
  "status": "REFUSED",
  "code": "STALE_REF",
  "reason": "stale expected old OID: ref is Some(\"ce1ef7c4dd55262bb078c3bc9a865a550bf8e559\"), expected 0000000000000000000000000000000000000000"
}
```

**MEASURED** — a red edit (exit 1):

```console
$ flow publish --expected-old <head>
{
  "status": "REFUSED",
  "code": "REQUIRED_NOT_PASSED",
  "reason": "required obligation \"test\" has status \"FAILED\""
}
```

**MEASURED** — a `--timeout` override at run time or an edited config (exit 1):

```json
{ "status": "REFUSED", "code": "STALE_POLICY",
  "reason": "the receipt proves policy e165e97fb473 but the current policy is 0571e4d9b636: any edit to .flow/config.json, and any `--timeout` override at run time, changes the proven policy - re-run `flow run` without overrides" }
```

Refusal codes, in the order the gate checks them (**CODE_READ**; measured ones
marked):

| Order | Code | Cause | Measured |
|---|---|---|---|
| 1 | `DISABLED` | `flow disable` is in effect | yes |
| 2 | `INVALID_POLICY` | current config has duplicate/unsafe ids or no required obligation | no |
| 3 | `REF_NOT_ALLOWED` | `--ref` is not a full `refs/heads/...` ref | yes |
| 4 | `LOCKED` | another Flow process holds `.flow/lock` | no |
| 5 | `NO_RECEIPT` | no complete receipt for the current index tree | yes |
| 6 | `RECEIPT_SCHEMA_STALE` | receipt schema is not `flow-receipt/2` | yes |
| 7 | `RECEIPT_INTEGRITY` | receipt body does not match its `integrity` digest | yes |
| 8 | `TREE_MISMATCH` | the receipt file names another tree | no |
| 9 | `SCOPE_INCOMPLETE` | a receipt in the complete slot declares a non-complete scope | no |
| 10 | `MISSING_REQUIRED` | a required obligation is absent from the receipt | no |
| 11 | `STALE_POLICY` | current effective policy digest differs from the receipt's | yes |
| 12 | `POLICY_VERSION_MISMATCH` | policy version changed | no |
| 13 | `EXECUTION_BOUNDARY_MISMATCH` | execution boundary changed | no |
| 14 | `CONTEXT_MISMATCH` | Git version / OS / arch changed since the run | no |
| 15 | `REQUIRED_NOT_PASSED` | a required obligation is not `PASSED` | yes |
| 16 | `STATUS_NOT_PASSED` | receipt status is not `PASSED` | no |
| 17 | `RECEIPT_COMMIT_MISMATCH` | the verification commit now carries a different tree | no |
| 18 | `UNKNOWN_REF` | the ref does not resolve | no |
| 19 | `STALE_REF` | current ref OID differs from `--expected-old` | yes |
| 20 | `ALREADY_PUBLISHED` | the ref already carries this tree | yes |
| 21 | `COMMIT_TREE_MISMATCH` | the prepared commit does not carry the verified tree | no |
| 22 | `REF_MOVED` | the ref moved between intent and CAS | no |
| 23 | `UPDATE_REF_REFUSED` | `git update-ref` rejected the CAS | no |

Errors (exit 1, `status: ERROR`): detached `HEAD` without `--ref`
(`detached HEAD: pass --ref explicitly`, measured), a `git write-tree` failure,
or an unreadable config.

`--dry-run` (planned): prints a `flow-plan/1` document, writes no ref and no
commit, exits 0 with status `PLAN`. **NOT_RUN** here; the old parser ignores the
flag, so a publish would be real.

### 5.5 `flow reconcile`

```text
flow reconcile
```

Resolves journal intents that have no outcome, by observing refs. Never moves a
ref, never rewrites the journal. Exit 0 only when the status is `RECONCILED`.

| Field | Type | Meaning |
|---|---|---|
| `status` | string | `RECONCILED`, `NEEDS_RECONCILIATION`, `CORRUPT_JOURNAL`, `UNKNOWN_OUTCOME` |
| `resolved[]` | array | per intent: `{id, ref, observed, resolution}` when decisive; `{id, resolution, reason}` or `{id, ref, resolution, reason}` when not |
| `manual_required` | boolean | true when a human must decide |
| `note` | string | always: reconciliation observes, it does not move refs |
| `code`, `error`, `journal_path`, `journal_preserved` | fields | only on the journal-failure statuses |

**MEASURED** — nothing pending (exit 0):

```console
$ flow reconcile
{
  "status": "RECONCILED",
  "resolved": [],
  "manual_required": false,
  "note": "reconciliation never moves a ref: it only records what is observable"
}
```

**MEASURED** — after a crash between intent and CAS (exit 0; see §8 for the
journal records):

```console
$ FLOW_TEST_CRASH=after-intent flow publish --expected-old <head>   # exit 89
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

**MEASURED** — corrupt journal (exit 1; the journal is preserved, not
rewritten):

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

A journal that cannot be opened at all is `UNKNOWN_OUTCOME` with code
`JOURNAL_UNREADABLE` and the same fail-closed fields (**CODE_READ**).

`resolved[].resolution` values: `PUBLISHED` (ref already equals the intent's
new OID), `NOT_PUBLISHED` (ref still equals the old OID), or
`NEEDS_RECONCILIATION` (neither; `reason` is `DUPLICATE_INTENT_ID`,
`INCOMPLETE_INTENT`, `REF_ABSENT`, or `REF_UNREADABLE`). Only a decisive
observation appends a `RECONCILED_*` outcome record.

### 5.6 `flow explain`

```text
flow explain [--tree OID]        # planned: [--candidate <id>]
```

Returns the receipt (if any) and journal records for a tree. Without `--tree`
the current `HEAD` tree is used. Does not require a receipt to exist.

| Field | Type | Meaning |
|---|---|---|
| `status` | string | `EXPLAIN` |
| `tree` | string | the tree asked about |
| `receipt` | object or null | the receipt document as stored (no `receipt_path` field) |
| `journal[]` | array | journal records whose `tree` equals `tree` |

Exit 0 on `EXPLAIN`; 1 if the receipt file is invalid JSON or the journal is
unreadable (`ERROR`).

**MEASURED** — a tree with no receipt:

```console
$ flow explain
{
  "status": "EXPLAIN",
  "tree": "67b91b9fb2cc4ca52a401e8a48afdf3540d015a1",
  "receipt": null,
  "journal": []
}
```

**MEASURED** — `--tree` for a published tree (exit 0; abridged):

```console
$ flow explain --tree 6b3f02484358afe092f061f9589cb4a5d714dcb4
{
  "status": "EXPLAIN",
  "tree": "6b3f02484358afe092f061f9589cb4a5d714dcb4",
  "receipt": {
    "schema": "flow-receipt/2",
    "status": "PASSED",
    ...
  },
  "journal": [ ... ]
}
```

### 5.7 `flow disable` / `flow enable`

```text
flow disable
flow enable
```

Flips `disabled` in `.flow/config.json` and persists it. While disabled, `run`
and `publish` refuse with code `DISABLED`; `status`/`doctor`/`explain` still
work. Requires an existing config (otherwise `ERROR`).

| Field | Type | Meaning |
|---|---|---|
| `status` | string | `DISABLED` or `ENABLED` |
| `disabled` | boolean | the new state |

**MEASURED** (exit 0 each):

```console
$ flow disable
{ "status": "DISABLED", "disabled": true }

$ flow run
{ "status": "REFUSED", "code": "DISABLED", "reason": "Flow is disabled in this repository (`flow enable` to re-enable)" }

$ flow publish
{ "status": "REFUSED", "code": "DISABLED", "reason": "Flow is disabled in this repository" }

$ flow enable
{ "status": "ENABLED", "disabled": false }
```

### 5.8 `flow status`

```text
flow status                       # this worktree
flow status [--json]              # planned; schema flow-status/1
```

| Field | Type | Meaning |
|---|---|---|
| `status` | string | `REPORT`, or `ERROR` if the journal cannot be read |
| `branch` | string | short branch name (empty when detached) |
| `head` | string | `HEAD` commit OID |
| `tree` | string | `HEAD` tree OID |
| `dirty_count` | number | non-untracked lines in `git status --porcelain` |
| `untracked_count` | number | `??` lines in `git status --porcelain` |
| `obligations_declared` | number | obligations in `.flow/config.json` (0 if the config is unreadable) |
| `disabled` | boolean | current `disabled` flag |
| `latest_receipt` | object or null | `tree_oid`, `status`, `scope`, `context_id`, `finished_at` of the **lexicographically last** receipt file in `.flow/receipts/` — this is not a chronological guarantee, and selective receipts are included |
| `pending_intents` | number | intents not yet closed by an outcome (counted per id, not set membership) |
| `hint` | string | reminder that `run` verifies the index tree |

Exit 0 on `REPORT`; 1 on `ERROR` (measured: corrupt journal →
`{"status":"ERROR","error":"journal corrupt: ..."}`, which prevents reporting
`pending_intents: 0` as if nothing happened).

**MEASURED** — clean repository:

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

**MEASURED** — with a receipt and a reconciled crash (exit 0; abridged):

```json
{
  "status": "REPORT",
  "branch": "main",
  "head": "52c552e296a89fda8d35356a254cd1702ad24928",
  "dirty_count": 1,
  "latest_receipt": {
    "tree_oid": "1d2d22a245481fb07e2ca9a3ebadb76473d6edcb",
    "status": "PASSED",
    "scope": "complete",
    "context_id": "1ed7a1afa4d93c0d9092a2128dee6acf6472c72d241140035f1ad740b0b046ca",
    "finished_at": "2026-11-09T20:22:19Z"
  },
  "pending_intents": 0
}
```

## 6. Receipt schema `flow-receipt/2`

Written by every `run`; the file is `flow-receipt/2` with the fields in §5.3,
without `receipt_path`. The document is sealed: `integrity` is the SHA-256 of
Flow's **compact** JSON encoding of the body (`Json::to_string`, field order as
written) with the `integrity` field removed. The file itself is written
pretty-printed; whitespace in the file is not part of the digest, but any change
to a parsed value is. This detects corruption and casual edits; it is **not a
signature** — whoever can write `.flow/receipts` can forge a receipt.

`status` is a property of the run, not a claim of general correctness:

| `status` | Meaning |
|---|---|
| `PASSED` | the declared policy ran completely and every required obligation passed |
| `PARTIAL` | selective run; never publishable |
| `FAILED` | at least one required obligation failed |
| `TIMEOUT` | at least one required obligation was killed at its deadline |
| `UNSUPPORTED` | at least one required obligation could not be started |
| `INCOMPLETE` | required obligations were not selected (defined; not observed in this worktree) |

`worktree_diff` is the output of `git diff --name-only <tree>` at run time. It is
usually empty; a non-empty list means the worktree differs from the verified
tree (for example a checkout conversion) and is recorded rather than hidden.

## 7. Config schema `flow-config/1`

`.flow/config.json`:

| Field | Type | Default | Meaning |
|---|---|---|---|
| `schema` | string | required | must be exactly `flow-config/1`; anything else is an error |
| `policy_version` | string | `FLOW-POLICY-1` | policy vocabulary; part of the effective policy |
| `execution_boundary` | string | `local-trusted` | part of the effective policy; the only qualified value is `local-trusted` |
| `obligations[]` | array | `[]` | each: `id` (string, non-empty, unique, path-safe), `program` (string, non-empty), `args` (array of strings), `required` (boolean, default `true`), `timeout_s` (number, default `300`) |
| `disabled` | boolean | `false` | when true, `run` and `publish` refuse |

Validation at `run`/`publish` (`INVALID_POLICY`): duplicate `id`, an `id` that is
not a safe relative path (it keys `.flow/logs/...`), or a policy with **no**
required obligation (nothing could be proven, so nothing would be publishable).
Unknown fields are ignored; a different `schema` is refused (**CODE_READ**).
Duplicate JSON keys are refused at any depth (the parser rejects ambiguity;
covered by the P0 hardening suite).

The **effective policy** is the object hashed into `policy_digest`:
`policy_version`, `execution_boundary`, the normalized `obligations`, and
`obligation_env` (the environment allowlist). Do not compute it by hand; ask the
CLI (it appears verbatim in the receipt) if you need the identity.

## 8. Journal schema `flow-journal/1`

Append-only, one JSON object per line, fsynced before the ref update (intent)
and after it (outcome). Every record written carries `"schema":
"flow-journal/1"`. Recovery parses each line as JSON; a torn line is
`CORRUPT_JOURNAL` (never skipped). The reader does not enforce the schema tag on
existing records in this version; treat it as informational until the planned
versioned reader lands (**CODE_READ**).

`intent` (**MEASURED**):

```json
{"schema":"flow-journal/1","event":"intent","id":"2026-11-09T20:22:19Z-66d8eb8a8570-1794255739651302000","ref":"refs/heads/main","old":"52c552e296a89fda8d35356a254cd1702ad24928","new":"66d8eb8a8570d72f7faef457d144e93ac04b15b2","tree":"1d2d22a245481fb07e2ca9a3ebadb76473d6edcb","at":"2026-11-09T20:22:19Z"}
```

`outcome` after a successful CAS (**MEASURED**):

```json
{"schema":"flow-journal/1","event":"outcome","id":"2026-11-09T20:22:19Z-66d8eb8a8570-1794255739651302000","status":"PUBLISHED","commit":"66d8eb8a8570d72f7faef457d144e93ac04b15b2","at":"2026-11-09T20:22:19Z"}
```

`outcome` after reconciliation (**MEASURED**):

```json
{"schema":"flow-journal/1","event":"outcome","id":"2026-11-09T20:22:19Z-66d8eb8a8570-1794255739651302000","status":"RECONCILED_NOT_PUBLISHED","observed":"52c552e296a89fda8d35356a254cd1702ad24928","at":"2026-11-09T20:22:19Z"}
```

Outcome statuses: `PUBLISHED`, `REFUSED`, `REFUSED_STALE`,
`RECONCILED_PUBLISHED`, `RECONCILED_NOT_PUBLISHED` (**CODE_READ**). The `id` is
`<UTC timestamp>-<first 12 chars of the new commit>-<nanosecond suffix>`; the
suffix exists so two publications in the same second cannot share an id (the
P0-2 defect). `pending_intents`/reconciliation count intents against outcomes
per id: one outcome can never close two intents.

## 9. Exit codes and statuses — complete table

See §2. Additional statuses in this worktree: `VERSION`, `REPORT`,
`INITIALIZED`, `PASSED`, `PUBLISHED`, `RECONCILED`, `EXPLAIN`, `DISABLED`,
`ENABLED` (0); `REFUSED`, `FAILED`, `TIMEOUT`, `PARTIAL`, `UNSUPPORTED`,
`UNKNOWN`, `INCOMPLETE`, `ERROR`, `CORRUPT_JOURNAL`, `UNKNOWN_OUTCOME`,
`NEEDS_RECONCILIATION` (1). Any future status that is not added to the
allowlist exits 1 by construction.

## 10. The 0.1 surface — implemented on `feat/flow-production-candidate`

This document was first written at `d884e29`, where the multi-agent surface did not exist. It does
now. The table below is the authoritative status; the field lists above (§4–§9) describe the commands
that already existed at `d884e29` and are unchanged except where this section says otherwise.

| Command | Status emitted | Schema | Evidence |
|---|---|---|---|
| `flow init --adopt-policy` | `INITIALIZED` (`adopted: true`, `adoption_source`); without the flag on a shipped config: `POLICY_ADOPTION_REQUIRED`, exit 1 | — | `examples/polyglot/run-demo.ps1` (four fixtures), CI job `polyglot` |
| `flow approve` | `APPROVED` + the exact obligation commands and the policy digest | — | every fixture helper; `m9_a_hostile_cloned_repository_cannot_execute_or_escape` |
| `flow propose` | `ADMITTED` (`idempotent: true` on re-submission); `IDEMPOTENCY_CONFLICT`, `PATCH_REFUSED`, `BASE_INVALID`, `FORBIDDEN_PATH` | `flow-proposal/1` | `admission_is_idempotent_and_immutable`, `proposals_outside_the_trusted_surface_are_refused` |
| `flow proposals list` | `RECORD` + `count` | `flow-proposal-list/1` | `m1_two_disjoint_agents_…` |
| `flow proposals show <id>` | the proposal record; `INVALID_RECORD_ID` for a non-hex id | `flow-proposal/1` | `m10_a_record_from_another_schema_is_refused` |
| `flow assemble` | `ASSEMBLED`; `BASE_MISMATCH`, `PATH_OPERATION_CONFLICT`, `CONFLICT`, `DUPLICATE_PROPOSAL`, `UNKNOWN_PROPOSAL` | `flow-candidate/1` | M1, M2, `assembly_reports_order_and_stays_deterministic`, `m8_…` |
| `flow run --candidate <id>` | receipt whose `candidate_id` and `tree_oid` name that candidate; `UNKNOWN_CANDIDATE`, `CANDIDATE_TREE_MISSING` | `flow-receipt/2` | M1 |
| `flow publish --candidate <id> --dry-run` | `PLAN`, exit 0, no commit/ref/journal write | `flow-plan/1` | `publish_dry_run_plans_without_mutating_anything` |
| `flow publish --candidate <id> --expected-old <oid>` | `PUBLISHED`; `BASE_MISMATCH` on a stale base, `NO_RECEIPT` without a verified candidate | — | M1, M3, `publishing_a_candidate_requires_a_verified_receipt` |
| `flow explain --candidate <id>` | `EXPLAIN` + `proposals`, `receipt_trust`, `receipt_status_display`, `evidence_complete`, `missing_logs` | — | `status_and_explain_qualify_a_tampered_receipt`, `explain_reports_missing_logs_as_incomplete_evidence` |
| `flow status --json` | `REPORT`, schema `flow-status/1`, `policy` block, `latest_receipt` with `trust` | `flow-status/1` | `status_reports_policy_provenance` |

Two behaviour changes matter here and are documented in `FLOW-PRODUCTION-COMPATIBILITY.md`: an
identifier that is not a hex object/record id is refused (`INVALID_TREE_ID` / `INVALID_RECORD_ID`),
and `status`/`explain` refuse to present a receipt whose digest no longer matches as `PASSED`.

The exit-code list in §2 is extended by the statuses above: `ADMITTED`, `ASSEMBLED`, `PLAN`,
`APPROVED`, `RECORD` and `REPORT` exit 0; every refusal and `ERROR` exits 1; usage errors exit 2.
## 11. Schema versioning and compatibility policy

| Artifact | Version token | Enforcement in this worktree |
|---|---|---|
| Config | `flow-config/1` | exact match required; a different value is a hard error (`unsupported config schema`) — **CODE_READ** |
| Receipt | `flow-receipt/2` | `publish` refuses another schema with `RECEIPT_SCHEMA_STALE` before any integrity check — **MEASURED** |
| Journal | `flow-journal/1` | written on every record; the reader accepts any valid JSON line, so the tag is informational here — **CODE_READ** |
| Context codec | `flow-context/2` | the codec name is inside the hashed context; a codec change changes `context_id`, and `publish` refuses `CONTEXT_MISMATCH` instead of reusing an old receipt — **CODE_READ** |
| Proposal / candidate / plan / status | `flow-proposal/1`, `flow-candidate/1`, `flow-plan/1`, `flow-status/1` | not defined in this worktree — **NOT_RUN** |

Compatibility rules this document asserts, all grounded in the code above:

1. **Refuse, do not reinterpret.** `publish` refuses a receipt under another
   schema, and the config reader refuses another config `schema`; neither is
   silently upgraded or downgraded. (`explain` still displays any receipt file
   that parses, because it is a read-only viewer, not a gate.)
2. **The policy identity is recomputed, never trusted.** `publish` recomputes
   the effective policy digest and the context from the current config,
   environment and tree; anything that changed between `run` and `publish` is a
   refusal with a specific code.
3. **The context version is part of the digest.** Adding an input to
   `flow-context` (or changing its codec) invalidates older receipts rather than
   silently comparing incompatible contexts.
4. **Integrity is not signature.** It detects edits; it does not authenticate
   the writer. A future signed receipt would be a new field/schema, not a
   reinterpretation of `integrity`.
5. **Additive config fields are safe, additive receipt fields are not free.** The
   config reader ignores unknown fields; a receipt is sealed over its whole body,
   so adding or editing any field without resealing is `RECEIPT_INTEGRITY`.
6. **A new status defaults to exit 1.** Only the allowlist in §2 maps to 0, so a
   status this version does not understand fails closed.

## 12. NOT_RUN inventory

- `init --adopt-policy` (integration-checked at `17c0156` through the drivers,
  not part of this worktree), `approve`, `--json` flags, `status --json`.
- `propose`, `proposals list`, `proposals show`, `assemble`,
  `run --candidate`, `publish --candidate`, `publish --dry-run`,
  `explain --candidate`.
- Schemas `flow-proposal/1`, `flow-candidate/1`, `flow-plan/1`,
  `flow-status/1`.
- Exit codes for `PLAN`, `PROPOSED`, `ADMITTED`, `ASSEMBLED`, `APPROVED`,
  `STATUS`.
- The proposal-level collision semantics of §9 of the
  [user guide](MVP-USER-GUIDE.md#9-a-controlled-collision-and-its-resolution).
- CI on Ubuntu and Windows (owned by the integration branch).
