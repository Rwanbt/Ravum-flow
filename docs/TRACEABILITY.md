# Traceability contract
All identifiers are historical unless marked as newly **PROPOSED**. No AI may ratify an ADR.

| Source baseline | Full source | Treatment in Flow |
|---|---|---|
| Ravum P0-A..P0-I | [V6 archive](reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md) + Ravum primary `docs/architecture/PROJECT_INVARIANTS.md` | 9/9 preserved; Flow owns workflow not language |
| Ravum ADR-000..041 | V6 original full text + upstream source `docs/adr` | 42/42 `PROPOSED`; no changes |
| Flow INV-01..12 | V6 archived v0.1 | 12/12 `PROPOSED`; INV-01-A amendment explicit |
| Flow RF-RFC-001..012 | V6 archived decisions + [rfc register](rfc/README.md) | 12/12 `PROPOSED` |
| Flow RF-RFC-013..024 | V6 newer review proposals | 12 ideas, NOT ACCEPTED |
| Historical RF-0..9 | V6 archived plan + [roadmap](ROADMAP.md) | mapped to D0–D4 / R0–R7 |
| RF-001..026 | V6 archived backlog + [backlog](BACKLOG.md) | 26/26 retained; individual phase mapping is proposed |
| Claude final V5 review | V6 final annex | DOCUMENT-CONDITIONAL; V6 corrections not independently re-signed |

## ADR/RFC mapping
Flow ADR-000–011 in `docs/adr` are **new proposed deliberation records**, never aliases of Ravum ADR-000..041. RF-RFC-001..012 keep original numbers and substance.

## Required mapping before R1
Every issue RF-001..026 must record dependencies, an owner, new phase R0..R7, acceptance test and gate; reclassification does not erase its historic wording. Check it in [backlog](BACKLOG.md).