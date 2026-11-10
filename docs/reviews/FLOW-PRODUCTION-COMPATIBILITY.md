# Flow production candidate — compatibility, formats and migration (G9)

## 1. Versioning

`flow 0.2.0-rc.1`. The CLI surface grew (propose/assemble/candidates) and the durable formats gained
identifiers, so `0.1.x` is not silently reinterpreted as `0.2.x`:

- a **reader** refuses a record whose `schema` it does not know, with a message naming both;
- a **writer** always stamps the current schema;
- no format migration is destructive, and none happens implicitly.

## 2. Durable formats

| Format | File | Version | Compatibility rule |
|---|---|---|---|
| Policy / config | `.flow/config.json` | `flow-config/1` | unknown schema refused; no obligation runs until the policy digest is adopted |
| Adoption | `.flow/adopted.json` | `flow-adoption/1` | unknown schema refused (`refusing to guess what was approved`) |
| Receipt | `.flow/receipts/<tree>.json` | `flow-receipt/2` | an older/newer schema is `RECEIPT_SCHEMA_STALE` at publication; re-run `flow run` |
| Journal | `.flow/journal.jsonl` | `flow-journal/1` per line | a line without a known schema is a corrupt journal (`CORRUPT_JOURNAL`), never skipped |
| Proposal | `.flow/proposals/<id>.json` + `.patch` | `flow-proposal/1` | a foreign schema is refused on load (`refusing to reinterpret`), test `m10` |
| Candidate | `.flow/candidates/<id>.json` | `flow-candidate/1` | same rule |
| Plan | stdout of `publish --dry-run` | `flow-plan/1` | output only, no on-disk state |
| Status | stdout of `status` | `flow-status/1` | output only |
| Lock | `.flow/lock` | `flow-lock/2` | a different record is `LOCK_UNKNOWN` + `manual_required`; never stolen |

### Migration from 0.1 to 0.2.0-rc.1

1. Install the new binary; `flow version` reports `0.2.0-rc.1` and the schema list.
2. Existing **receipts** are still readable for inspection (`explain`), and `publish` refuses them
   with `RECEIPT_SCHEMA_STALE` until `flow run` is re-executed — the honest behaviour, because the
   policy semantics changed (the obligation environment allowlist now includes the MSVC toolchain
   variables, so the policy digest changed).
3. Existing **proposals** and **candidates** are `flow-proposal/1` / `flow-candidate/1` and remain
   readable; they were introduced by this version.
4. `.flow/journal.jsonl` is append-only and is never rewritten. `reconcile` still resolves intents
   written by 0.1.
5. No backup is strictly required, but `cp -r .flow .flow.bak` before the first 0.2 run is
   recommended and documented in the release checklist.

**There is no automatic, destructive migration**, and this document does not claim one.

## 3. Operating systems and toolchains

| OS | Status | Evidence |
|---|---|---|
| Windows 11 x86_64 (MSVC, rustc 1.95.0) | **supported, local + CI** | 163 tests, polyglot demo, RC install smoke (local and CI) |
| Ubuntu (GitHub `ubuntu-latest`, rustc 1.95.0) | **supported, CI only** | 162 tests (the PATH/PATHEXT shim test is Windows-only), polyglot demo, install smoke |
| macOS | `NOT_TESTED_MACOS` | no runner, no claim |

Language fixtures: Rust (`cargo fmt/clippy/test`, offline), Node JavaScript (`node --test`), Python
(`python -m unittest`), TypeScript (`tsc --noEmit` + executed tests) — see `examples/polyglot/`.
TypeScript requires a compiler on `PATH`; when it is missing the required obligation fails, and that
is a refusal, not a skip.

## 4. CLI contract

Exit codes: `0` for `PASSED/PUBLISHED/PLAN/INITIALIZED/APPROVED/ADMITTED/ASSEMBLED/RECORD/
RECONCILED/EXPLAIN/REPORT (overall=OK)/STATUS/DISABLED/ENABLED`; `1` for every refusal, failure,
unknown outcome, and `doctor` with `overall=FAILED`; `2` for usage errors (unknown option, value flag
without a value, unknown command).

JSON is the only machine output; `--json` is accepted for compatibility. `help` is text and says so.
Refusals always carry a stable `code`; `ERROR` carries `error`.

## 5. Runtime and network

The engine performs no network I/O. Obligations may do anything the user's programs do; this is the
declared trust boundary, not a claim of network isolation. `cargo test --offline`, the polyglot demo
and the RC install smoke all run without network.

## 6. What changed for users of 0.1

| Change | Effect |
|---|---|
| policy adoption gate | `flow init` on a repository that already ships `.flow/config.json` requires `--adopt-policy`, and `flow approve` after any intentional policy edit |
| obligation environment allowlist now includes MSVC toolchain variables | older receipts become `STALE_POLICY`; `doctor` and `status` show the policy digest |
| `explain --tree` accepts only an object id | a path used to be read as JSON; now `INVALID_TREE_ID` |
| `doctor` exits 1 when a guarantee is broken | scripts that ignored the exit code will now see a failure |
| unknown options are usage errors (exit 2) | a typo no longer silently publishes |
