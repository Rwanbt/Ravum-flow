# G10 — independent final review: brief and evidence pack

**Status: SLOT OPEN — no independent reviewer has reviewed this line yet.** `G10` is a gate, not a
document; writing this brief does not satisfy it. An *internal adversarial pre-review* may be run by
separate agents to prepare the ground, and is labelled as such, never as `G10`.

## What G10 is

A reviewer who did not author this branch and does not belong to the team that wrote it reads the
diff, the evidence and the contracts, then answers `GO` or `NO-GO` for promotion to
`PRODUCTION_READY_LOCAL_TRUSTED` on the local-trusted threat model. The reviewer's verdict is
recorded verbatim; it can be `GO WITH CONDITIONS`.

## Evidence pack (all in this repository)

| Artifact | Why |
|---|---|
| Branch `feat/flow-production-hardening-fable`, diff `d8642a1..HEAD` (plus the docs commit) | the change under review |
| [tracking report](FLOW-POST-FABLE-PRODUCTION-HARDENING.md) | FBL matrix, gates, evidence, verdict rules |
| [Fable counter-audit](../../RAVUM_FLOW_FABLE_AUDIT.md) | the findings being answered |
| [MEASUREMENTS](../MEASUREMENTS.md) | every count with its head, platform and CI run id |
| [ADR-012](../adr/FLOW-ADR-012-evidence-store-and-lock-protocol.md) | the persistence/lock/context contract to ratify |
| [human decisions memo](FLOW-HUMAN-DECISIONS-2026-11-15.md) | what the owner must still decide |
| CI run `43708083607` | green on every required check at the head |

## Questions the reviewer must answer

1. **FBL-001..013**: is each row's `CLOSED`/`PARTIAL`/`DEFERRED` state supported by a named test or
   by code the reviewer read? Name any row whose state overstates its proof.
2. **Evidence store**: can two writers still publish one name with different bytes, or leave metadata
   pointing at missing bytes? Consider crash points between `put_blob`, `create_once` and the journal.
3. **Lock**: can two processes both believe they own `.flow/lock`? Is any path a steal of a live
   owner, including a foreign or host-less record, a name reused after reboot, or a claimed recovery?
4. **Publish**: can a ref move without a verified receipt for that exact tree, policy, context and
   candidate? Can a failure be reported as success?
5. **Context**: can two different toolchains/environments produce the same `flow-context/3` digest?
6. **`gc`**: can it delete evidence a live run, a pending intent or an unpublished receipt needs?
7. **Claims**: does any document claim more than the measurements show (test counts, coverage,
   mutation, productivity, sandbox, multi-host)?
8. **Threat model**: is the documented boundary (`local-trusted`, single host, unsigned receipts, no
   OS sandbox) stated everywhere it matters, and does any surface contradict it?

## Required output

- `GO`, `GO WITH CONDITIONS` or `NO-GO`, with the conditions listed concretely.
- Findings by severity (`P0`/`P1`/`P2`/`P3`) with `file:line`, the failure mode, and whether the
  finding blocks promotion.
- For each finding: `CODE_READ`, `REPRODUCED` or `NOT_VERIFIED`.
- A statement of independence: the reviewer confirms they did not write the branch.

## Boundaries

- The reviewer reads and may run the tests; the reviewer **does not edit the branch**.
- No merge, tag, release or `Rwanbt/Ravum` write is authorized by this review.
- A review that cannot tie a claim to a cited measurement must say `UNPROVEN`, not `PASS`.

## Internal adversarial pre-review (not G10)

Two separate agents reviewed the branch before the external slot was filled, one on the transactional
contracts and one on the claims/evidence. Their findings are recorded in
[FLOW-PRE-G10-ADVERSARIAL-REVIEW.md](FLOW-PRE-G10-ADVERSARIAL-REVIEW.md); that file states plainly
that it is **not** the independent review and cannot close `G10`.
