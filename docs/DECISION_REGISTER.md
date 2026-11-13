# Decision register — HUMAN SIGN-OFF REQUIRED
No choice below is accepted merely because documentation has been uploaded. Mark human decisions with date, actor, evidence and constraints.

| ID | Decision | Current state | Candidate direction | Evidence required |
|---|---|---|---|---|
| H1 | Provisional design O0/O1/O2/O3 | OPEN | Compare O0, O1, O3; keep O2 as subset | S9 + safety + maintainability |
| H2 | License, rights, extraction vs clean-room | **HUMAN_DECIDED 2026-11-09**: MPL-2.0 for original Flow material (PR #49 main, #50 dev). Copying/deriving external code still **OPEN** | provenance + clean-room review before any external reuse | rights inventory; per-file third-party declaration |
| H3 | Untrusted sandbox Windows/Linux policy | OPEN | refuse/fail-closed where isolation unsupported | actual OS-level containment tests |
| H4 | S9 threshold and resource budget | OPEN — **no numbers granted** | pre-register paired difference, cost and stop criteria | human-set numeric threshold/budget before run |
| H5 | R0 fixtures authorization, then minimal implementation | **HUMAN_DECIDED 2026-11-09** (owner-delegated GOAL, bounded) | R0-a/b/c on synthetic fixtures, then smallest useful core only if O0 is demonstrably insufficient | this GOAL's scope; results in `FLOW-FOUNDATION-VERIFIED-REPORT.md` |
| H6 | Handoff P0 to separate Ravum session | OPEN | send risk report, never edits | session priority / human agreement |
| H7 | Internal-first use vs independent product | **HUMAN_DECIDED 2026-11-13**: independent product, evaluated on real projects (mandate below) | production-grade envelope and demonstrable advantage, not only internal use | real-project pilot evidence, maintenance cost |
| H8 | Stop rule for documentary reviews | OPEN (**new**, proposed) | no new review dossier unless a decision is blocked; every review ends with GO/NO-GO + one concrete artifact | observed review cost vs decision throughput |
| D0 | source, provenance and rights inventoried | PARTIAL | verify GitHub actual refs and rights | citations, SHA, 42 ADR audit |
| D1 | independent contradictory review | DOCUMENT-CONDITIONAL | Claude V5 reviewed; V6 corrected afterwards | unresolved differences + provenance |
| D2 | security and transactional contract | PROPOSED | tree/hook/context/receipt/fencing rules | negative test specifications; no claims of tests run |
| D3 | S9 protocol frozen | DRAFT | paired benchmark with real worktrees and tools | budget, threshold, seeds, gates |
| D4 | reviewed human decision and scoped GO | PARTIAL (planning + R0 granted; D4 not closed) | separate planning, fixture and implementation permissions | dated explicit acceptance |

## Mandate of 2026-11-09 (bounded, second wave) — recorded, not ratified
The owner message that opened the MVP 0.1 wave is a **bounded technical mandate**, not a decision:
it authorizes Flow branches, synthetic fixtures, tests, CI and **draft** PRs only. It does not
ratify `H1`/`H3`/`H4`/`H6`/`H7`/`H8`, does not authorize a merge into `dev`/`main`, does not change
branch protection, does not authorize a release or real secrets, and does not authorize any write,
build or branch in `Rwanbt/Ravum`.

| Recorded fact | Value |
|---|---|
| Branch | `feat/flow-mvp-0.1`, stacked on `fix/flow-foundation-proof-gates` (`d884e29`) |
| PR | #53 (draft, base `fix/flow-foundation-proof-gates`) |
| Scope actually executed | lock contract `flow-lock/2`; bounded Git execution; proposals/candidates; candidate `run`/`publish`; `--dry-run`; policy adoption gate; symref refusal; polyglot fixtures + demo; tests; CI |
| Not executed | confirmatory S9 (`H4`), OS sandbox (`H3`), hook execution, ADR ratification, any merge |
| Performance claim | none — `PERFORMANCE_UNPROVEN`; the existing S9 pilot stays exploratory |

## Scope of authorizations
- `GO_PLANNING` = documents/issues/research only.
- `GO_R0_FIXTURES` = **GRANTED 2026-11-09** by the owner's operational GOAL, bounded to: branch `feat/flow-verified-foundation`, disposable synthetic fixtures, fictitious canaries, no real credentials, no Ravum modification, pilot-scale measurement only.
- `GO_R1`…`GO_R7` = separate scoped authorizations after each predecessor's evidence. The same GOAL permits only the *smallest useful core* after the R0 gate, with tests and CI; it is not a general implementation permission.
- `GO_MVP_0_1` = **GRANTED 2026-11-09** by the owner message (second wave), bounded to: branches
  `feat/flow-mvp-0.1` and the stacked bases, disposable synthetic fixtures, fictitious canaries,
  tests/CI/draft PRs, no merge, no release, no ADR ratification, no Ravum write.
- `GO_PRODUCTION` = future release authorization, not implied.

## Mandate of 2026-11-13 (production) — recorded from the owner's instruction
The owner's message that opened this wave is a **scoped GO for production work on this repository**, and it
also answers `H7`. It is recorded here, not inferred.

| Field | Value |
|---|---|
| `GO(scope)` | merge the production candidate into `dev` then `main`; perform the missing qualifications **by measurement**; fix the documentary drifts; demonstrate the product on real projects |
| `repository` | `Rwanbt/Ravum-flow` only — `Rwanbt/Ravum` remains read-only and out of scope |
| `branches` | `feat/flow-production-candidate` → `dev` → `main`; short-lived work branches; tags and GitHub Releases for this repository |
| `allowed_paths` | `crates/flow/**`, `.github/**`, `docs/**`, `scripts/**`, `examples/**`, `experiments/quality/**` (evidence) |
| `budgets` | reference machine 16 GB RAM; **no concurrent heavy Cargo campaigns**; mutation/fuzz campaigns run in CI or bounded per module; no unbounded local benchmark |
| `evidence` | CI run ids per head, measured test counts, coverage, mutation outcomes, crash-harness logs, release artefacts with provenance |
| `stop_conditions` | a red required check; any credential or real secret in a fixture/log; any write to `Rwanbt/Ravum`; any destructive Git operation on user files; a claim that cannot be tied to a cited measurement |
| `definition of production ready` | owner, 2026-11-13: *"prêt et efficace pour un usage en production sur de vrais projets et apporte de réels avantages par rapport aux autres systèmes déjà existants pour le workflow de travail en collaboration"* |

**`H7` is decided by that message:** the target is **an independent product evaluated on real projects**, not an
internal-only tool. The consequences are therefore the "produit indépendant" column of the readiness matrix
(real-project envelope, demonstrable advantage, collaboration quality), and the evidence gap `PA-B13` (S9) is a
product requirement rather than an optional study.

**Still not ratified by this GO:** `H1` (provisional architecture option), `H3` (untrusted sandbox), `H4` (S9
thresholds and budget), `H6` (Ravum handoff), `H8` (review stop rule), any `RAVUM-ADR-*` or `FLOW-ADR-*`, and
`G10` (which requires a reviewer outside the team). `GO_PRODUCTION` itself remains the owner's act.

## Decisions this GOAL does **not** ratify
The owner's operational GOAL is not a ratification of `H1`, `H3`, `H4`, `H6`, `H7`, `H8`, of any `RAVUM-ADR-*`, or of any `FLOW-ADR-*`. `H2` was decided separately by the owner (MPL-2.0, PRs #49/#50). The GOAL does not close `D3` and does not authorize a merge into `dev` or `main`.

## Pending architecture decisions
- RF-RFC-001..012 retain historical `PROPOSED` meanings. Proposed RF-RFC-013..024 are only ideas.
- No new file may silently mark a Ravum ADR or Flow ADR `ACCEPTED`.
- Original Flow material is licensed MPL-2.0 by its rightsholders. No GPL/FSL/copyleft or external code reuse without a precise licence/rights assessment; no implicit MIT/Apache grant.
- Canonical ADR namespaces: `RAVUM-ADR-*` vs `FLOW-ADR-*` (see [TRACEABILITY](TRACEABILITY.md)); bare `ADR-0NN` is ambiguous.

## Evidence labeling
`CODE_READ`, `SOURCE_READ`, `MEASURED_HISTORICAL`, `MEASURED`, `PLAUSIBLE_NOT_REPRODUCED`, `NOT_REVIEWED`, `PROPOSED`, `UNKNOWN`. Human verdict documented independently from model opinions.
