# Human decisions required before promotion — 2026-11-15

**Status: ANSWERED 2026-11-15 — recorded in [DECISION_REGISTER](../DECISION_REGISTER.md).** The
owner's answers are kept verbatim there, including the one that departs from the recommendation:
**`H1 = O3 complet`**. This document prepared the questions; the register is the authority.

| Row | Owner's answer | What it changes |
|---|---|---|
| H1 | **O3 complet** | O3 is the target architecture. **It is not implemented**: the branch delivers the O1 + O2 subset, and O3 needs its own design, budget and P0 review. No document may present O3 as delivered. |
| H3 | Refus/fail-closed hors isolation OS | Product stays `local-trusted`; untrusted execution `UNSUPPORTED`; no confinement claim. |
| H4 | ≥30 % less time, p<0.05, 10 pairs, 2 cores/8 GB | S9 thresholds and budget are frozen; the campaign is not run, so productivity stays `UNPROVEN`. |
| H6 | Draft handoff, no Ravum write | [RAVUM-HANDOFF-DRAFT-2026-11-15.md](RAVUM-HANDOFF-DRAFT-2026-11-15.md) is authorized; transmission is the owner's act. |
| H8 | Adopt the review stop rule | Adopted from 2026-11-15. |
| ADR-012 | Stay `PROPOSED` until G10, then ratify | Still `PROPOSED`; ratification is a post-`G10` act. |

`G10` (reviewer outside the team) and `GO_PRODUCTION` remain open; the verdict stays
`PRODUCTION_PARTIAL`.

Context: the branch `feat/flow-production-hardening-fable` is green on every required CI check at
head `422fad2` (run `43712727257`): 539 Windows / 550 Linux / 550 macOS executed, 0 ignored, and the
product-only coverage gate at **95.95 % lines (5421/5650) / 90.52 % branches** with the thresholds
unchanged and no waiver. The three fixes found by the internal pre-review (unapproved context probes,
publication of swapped/missing log evidence, `gc --keep` deleting fresh candidates) and the two
CI-found concurrency defects (worktree-admin race, staging-link `nlink` race) are applied and green on
that head. The remaining blockers are the independent review and the O3 workstream, not this code.

## H1 — architecture option

| Option | What it means | Consequence |
|---|---|---|
| **O1 + O2 subset (recommended)** | Local CLI: verify an exact tree, run reviewed obligations in an isolated scratch checkout, seal evidence, publish by ref CAS. No scheduler, no agent registry, no distributed state. | Matches what is implemented and measured here; keeps the threat model local-trusted. |
| O3 full | Add orchestration/distribution (multi-agent scheduling, remote state). | Not designed, not budgeted, not measured; each addition needs its own P0 review. |
| O0 build nothing | Keep the audit only. | Contradicts `H7` (independent product, decided 2026-11-13). |

## H3 — untrusted sandbox policy

| Option | What it means | Consequence |
|---|---|---|
| **Refuse/fail-closed where no OS containment (recommended)** | Keep the documented `local-trusted` boundary; untrusted code execution stays `UNSUPPORTED`; no claim of isolation. | Honest and already displayed by `doctor`; untrusted use is refused by documentation, not by a kernel boundary. |
| Implement real containment | Linux namespaces + Windows Job Objects, with containment tests. | A new subsystem with its own threat model; not in this branch. |
| Defer | `H3` stays `OPEN`. | Promotion stays blocked; the limit stays documented. |

## H4 — S9 threshold and budget

| Option | What it means | Consequence |
|---|---|---|
| **Grant (recommended)** | Confirmatory paired campaign: Flow vs plain Git + CI on the S9 fixture set, 10 paired runs, 2 cores / 8 GB, pre-registered stop if Flow is > 2× slower or variance is unexplained; success = **≥ 30 % less wall-clock to a verified publish, p < 0.05**. | Turns the current `UNPROVEN` productivity claim into a bounded, falsifiable measurement. |
| No numbers | Keep S9 `UNPROVEN`. | No productivity claim, `H4` stays blocking. |

## H6 — handoff of the P0 risk to the separate Ravum session

| Option | What it means | Consequence |
|---|---|---|
| **Draft handoff, no Ravum write (recommended)** | A handoff document is prepared in this repository; the Ravum session decides what to apply. | Flow never edits `Rwanbt/Ravum`; the risk is visible to the session that owns the code. |
| Defer | Recorded here only. | The other session may not see the risk. |

## H8 — stop rule for documentary reviews

| Option | What it means | Consequence |
|---|---|---|
| **Adopt (recommended)** | No new review dossier unless a decision is blocked; every review ends with `GO`/`NO-GO` and one concrete artifact. | Stops review inflation; this report already follows it. |
| Defer | `H8` stays `OPEN`. | Review cost stays uncapped. |

## ADR-012 — evidence store, lock protocol, context

| Option | What it means | Consequence |
|---|---|---|
| **Keep `PROPOSED` until the independent review, then ratify (recommended)** | The contract is already implemented and tested; the reviewer may still ask for a change. | Ratification is informed by an outside read. |
| Ratify `ACCEPTED` now | Freezes the contract before the review. | Less prudent; a reviewer finding would reopen an accepted ADR. |
| Reject | No contract for the store/lock/context. | The implemented invariants lose their written authority; promotion blocked. |

## What each decision unlocks

- All six answered and ADR-012 ratified (or explicitly rejected with a replacement plan) → `G10`
  becomes the only remaining gate.
- `G10` passed by a reviewer who did not author the branch + `GO_PRODUCTION` from the owner → the
  verdict can move from `PRODUCTION_PARTIAL` to `PRODUCTION_READY_LOCAL_TRUSTED` for this line.
- Anything left unanswered keeps the current verdict; no code change can close it.

**How to answer.** The owner can answer in one message naming the rows (`H1=O1+O2`, `H3=fail-closed`,
`H4=grant`, `H6=draft`, `H8=adopt`, `ADR-012=ratify after G10`) or state different choices. The
answers will be recorded verbatim in `docs/DECISION_REGISTER.md` with the date and the evidence they
rely on.

