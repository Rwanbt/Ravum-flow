# Flow — post-Fable production hardening (2026-11-15)

**Status: IN PROGRESS — no production verdict is claimed by this line yet.**
This is the single tracking report for the Fable hardening wave. It carries the baseline, the GO
record, the FBL matrix, the gates, the evidence and the final verdict. Every claim below names its
status: `CODE_READ`, `SOURCE_READ`, `MEASURED`, `CONFIRMED_BY_TEST`, `NOT_REPRODUCED`, `NOT_RUN`,
`OPEN`, `REFUTED`.

## 1. Baseline

| Field | Value |
|---|---|
| Repository | `Rwanbt/Ravum-flow` (local `D:\App\Ravum-flow`) |
| Fable audit SHA | `1952325c973c3b002d6436a83656a5593003ad48` (`main` = `origin/main`) |
| `origin/dev` at start | `2ab79c754e0a48746fc50689af28d8124b536563` (same tree as `main`) |
| Working branch | `feat/flow-production-hardening-fable`, created from `origin/dev` |
| Version | `0.2.0-rc.2` |
| Toolchain | `rustc 1.95.0`, pinned by `rust-toolchain.toml`, zero runtime dependencies |
| Baseline state | working tree clean except untracked `RAVUM_FLOW_FABLE_AUDIT.md` |

The Fable report is an independent counter-audit. Its G1 test **reproduced** FBL-001 on a clone of
this SHA (`multi_admit_rounds=15`, `integrity_breaks=[12]`). G3 was **not reproduced** with its
thread harness (`overlap_rounds=0/60`). On this line, every finding is re-verified independently;
`CONFIRMED_BY_CODE` is never upgraded to `CONFIRMED_BY_TEST` without a run.

## 2. GO record

The owner granted `GO(scope, repository, branch, allowed_paths, budgets, evidence, stop_conditions)`
on 2026-11-15; it is recorded verbatim in [DECISION_REGISTER § Mandate of 2026-11-15](../DECISION_REGISTER.md).
It is scoped to this repository, this branch, hardening work only. **No merge, tag, release or
`Rwanbt/Ravum` write is authorized.** `GO_PRODUCTION` remains the owner's act.

## 3. FBL matrix (initial)

`HEAD status` is what an independent read at the working SHA establishes; `fix` links the change
once it exists; `proof` links the regression test and its run. A row is only `closed` when a test
proves the failure mode is gone.

| ID | Sev | Fable status | HEAD status | Fix | Proof | State |
|---|---|---|---|---|---|---|
| FBL-001 | P0 | CONFIRMED_BY_TEST | CODE_READ: `admit` check-then-act, two independent renames, `load`/`assemble` do not re-hash | — | — | OPEN |
| FBL-002 | P1 | CONFIRMED_BY_CODE | CODE_READ: logs keyed by tree only, receipt slot per tree, no `run_id` | — | — | OPEN |
| FBL-003 | P1 | CONFIRMED_BY_CODE, NOT_REPRODUCED | CODE_READ: read-check-`remove_file` steal window | — | — | OPEN |
| FBL-004 | P1 | CONFIRMED_BY_CODE | CODE_READ: `BuildContextId` hashes env **names**, not values/binary identity/toolchain | — | — | OPEN |
| FBL-005 | P2 | CONFIRMED_BY_CODE | CODE_READ: `gc` takes no shared lock and has no run-active marker | — | — | OPEN |
| FBL-006 | P2 | CONFIRMED_BY_CODE | CODE_READ: `Config::from_json` lenient (unknown keys, non-string args dropped, timeouts clamped) | — | — | OPEN |
| FBL-007 | P2 | CONFIRMED_BY_CODE | CODE_READ: receipt selected by tree, no `candidate_id` binding in publish | — | — | OPEN |
| FBL-008 | P2 | HYPOTHESIS | CODE_READ: lock record has pid but no host/boot identity | — | — | OPEN |
| FBL-009 | P3 | CONFIRMED_BY_CODE | CODE_READ: candidate `exists()` then atomic write | — | — | OPEN |
| FBL-010 | P2 | CONFIRMED_BY_CODE | CODE_READ: candidate records not sealed/revalidated at load | — | — | OPEN |
| FBL-011 | P3 | CONFIRMED_BY_CODE | CODE_READ: absolute personal paths persisted in JSON outputs | — | — | OPEN |
| FBL-012 | P3 | CONFIRMED_BY_CODE | CODE_READ: `append_journal` rewrites the whole file per append | — | — | OPEN |

## 4. Gates

| Gate | Requirement | Status |
|---|---|---|
| G1 | concurrent `propose`, same key, distinct patches: one winner, others conflict, digest matches; `assemble` refuses corrupted patch | NOT_RUN |
| G2 | concurrent `run`: per-run logs, receipt digests, corruption detected | NOT_RUN |
| G3 | orphan-lock stress multi-process: never two holders; journal intent/outcome consistent after crash | NOT_RUN |
| G4 | `run` then env/toolchain/program/policy change before `publish` → named refusal | NOT_RUN |
| G5 | altered proposals/candidates/receipts/logs → fail-closed named diagnostic | NOT_RUN |
| G6 | strict policy parser + migration matrix | NOT_RUN |
| G7 | CI green Linux/Windows/macOS on final commit, G1–G6 on Linux and Windows | NOT_RUN |
| G8 | multi-host limits documented and refused safely | NOT_RUN |

## 5. Measurements and residual risk

(to fill: baseline test counts, per-fix runs, any benchmark; none are claimed before they are run)

## 6. Limits of this wave

- Threat model: **local-trusted, single-host**. No OS sandbox, no signed receipts, no multi-host
  synchronization is claimed or implemented.
- `RAVUM_FLOW_FABLE_AUDIT.md` itself is evidence of the baseline, not of this line.
- Unreviewed by Fable and re-read here: `cmd_reconcile`, `paths.rs`, `main.rs`, CI workflows,
  `MEASUREMENTS.md`, `CAPABILITIES.md`, ADR registry.

## 7. Verdict

**OPEN.** The verdict is set only after G1–G8 and the CI evidence are recorded. A P0 that remains
open forces `NOT_READY`; remaining blocking gates force `PRODUCTION_PARTIAL`.
