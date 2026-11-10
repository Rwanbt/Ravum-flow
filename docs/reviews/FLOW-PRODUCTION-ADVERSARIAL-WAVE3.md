# FLOW production adversarial campaign — wave 3

**Date:** 2026-11-10 (Europe/Paris) · **Reviewer:** `verify-p0` (independent; did not write the product)
**Branch:** `feat/flow-production-candidate` · **Head at run:** `5077c508b00c9e6b2062ec4a68e166c265140547`
· **Harness:** [`experiments/mvp-qa/attacks.py`](../../experiments/mvp-qa/attacks.py) (Python stdlib only)
· **Machine-readable result:** [`experiments/mvp-qa/results-wave3.json`](../../experiments/mvp-qa/results-wave3.json)
· **Human verdict tables:** [`experiments/verify-p0/verdict-tables-wave3.md`](../../experiments/verify-p0/verdict-tables-wave3.md)

## 1. Exact invocation and environment

```powershell
cd D:\App\Ravum-flow
python experiments\mvp-qa\attacks.py `
  --flow "$env:TEMP\flow-head-target\debug\flow.exe" `
  --release-flow "$env:TEMP\flow-head-target\release\flow.exe" `
  --out experiments\mvp-qa\results-wave3.json `
  --markdown experiments\verify-p0\verdict-tables-wave3.md `
  --base-sha 5077c508b00c9e6b2062ec4a68e166c265140547
```

| Item | Value |
|---|---|
| Head | `5077c50` "fix: verify the executable bit of the verification checkout on Unix" |
| Parent chain | `0de8c21` → `dfb941d` (P0 fidelity fix) → `2f4814e` → `5077c50` |
| `git status --porcelain` (recorded around this run) | `M experiments/mvp-qa/attacks.py` plus other agents' untracked artefacts under `experiments/`, `docs/`, `scripts/`; **no `crates/flow/src/**` change** — the binary is the committed head |
| Debug binary | `C:\…\Temp\flow-head-target\debug\flow.exe`, sha256 `69622f4936e2478fda0dcb2a927b0e4aef605bda96d03a1ea48abfe20d6f1fc4` |
| Release binary (PRC-14) | `C:\…\Temp\flow-head-target\release\flow.exe`, sha256 `3cb65065d04a86d698de1b9b435ecbe6baa41d3c422c4c4d42659ce25e49163f` |
| Provenance | both built by `git archive 5077c50` → `cargo build --locked --offline` (`CARGO_TARGET_DIR` outside the repository) |
| Host | Windows 11 `10.0.26100`, Git `2.51.1.windows.1`, Python 3.13.7, rustc 1.95.0 |
| Fixtures | disposable under `%TEMP%`, removed by the harness (`FLOW_QA_KEEP` not set) |

> The branch moved twice while this review was running (`dfb941d` → `2f4814e` → `5077c50`).
> `dfb941d` is the commit the Lead named; the table above records the head that the canonical run
> actually executed. A first replay of the unmodified 119-scenario set was also made on `dfb941d`
> (debug `b023024e…`, release `38a01662…`) and is kept in
> `experiments/mvp-qa/results-wave3-119.json`; its verdicts are identical to the 119 subset of this run.
> After the run the branch advanced to `ccdbec9` (`fc80af3` pins a local smudge filter as a checkout
> bypass with a red-first gate; `ccdbec9` adds error-path tests). Both commits add **tests only** — no
> `crates/flow/src/**` change — so the binaries measured here remain the code under review; the
> `git_fidelity` file at `ccdbec9` has 12/12 Windows tests passing.

Wave-2 history is untouched: [`results.json`](../../experiments/mvp-qa/results.json) (branch
`work/prod-security`, head `f237d57`, 119 scenarios, debug `5d0b778e…`) remains the wave-2 record.
Wave 3 writes new files only.

## 2. Verdict

- Declared: **122** · executed: **121** · REPRODUCED: **6** · REFUTED: **115** · NOT_RUN: **1** · ERROR: **0**
- Reproduced by severity: `{'P2': 3, 'P3': 3}`; P0 reproduced: **0**
- Binary: debug `69622f4936e2478f…`, release `3cb65065d04a86d6…`; base commit `5077c508b00c`

## 3. Per-axis table

| Axis | Declared | Executed | REPRODUCED | REFUTED | NOT_RUN | ERROR |
|---|---|---|---|---|---|---|
| Identity / receipt scope | 20 | 20 | 0 | 20 | 0 | 0 |
| Scope (portée) | 11 | 11 | 0 | 11 | 0 | 0 |
| Git refs and CAS | 12 | 12 | 0 | 12 | 0 | 0 |
| Durable journal and recovery | 11 | 11 | 0 | 11 | 0 | 0 |
| User files and hostile paths | 23 | 23 | 1 | 22 | 0 | 0 |
| Process containment | 14 | 14 | 1 | 13 | 0 | 0 |
| Proof quality | 9 | 8 | 3 | 5 | 1 | 0 |
| Compatibility | 7 | 7 | 1 | 6 | 0 | 0 |
| 0.1 multi-agent surface | 7 | 7 | 0 | 7 | 0 | 0 |
| Huge and odd inputs | 5 | 5 | 0 | 5 | 0 | 0 |
| Checkout fidelity (wave 3, new) | 3 | 3 | 0 | 3 | 0 | 0 |
| **Total** | **122** | **121** | **6** | **115** | **1** | **0** |

## 4. The three wave-2 NOT_RUN scenarios

| ID | Wave-2 reason | Wave-3 status |
|---|---|---|
| `DUR-09` | "no injection point exists for a crash before the intent append: FLOW_TEST_CRASH supports only after-intent and after-ref" | **IMPLEMENTED and REFUTED.** This head has `crash_point("before-intent")` (`crates/flow/src/commands.rs`, between `commit_tree` and `append_journal`). Measured: `FLOW_TEST_CRASH=before-intent` → exit 89; ref unchanged; journal unchanged (0 bytes); `rev-list --count HEAD` unchanged; `reconcile` → `RECONCILED` and appends nothing; the next `publish` → `PUBLISHED` exactly one commit with the verified tree; journal ends with exactly 1 intent + 1 outcome. |
| `FIL-06` | "Git itself refuses to place a Windows-reserved name or a `..` path in the index (update-index: Invalid path; read-tree refuses the raw tree)" | **IMPLEMENTED and REFUTED.** The index route still refuses, but the fixture is constructible: `git mktree` builds a raw tree naming `CON`, `NUL`, `..`, `.git`, `x.` and `a\b.txt`, and `.flow/candidates/<32-hex>.json` is a file a clone can ship. `flow run --candidate` then calls `git worktree add`, which refuses each invalid path; the run returns `REFUSED/SCRATCH_REFUSED`, writes no receipt, and creates no file outside the scratch. All six names refused. |
| `PRF-09` | "an embedded NUL cannot be delivered to any process on this host: CreateProcess/`subprocess` refuses it" | **STILL NOT_RUN, and it is not host-specific.** An argument is a NUL-terminated byte string at every OS boundary (`execve`, `CreateProcess`); an embedded NUL cannot be delivered to `flow` on Windows *or* Linux, so no fixture can test "what the product does with it". The scenario stays `NOT_RUN` rather than being counted as a pass. CODE_READ shows the shape is also bounded: `Receipt::load` rejects any non-hex `tree` through `paths::is_object_id` before constructing a path (`crates/flow/src/state.rs`, `crates/flow/src/paths.rs`), so even a delivered NUL would be an invalid identifier, not a path-traversal primitive. |

Wave-3 harness changes (only `attacks.py`; wave-2 data files untouched):

1. `DUR-09` rewritten to drive the existing `before-intent` injection point.
2. `FIL-06` rewritten to use `git mktree` + a shipped candidate record (the index route is kept as
   evidence in `attempts[*].index_route`).
3. Three new scenarios, `W3-01`–`W3-03`, under a new `fidelity` axis, so the campaign regresses the
   `dfb941d` fix itself:

| ID | Scenario | Expected | Observed | Verdict | Sev |
|---|---|---|---|---|---|
| W3-01 | Local attribute conversion: `.git/info/attributes` `*.md text eol=crlf` on a tracked LF blob, and `core.attributesFile` (outside the repo) `data.txt text eol=crlf`, in two independent repos (`Path.write_text` writes CRLF on Windows, so blobs are staged with `write_bytes` before the attributes exist) | both runs refuse `UNSUPPORTED_CHECKOUT` naming the converted entry, no receipt; `doctor` names the local source in each repo | both refused (`payload.md` 15 vs 13 bytes; `data.txt` 4 vs 3 bytes), no receipt, doctor detail names `.git/info/attributes` / `core.attributesFile` | **REFUTED** | P0 |
| W3-02 | Tracked `post-checkout` under `core.hooksPath=.hooks` creates `injected.txt` | refuse (`does not declare`), no receipt | `REFUSED/UNSUPPORTED_CHECKOUT`: "holds 1 file(s) the tree does not declare … `injected.txt`", no receipt | **REFUTED** | P1 |
| W3-03 | Same hook rewrites `README.md` from `hello\n` to `HELLO\n` (6 bytes → 6 bytes) | refuse by byte comparison, no receipt | refused: "different bytes (6 on disk vs 6 in the tree)" | **REFUTED** | P0 |

These three close the campaign gap for the wave's fix: before wave 3 the harness only exercised
**in-tree** `.gitattributes` (FIL-08) and had no hook-mutation scenario.

## 5. Reproduced findings

| ID | Severity | Title | Observed (truncated) |
|---|---|---|---|
| FIL-20 | P2 | `.flow/journal.jsonl` hard link -> append outside the repo | publish PUBLISHED but outside changed=False (0 -> 0 bytes) |
| PRC-10 | P2 | FLOW_TEST_CRASH in the shipped binary | FLOW_TEST_CRASH=after-intent -> exit 89; after-ref -> exit 89; unrecognised value -> exit 1 |
| PRF-01 | P2 | Re-sealed forged receipt | PUBLISHED a commit for a tree whose required obligation was FAILED: status and obligation entries edited, integrity digest recomputed (612c5c8c19d0...) |
| CMP-01 | P3 | JSON is the default and parseable | these invocations printed non-JSON text: ['no-args', 'help'] (exit codes [('no-args', 2), ('help', 0)]) |
| PRF-02 | P3 | Log files deleted after the run | PUBLISHED with all 2 log files deleted; publish output reports receipt_logs=2 |
| PRF-08 | P3 | Deleted evidence files, then status/explain/publish | all 2 log files deleted: status shows latest_receipt.status=PASSED, explain shows PASSED and still lists 2 log paths, publish -> PUBLISHED (receipt_logs=2) |

Every reproduced finding is P2/P3. No P0 was reproduced in this run (the harness exits 0 and prints
`OK: no reproduced P0`). `PRF-01` (re-sealed forged receipt) is the documented non-guarantee "anyone
who can write `.flow/receipts` can forge one"; `FIL-20` (journal hardlink append outside the repo),
`PRC-10` (debug-only `FLOW_TEST_CRASH`), `CMP-01` (help/usage is text, not JSON), `PRF-02`/`PRF-08`
(deleted evidence still displayed as `PASSED` by `status`/`explain`, and publishable) are pre-existing
and unchanged from wave 2.

## 6. Wave-2 → wave-3 delta

The wave-2 recorded run was made on a different branch/head (`work/prod-security` at `f237d57`), so
this is a replay of the same scenarios on a newer head, not a controlled regression diff. Two wave-3
numbers are reported:

- the **literal replay** of the unmodified 119-scenario harness on `dfb941d`
  (`results-wave3-119.json`) — apples-to-apples with wave 2, including its three `NOT_RUN` entries;
- the **wave-3 run** on `5077c50` (`results-wave3.json`, 122 scenarios) where `DUR-09` and `FIL-06`
  are implemented and `W3-01..03` are added.

| | Wave 2 (`f237d57`, literal 119) | Wave-3 replay (`dfb941d`, literal 119) | Wave-3 run (`5077c50`, 122) |
|---|---|---|---|
| REPRODUCED | 7 | 6 | 6 |
| REFUTED | 109 | 110 | 115 |
| NOT_RUN | 3 (`DUR-09`, `FIL-06`, `PRF-09`) | 3 (`DUR-09`, `FIL-06`, `PRF-09`) | 1 (`PRF-09`) |
| ERROR | 0 | 0 | 0 |

The only verdict change between the wave-2 literal set and the `dfb941d` replay is `INP-03` (two
obligation ids that differ only by case): wave 2 `REPRODUCED`, wave 3 `REFUTED` — this head's policy
validator refuses case-colliding ids with `INVALID_POLICY` ("their log files would collide on a
case-insensitive filesystem"). That is a real improvement since the wave-2 head, not a wave-3 change.

In the 122-scenario run, `W3-01..03` are all `REFUTED` (the fix holds), `DUR-09` and `FIL-06` are
`REFUTED` (executable now), so the 119 wave-2 *ids* yield 6 reproduced / 112 refuted / 1 NOT_RUN; the
difference from the literal replay is exactly the two implemented scenarios.

## 7. Limits

- One host (Windows 11). The campaign is Windows-specific by design (junctions, reserved names,
  `cmd`); the Linux data point for the checkout fix is in
  [`FLOW-P0-GIT-FIDELITY-REVIEW.md`](FLOW-P0-GIT-FIDELITY-REVIEW.md) §7 (11/11 `git_fidelity` tests,
  local-filter refusal, TOCTOU reproduction).
- `NOT_RUN` is never counted as a pass; `PRF-09` remains the only one.
- The environment was shared with other verification agents (coverage/mutation runs), which changes
  durations but not verdicts; each scenario is deterministic in its verdict on re-run.
- The campaign does not cover OS-level isolation, which the product documents as absent.

## 8. Files

- `experiments/mvp-qa/attacks.py` — modified (additive: `DUR-09`, `FIL-06`, `W3-01..03`, `fidelity` axis).
- `experiments/mvp-qa/results-wave3.json` — this run (122 scenarios).
- `experiments/mvp-qa/results-wave3-119.json` — the earlier replay of the unmodified 119-scenario set
  on `dfb941d` (kept as an independent data point; same verdicts as the 119 subset above).
- `experiments/verify-p0/verdict-tables-wave3.md` — generated per-axis tables.
- `experiments/verify-p0/campaign-wave3.log` — console log of this run.
- `docs/reviews/FLOW-P0-GIT-FIDELITY-REVIEW.md` — independent adversarial review of the P0 fix.

Claim classification: **MEASURED** (binary observed), **CODE_READ** (source read), **PROPOSED**
(recommendation), **NOT_RUN** (explicitly not executed).
