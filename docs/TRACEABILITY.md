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

## Canonical ADR namespaces (`X-11`)
Two independent series share the numbers 000–011. A bare `ADR-000` is ambiguous and must no longer appear in new or edited text.

| Canonical name | Series | Location | Count | Status |
|---|---|---|---|---|
| `RAVUM-ADR-000` … `RAVUM-ADR-041` | Ravum language/architecture history, preserved in V6 | V6 archive + Ravum `docs/adr` (read-only) | 42 | all `PROPOSED` in Flow; none ratified |
| `FLOW-ADR-000` … `FLOW-ADR-011` | Flow deliberation records | `docs/adr/FLOW-ADR-0NN.md` | 12 | all `PROPOSED` |
| `FLOW-INV-01` … `FLOW-INV-12` | Flow invariants | V6 archive | 12 | all `PROPOSED` |
| `RF-RFC-001` … `RF-RFC-012` | historical Flow RFCs | `docs/rfc/` | 12 | `PENDING`/`PROPOSED` unchanged |
| `RF-RFC-013` … `RF-RFC-024` | later ideas | `docs/rfc/` | 12 | ideas, not accepted |

**Rule:** the file header, index entry and every cross-reference use the prefixed form. Aliases such as "ADR-003 (Flow)" or "ADR-000 (Ravum)" are tolerated only inside quotations of older documents and must be rewritten when that passage is edited. The Ravum session owns `RAVUM-ADR-*`; Flow agents never edit them and never mark any ADR `ACCEPTED`.

## Required mapping before R1
Every issue RF-001..026 must record dependencies, an owner, new phase R0..R7, acceptance test and gate; reclassification does not erase its historic wording. Check it in [backlog](BACKLOG.md).
## MVP 0.1 — requirement → test → evidence

| Requirement (mandate §1.1) | Implementation | Test / evidence |
|---|---|---|
| 1. `init`/`doctor` state limits honestly | `cmd_init`, `cmd_doctor` | `doctor_is_read_only_and_reports_guarantees`; doctor reports lock, adoption, non-guarantees |
| 2. Agent proposal attributed, pinned base, no write to the user's worktree/index | `proposal::admit` (temporary `GIT_INDEX_FILE`) | `proposal` admission tests, M6 snapshot |
| 3. Durable, idempotent, immutable proposal | `.flow/proposals/<id>.{json,patch}` | `admission_is_idempotent_and_immutable` |
| 4. Collision-free exact candidate tree, no checkout | `proposal::assemble` | M1, M2, `assembly_reports_order_and_stays_deterministic` |
| 5. Obligations run on that exact tree, receipt binds identity/context/tree/scope | `cmd_run` (existing engine) + `candidate_id` | M1 receipt assertions; P0-A/P0-B gates |
| 6. Publish only the verified tree, CAS + journal, restart/reconcile | `cmd_publish` + `cmd_reconcile` | M5 crash tests, `m1` tree equality |
| 7. External advance ⇒ STALE/REFUSED, user files intact | `cmd_publish` CAS + `BASE_MISMATCH` | M3, M6 |
| 8. `status`/`explain` observable in JSON | `cmd_status` (`flow-status/1`), `cmd_explain` | CLI tests parse the JSON; `explain --candidate` documented as `NOT_RUN` scenario |
| 9. No LLM, no network, no Ravum dependency | zero-dependency crate, offline toolchains | `Cargo.toml` (no deps), polyglot demo offline, no `Ravum` access during the wave |