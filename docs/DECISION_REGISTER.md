# Decision register — HUMAN SIGN-OFF REQUIRED
No choice below is accepted merely because documentation has been uploaded. Mark human decisions with date, actor, evidence and constraints.

| ID | Decision | Current state | Candidate direction | Evidence required |
|---|---|---|---|---|
| H1 | Provisional design O0/O1/O2/O3 | **HUMAN_DECIDED 2026-11-15: O3 complet** (owner, via the session questionnaire). **O3 is the target, not what exists**: the branch implements the O1 + O2 subset (local CLI, exact-tree verification, CAS publication); orchestration/distribution (O3) is **not designed, budgeted or measured** and needs its own ADR, budget and P0 review before any claim. Four shapes and their tradeoffs are compared in [FLOW-O3-PROPOSALS.md](reviews/FLOW-O3-PROPOSALS.md) (local daemon, network coordinator, Git federation, embedded protocol) | O3 as the long-term target; the shipped subset stays labeled as such | O3 design + budget + review (not satisfied by this branch) |
| H2 | License, rights, extraction vs clean-room | **HUMAN_DECIDED 2026-11-09**: MPL-2.0 for original Flow material (PR #49 main, #50 dev). Copying/deriving external code still **OPEN** | provenance + clean-room review before any external reuse | rights inventory; per-file third-party declaration |
| H3 | Untrusted sandbox Windows/Linux policy | **HUMAN_DECIDED 2026-11-15: refuse/fail-closed where OS isolation is absent** (owner). The product stays `local-trusted`, untrusted execution is `UNSUPPORTED`, and `doctor` states it | fail-closed; no confinement claim | no OS containment tests exist, and none is claimed |
| H4 | S9 threshold and resource budget | **HUMAN_DECIDED 2026-11-15: granted** (owner): confirmatory paired campaign, **10 pairs, 2 cores / 8 GB, stop if Flow is >2× slower or variance is unexplained; success = ≥30 % less wall-clock to a verified publish, p < 0.05** | pre-registered threshold and budget | campaign not yet run; until it is, productivity stays `UNPROVEN` |
| H5 | R0 fixtures authorization, then minimal implementation | **HUMAN_DECIDED 2026-11-09** (owner-delegated GOAL, bounded) | R0-a/b/c on synthetic fixtures, then smallest useful core only if O0 is demonstrably insufficient | this GOAL's scope; results in `FLOW-FOUNDATION-VERIFIED-REPORT.md` |
| H6 | Handoff P0 to separate Ravum session | **HUMAN_DECIDED 2026-11-15: draft handoff authorized, no write into `Rwanbt/Ravum`** (owner). The draft is [RAVUM-HANDOFF-DRAFT-2026-11-15.md](reviews/RAVUM-HANDOFF-DRAFT-2026-11-15.md); transmission to the owning session is the owner's act | send the evidence-based draft; never edit Ravum | Flow has no channel into the Ravum session; the draft stays local until the owner transmits it |
| H7 | Internal-first use vs independent product | **HUMAN_DECIDED 2026-11-13**: independent product, evaluated on real projects (mandate below) | production-grade envelope and demonstrable advantage, not only internal use | real-project pilot evidence, maintenance cost |
| H8 | Stop rule for documentary reviews | **HUMAN_DECIDED 2026-11-15: adopted** (owner): no new review dossier unless a decision is blocked; every review ends with `GO`/`NO-GO` and one concrete artifact | adopted rule | applies from this date; no retroactive re-scoring |
| D0 | source, provenance and rights inventoried | PARTIAL | verify GitHub actual refs and rights | citations, SHA, 42 ADR audit |
| D1 | independent contradictory review | DOCUMENT-CONDITIONAL | Claude V5 reviewed; V6 corrected afterwards | unresolved differences + provenance |
| D2 | security and transactional contract | PROPOSED (ADR-012, plus the 2026-11-15 review notes) | tree/hook/context/receipt/fencing rules; probes in the approved policy; publish checks log bytes | negative test specifications; ADR-012 ratification is post-`G10` |
| D3 | S9 protocol frozen | **FROZEN 2026-11-15 by `H4`** (thresholds + budget); campaign **NOT RUN** | paired benchmark with real worktrees and tools | run the pre-registered campaign; until then productivity is `UNPROVEN` |
| D4 | reviewed human decision and scoped GO | PARTIAL: human decisions `H1`/`H3`/`H4`/`H6`/`H8` recorded 2026-11-15; `G10` and `GO_PRODUCTION` outstanding | separate planning, fixture and implementation permissions | external review verdict + owner `GO_PRODUCTION` |

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

## Decisions of 2026-11-15 (owner's answers, recorded verbatim)

The owner answered the questionnaire of [FLOW-HUMAN-DECISIONS-2026-11-15.md](reviews/FLOW-HUMAN-DECISIONS-2026-11-15.md)
in session. The answers are recorded as given, including the one that departs from the recommendation:

| Question | Owner's answer | Consequence recorded here |
|---|---|---|
| H1 | **"O3 complet"** | O3 is the target architecture. The branch implements only the O1 + O2 subset; O3 (orchestration/distribution) has **no design, no budget, no measurement and no P0 review** and is a separate workstream. No document may present O3 as delivered. |
| H3 | **"Refus/fail-closed hors isolation OS"** | The product keeps `local-trusted`; untrusted execution is `UNSUPPORTED`; no confinement is claimed. |
| H4 | **"Oui : ≥30 % de temps en moins, p<0,05, 10 paires"** | The S9 thresholds and budget are frozen as stated; the campaign is **not yet run**, so productivity remains `UNPROVEN` until it is. |
| H6 | **"Les trois recommandations"** (handoff draft, no Ravum write) | The draft [RAVUM-HANDOFF-DRAFT-2026-11-15.md](reviews/RAVUM-HANDOFF-DRAFT-2026-11-15.md) is authorized; transmission is the owner's act; Flow never writes `Rwanbt/Ravum`. |
| H8 | **"Les trois recommandations"** (review stop rule) | The rule is adopted from 2026-11-15. |
| ADR-012 | **"Les trois recommandations"** (stay `PROPOSED` until G10, then ratify) | ADR-012 remains **PROPOSED**; its ratification is a post-`G10` act. |

`G10` (reviewer outside the team) is **not** satisfied by this record: it remains an open gate, and the
verdict stays `PRODUCTION_PARTIAL` until it is passed and `GO_PRODUCTION` is granted.

## Mandate of 2026-11-15 (Fable hardening) — granted by the owner in session
The owner granted the following scoped GO on 2026-11-15, after reading the Fable counter-audit
(`RAVUM_FLOW_FABLE_AUDIT.md`, SHA `1952325c`). It is recorded here verbatim; it is a **production
hardening** authorization, not `GO_PRODUCTION` and not a release decision.

| Field | Value |
|---|---|
| `GO(scope)` | Corrections FBL-001..FBL-012 du contre-audit Fable du 2026-11-15 ; tests de régression G1–G8 ; adaptations de schémas et migration ; refactoring strictement nécessaire ; CI/documentation/ADR ; benchmarks bornés ; commits et PR sur une branche feature. Périmètre produit : local-trusted mono-hôte, Git en sous-processus, aucune promesse de sandbox OS ni de système multi-hôtes. |
| `repository` | `Rwanbt/Ravum-flow` uniquement ; `Rwanbt/Ravum` reste en lecture seule, hors périmètre |
| `branch` | `feat/flow-production-hardening-fable` créée depuis `origin/dev` (`2ab79c7`) ; branches courtes dérivées autorisées ; aucun force-push, aucun merge `dev`/`main` |
| `allowed_paths` | `crates/flow/**`, `tests/**`, `docs/**`, `.github/**`, `scripts/**`, `examples/**`, `experiments/quality/**` |
| `budgets` | machine de référence 16 Go RAM ; pas de campagnes Cargo lourdes concurrentes ; benchmarks et fuzz bornés (local ou CI), aucun run illimité |
| `evidence` | `docs/reviews/FLOW-POST-FABLE-PRODUCTION-HARDENING.md` ; commandes, OS, exit codes, digests, run ids CI, résultats G1–G8 |
| `stop_conditions` | check requis rouge ; secret réel dans un log/fixture ; écriture dans `Rwanbt/Ravum` ; opération Git destructive sur fichiers utilisateur ; affirmation non traçable à une mesure ; P0 encore ouvert au verdict final |

**Explicitly excluded by the owner:** merges into protected branches, tags, releases, and any change
to `Rwanbt/Ravum`. **Still not ratified:** `H1`, `H3`, `H4`, `H6`, `H8`, any `FLOW-ADR-*` /
`RAVUM-ADR-*`, coverage/mutation thresholds, and `GO_PRODUCTION` itself.

## Decisions this GOAL does **not** ratify
The owner's operational GOAL is not a ratification of `H1`, `H3`, `H4`, `H6`, `H7`, `H8`, of any `RAVUM-ADR-*`, or of any `FLOW-ADR-*`. `H2` was decided separately by the owner (MPL-2.0, PRs #49/#50). The GOAL does not close `D3` and does not authorize a merge into `dev` or `main`.

## Pending architecture decisions
- RF-RFC-001..012 retain historical `PROPOSED` meanings. Proposed RF-RFC-013..024 are only ideas.
- No new file may silently mark a Ravum ADR or Flow ADR `ACCEPTED`.
- Original Flow material is licensed MPL-2.0 by its rightsholders. No GPL/FSL/copyleft or external code reuse without a precise licence/rights assessment; no implicit MIT/Apache grant.
- Canonical ADR namespaces: `RAVUM-ADR-*` vs `FLOW-ADR-*` (see [TRACEABILITY](TRACEABILITY.md)); bare `ADR-0NN` is ambiguous.

## Evidence labeling
`CODE_READ`, `SOURCE_READ`, `MEASURED_HISTORICAL`, `MEASURED`, `PLAUSIBLE_NOT_REPRODUCED`, `NOT_REVIEWED`, `PROPOSED`, `UNKNOWN`. Human verdict documented independently from model opinions.

