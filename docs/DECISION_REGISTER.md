# Decision register — HUMAN SIGN-OFF REQUIRED
No choice below is accepted merely because documentation has been uploaded. Mark human decisions with date, actor, evidence and constraints.

| ID | Decision | Current state | Candidate direction | Evidence required |
|---|---|---|---|---|
| H1 | Provisional design O0/O1/O2/O3 | OPEN | Compare O0, O1, O3; keep O2 as subset | S9 + safety + maintainability |
| H2 | License, rights, extraction vs clean-room | OPEN / BLOCKER for copying | Decide ownership and provenance first | rights inventory + legal review where needed |
| H3 | Untrusted sandbox Windows/Linux policy | OPEN | refuse/fail-closed where isolation unsupported | actual OS-level containment tests |
| H4 | S9 threshold and resource budget | OPEN — **no numbers granted** | pre-register paired difference, cost and stop criteria | human-set numeric threshold/budget before run |
| H5 | R0 fixtures authorization, then minimal implementation | **HUMAN_DECIDED 2026-11-09** (owner-delegated GOAL, bounded) | R0-a/b/c on synthetic fixtures, then smallest useful core only if O0 is demonstrably insufficient | this GOAL's scope; results in `FLOW-FOUNDATION-VERIFIED-REPORT.md` |
| H6 | Handoff P0 to separate Ravum session | OPEN | send risk report, never edits | session priority / human agreement |
| H7 | Internal-first use vs independent product | OPEN (**new**, proposed) | validate on Ravum as first user; keep the polyglot product target, do not redefine Flow as an internal tool | usage evidence, maintenance cost, product intent |
| H8 | Stop rule for documentary reviews | OPEN (**new**, proposed) | no new review dossier unless a decision is blocked; every review ends with GO/NO-GO + one concrete artifact | observed review cost vs decision throughput |
| D0 | source, provenance and rights inventoried | PARTIAL | verify GitHub actual refs and rights | citations, SHA, 42 ADR audit |
| D1 | independent contradictory review | DOCUMENT-CONDITIONAL | Claude V5 reviewed; V6 corrected afterwards | unresolved differences + provenance |
| D2 | security and transactional contract | PROPOSED | tree/hook/context/receipt/fencing rules | negative test specifications; no claims of tests run |
| D3 | S9 protocol frozen | DRAFT | paired benchmark with real worktrees and tools | budget, threshold, seeds, gates |
| D4 | reviewed human decision and scoped GO | PARTIAL (planning + R0 granted; D4 not closed) | separate planning, fixture and implementation permissions | dated explicit acceptance |

## Scope of authorizations
- `GO_PLANNING` = documents/issues/research only.
- `GO_R0_FIXTURES` = **GRANTED 2026-11-09** by the owner's operational GOAL, bounded to: branch `feat/flow-verified-foundation`, disposable synthetic fixtures, fictitious canaries, no real credentials, no Ravum modification, pilot-scale measurement only.
- `GO_R1`…`GO_R7` = separate scoped authorizations after each predecessor's evidence. The same GOAL permits only the *smallest useful core* after the R0 gate, with tests and CI; it is not a general implementation permission.
- `GO_PRODUCTION` = future release authorization, not implied.

## Decisions this GOAL does **not** ratify
The owner's operational GOAL is not a ratification of `H1`, `H2`, `H3`, `H4`, `H6`, `H7`, `H8`, of any `RAVUM-ADR-*`, or of any `FLOW-ADR-*`. It does not select a license, does not close `D3`, and does not authorize a merge into `dev` or `main`.

## Pending architecture decisions
- RF-RFC-001..012 retain historical `PROPOSED` meanings. Proposed RF-RFC-013..024 are only ideas.
- No new file may silently mark a Ravum ADR or Flow ADR `ACCEPTED`.
- No GPL/FSL/copyleft code reuse without precise license/legal assessment. No implicit MIT/Apache license in Flow.
- Canonical ADR namespaces: `RAVUM-ADR-*` vs `FLOW-ADR-*` (see [TRACEABILITY](TRACEABILITY.md)); bare `ADR-0NN` is ambiguous.

## Evidence labeling
`CODE_READ`, `SOURCE_READ`, `MEASURED_HISTORICAL`, `MEASURED`, `PLAUSIBLE_NOT_REPRODUCED`, `NOT_REVIEWED`, `PROPOSED`, `UNKNOWN`. Human verdict documented independently from model opinions.
