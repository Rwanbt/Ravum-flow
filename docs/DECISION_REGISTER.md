# Decision register — HUMAN SIGN-OFF REQUIRED
No choice below is accepted merely because documentation has been uploaded. Mark human decisions with date, actor, evidence and constraints.

| ID | Decision | Current state | Candidate direction | Evidence required |
|---|---|---|---|---|
| H1 | Provisional design O0/O1/O2/O3 | OPEN | Compare O0, O1, O3; keep O2 as subset | S9 + safety + maintainability |
| H2 | License, rights, extraction vs clean-room | OPEN / BLOCKER for copying | Decide ownership and provenance first | rights inventory + legal review where needed |
| H3 | Untrusted sandbox Windows/Linux policy | OPEN | refuse/fail-closed where isolation unsupported | actual OS-level containment tests |
| H4 | S9 threshold and resource budget | OPEN | pre-register paired difference, cost and stop criteria | human-set numeric threshold/budget before run |
| H5 | GO_R0_FIXTURES limited authorization | NOT GIVEN | fixtures only; Ravum read-only | scope, budget, stop conditions |
| H6 | Handoff P0 to separate Ravum session | OPEN | send risk report, not edits | session priority / human agreement |
| D0 | source, provenance and rights inventoried | PARTIAL | verify GitHub actual refs and rights | citations, SHA, 42 ADR audit |
| D1 | independent contradictory review | DOCUMENT-CONDITIONAL | Claude V5 reviewed; V6 corrected afterwards | unresolved differences + provenance |
| D2 | security and transactional contract | PROPOSED | tree/hook/context/receipt/fencing rules | negative test specifications; no claims of tests run |
| D3 | S9 protocol frozen | DRAFT | paired benchmark with real worktrees and tools | budget, threshold, seeds, gates |
| D4 | reviewed human decision and scoped GO | NOT GIVEN | separate planning, fixture and implementation permissions | dated explicit acceptance |

## Scope of authorizations
- `GO_PLANNING` = documents/issues/research only (today's user request).
- `GO_R0_FIXTURES` = separate explicit approval for only disposable synthetic repositories/measurements.
- `GO_R1`…`GO_R7` = separate scoped authorizations after each predecessor's evidence.
- `GO_PRODUCTION` = future release authorization, not implied.

## Pending architecture decisions
- RF-RFC-001..012 retain historical `PROPOSED` meanings. Proposed RF-RFC-013..024 are only ideas.
- No new file may silently mark a Ravum ADR or Flow ADR `ACCEPTED`.
- No GPL/FSL/copyleft code reuse without precise license/legal assessment. No implicit MIT/Apache license in Flow.

## Evidence labeling
`CODE_READ`, `SOURCE_READ`, `MEASURED_HISTORICAL`, `PLAUSIBLE_NOT_REPRODUCED`, `NOT_REVIEWED`, `PROPOSED`, `UNKNOWN`. Human verdict documented independently from model opinions.