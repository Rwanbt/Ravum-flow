# Handoff draft — Flow observations for the Ravum session

**Status: DRAFT — NOT SENT. Requires `H6`.** This document is prepared inside `Rwanbt/Ravum-flow`
only. Sending it, or acting on it in `Rwanbt/Ravum`, is the owner's decision (`H6`) and the Ravum
session's own act. Flow agents never edit, branch or build `Rwanbt/Ravum`
([boundary](../RAVUM_BOUNDARY.md)).

Recipient: the session that owns `Rwanbt/Ravum`.
Prepared: 2026-11-15, from the Flow hardening branch `feat/flow-production-hardening-fable`
(head `422fad2`, evidence in the [tracking report](FLOW-POST-FABLE-PRODUCTION-HARDENING.md)).

## What this is not

- Not a vulnerability report: none of the items below was demonstrated on a live Ravum checkout by
  this session. They are **review observations** from the Flow side, classified honestly.
- Not a request to change Ravum now; it is a list of checks the owning session can run on a
  throwaway copy.
- Not a claim about Ravum's current HEAD: the inspected baseline is the one recorded in
  [V6](RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md) and `RAVUM_BOUNDARY.md` (Claude review of
  `dev@5408d9b3…`, 2026-11-09). A newer HEAD must be re-checked before any conclusion.

## Observations, by class

| ID | Observation | Class | Why the owning session should look |
|---|---|---|---|
| P0-TREE | A verified source snapshot id and a filtered/published Git tree are different namespaces. Comparing them directly would certify a tree no obligation saw. | `CODE_READ` (Flow side), `NOT_REPRODUCED` on Ravum | Flow's own rule: `SourceSnapshotId ≠ tree OID`, and a candidate's tree is re-derived before publish. If Ravum maps a snapshot id onto a filtered tree, the mapping must be proven, not assumed. |
| P0-HOOK-ORDER | Hooks run in Git's order on `worktree add`/`update-ref`; a pre-hook tree or the actual working tree can differ from the frozen tree. | `CODE_READ`, `NOT_REPRODUCED` | Flow freezes and re-checks the checkout, and documents that a hook writing after the check is not detected. A host that runs hooks must decide what it certifies. |
| P0-ENV | An inherited environment can change what a build or a proof means. | `CODE_READ`, Flow-side fix measured | Flow now digests allowlisted env **values** (digests only) in `flow-context/3`. Any Ravum verification that consumes a build context should decide which variables it treats as input. |
| P0-LEGACY | A legacy text-only applicability verdict is not a semantic validation. | `CODE_READ` | Flow maps it to `TEXTUALLY_APPLICABLE` only, never `PASSED`. If Ravum consumes such a verdict, the mapping should be explicit. |
| P1-FILES | UTF-8-only paths and executable bits: a checkout that cannot reproduce them faithfully is not the verified tree. | `CODE_READ`, Flow-side gates | Flow refuses such trees (`a_tree_path_that_is_not_valid_utf8_is_refused…`, gitlink/mode gates). |
| P1-STORE | Reusing a store/coordinator component across the boundary carries API, licence and ownership questions. | `HYPOTHESIS` | `RAVUM_BOUNDARY.md` lists the candidates and the rights questions; nothing is reused today. |

## Suggested checks for the Ravum session (on a throwaway copy, no real secrets)

1. Reproduce the snapshot-id → tree mapping on a synthetic repository and state whether the
   comparison is by construction or by convention.
2. Add a `post-checkout`/`reference-transaction` hook that writes a file, and observe what a
   verification would certify if it ran before or after the hook.
3. Change an allowlisted environment variable between "verify" and "publish" in any existing flow,
   and observe whether the evidence still claims the same context.
4. Take one legacy `AUTO_SAFE` record and check what the consumer displays: `TEXTUALLY_APPLICABLE`
   or `PASSED`.

## Attribution and evidence

Prepared by the Flow session as a **draft**; it carries no Ravum approval and no Flow ratification.
Flow-side evidence: the gates and tests cited in the [tracking report](FLOW-POST-FABLE-PRODUCTION-HARDENING.md)
(§4–§5) and [MEASUREMENTS](../MEASUREMENTS.md). If `H6` is granted, this file is the text to send
verbatim; if not, it stays a local draft.
