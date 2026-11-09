# Documentation index

- [V6 master reference, 42 Ravum ADRs and Claude report](reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md)
- [Project charter](PROJECT_CHARTER.md)
- [Architecture](ARCHITECTURE.md)
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