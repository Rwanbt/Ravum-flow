# Documentation index

- [V6 master reference, 42 Ravum ADRs and Claude report](reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md)
- [Project charter](PROJECT_CHARTER.md)
- [Architecture](ARCHITECTURE.md)
- [Capabilities and their evidence](CAPABILITIES.md) — what is available, demonstrated, specified-only or not available
- [Human decision register](DECISION_REGISTER.md)
- [Roadmap / gates](ROADMAP.md)
- [User stories](USER_STORIES.md)
- [Backlog RF-001..026](BACKLOG.md)
- [GitHub issue map — 47 tracked issues](ISSUE_MAP.md)
- [main/dev branch workflow and administrative protections](BRANCH_WORKFLOW.md)
- [Benchmark S9](BENCHMARK_S9.md)
- [Security negative-test matrix](SECURITY_TESTS.md)
- [Ravum / Flow boundary](RAVUM_BOUNDARY.md)
- [Traceability](TRACEABILITY.md)
- [New Flow ADRs](adr/README.md)
- [Historic Flow RFC proposals](rfc/README.md)
- [DeepSeek GOAL](DEEPSEEK_GOAL.md)

## DeepSeek D0–D4 review dossier (2026-11-09)

Ten consolidated deliverables, plus eight raw sub-agent reports in [`reviews/raw/`](reviews/raw/):

- [Summary, verdict and human handoff](reviews/DEEPSEEK-SUMMARY.md) — **start here**
- [00 — Baseline, provenance, rights](reviews/DEEPSEEK-00-BASELINE.md)
- [01 — Prior art and O0–O4 comparison](reviews/DEEPSEEK-01-PRIOR-ART.md)
- [02 — ADR/RFC matrix and ratification order](reviews/DEEPSEEK-02-ADR-RFC-MATRIX.md)
- [03 — Threat model, P0 and forbidden promises](reviews/DEEPSEEK-03-THREAT-MODEL.md)
- [04 — Pre-registered S9 protocol](reviews/DEEPSEEK-04-S9-PROTOCOL.md)
- [05 — Optimized roadmap, RF/US trims, candidate architecture](reviews/DEEPSEEK-05-OPTIMIZED-ROADMAP.md)
- [06 — Decisions H1–H6 (+H7–H8) to freeze](reviews/DEEPSEEK-06-DECISIONS-TO-FREEZE.md)
- [07 — Adversarial dissent and resolved/open disagreements](reviews/DEEPSEEK-07-ADVERSARIAL-DISSENT.md)
- [Execution GOAL draft — **not launched**](reviews/DEEPSEEK-EXECUTION-GOAL-DRAFT.md)

No ADR/RFC is ratified, no GO is granted by this dossier alone, and no code was written by it.

## Branch and authorization status (2026-11-09)

- `docs/deepseek-d0-d4-review@f113791` is **isolated**: never merged into `dev`, kept for provenance; PR #33 is untouched. The dossier is therefore *not* part of `dev` documentation yet.
- Canonical form corrections (`X-05` state machine, `X-06` S9 letters, `X-07` test IDs, `X-11` ADR namespaces, S8 metric framing, `H5`/`H7`/`H8`) are applied on `feat/flow-verified-foundation`, branched from `f113791`.
- `GO_R0_FIXTURES` was **granted on 2026-11-09** by the owner's operational GOAL, bounded to synthetic fixtures and this branch; it ratifies no ADR, no license and no S9 threshold.
- The execution phase (R0-a/b/c, minimal CLI, CI, draft PR #51) is reported in the [verified-foundation report](reviews/FLOW-FOUNDATION-VERIFIED-REPORT.md).

**All technical choices remain proposals until a human sign-off.**

## Production candidate dossier (2026-11-10, `feat/flow-production-candidate`)

Read in this order; each document states the SHA and OS its numbers were taken on.

- [Capabilities and their evidence](CAPABILITIES.md) — the qualification matrix (status vocabulary, limits, no inferred capability)
- [Gate report G10](reviews/FLOW-PRODUCTION-GATE-REPORT.md) — `PRODUCTION_PARTIAL`, the 24 invariants and the evidence per gate
- [Invariants and proofs G1](reviews/FLOW-PRODUCTION-INVARIANTS.md) — one executed test per invariant
- [Baseline G0](reviews/FLOW-PRODUCTION-BASELINE.md) · [Compatibility G5](reviews/FLOW-PRODUCTION-COMPATIBILITY.md) · [Test matrix G6](reviews/FLOW-PRODUCTION-TEST-MATRIX.md) · [Release checklist G7](reviews/FLOW-PRODUCTION-RELEASE-CHECKLIST.md)
- [Adversarial campaign](reviews/FLOW-PRODUCTION-ADVERSARIAL.md) and [its wave-3 replay](reviews/FLOW-PRODUCTION-ADVERSARIAL-WAVE3.md)
- [Second independent review](reviews/FLOW-PRODUCTION-SECOND-REVIEW.md)
- [Wave 3 — git fidelity, secrets, crash and artefacts](reviews/FLOW-PRODUCTION-WAVE3.md)
- [Packaging and artefacts](reviews/FLOW-PACKAGING-WAVE3.md) · [Quality measurements](reviews/FLOW-QUALITY-WAVE3.md) · [Quality wave 4 — coverage gate met, mutation widened](reviews/FLOW-QUALITY-WAVE4.md) · [Quality wave 5 — thresholds raised to 95/90 and enforced in CI](reviews/FLOW-QUALITY-WAVE5.md) · [Quality wave 6 — the four remaining mutation modules and a randomised parser sweep](reviews/FLOW-QUALITY-WAVE6.md) · [Quality wave 7 — the last assertion gaps and the Unix sites measured on Linux](reviews/FLOW-QUALITY-WAVE7.md) · [Quality wave 8 — the module rates on Linux and the last survivors](reviews/FLOW-QUALITY-WAVE8.md) · [Quality wave 9 — the host-gated mutants and a clean stability baseline](reviews/FLOW-QUALITY-WAVE9.md)
- [Planning wave 9 — H1 options](planning/2026-11-11-flow-h1-options.md) · [H3 sandbox comparison](planning/2026-11-11-flow-h3-sandbox-comparison.md) · [H5 consumed GO](planning/2026-11-11-flow-h5-consumed-go.md) · [RF-001 rights](planning/2026-11-11-flow-rf001-rights.md) · [RF-003 S9 pre-registration](planning/2026-11-11-flow-rf003-s9-preregistration.md) · [G10 maintainability review](reviews/FLOW-PRODUCTION-G10-MAINTAINABILITY-REVIEW.md)
- [Decision dossiers — every pending human decision, answerable in one pass](planning/2026-11-12-flow-decision-dossiers.md)
- [Git-fidelity independent review](reviews/FLOW-P0-GIT-FIDELITY-REVIEW.md)
