# Ravum ↔ Ravum Flow: ownership, integration and rights

| Responsibility | Owner | Flow relationship |
|---|---|---|
| Ravum language syntax, compiler, IR, memory/ownership, effects | **Ravum** | consume declared adapter capabilities |
| Semantic proof solver, bounded certificates, proof trust base | **Ravum** | record scoped receipt/evidence only |
| ActiveVersionId, PatchHost, hot swap, rollback, real-time audio | **Ravum** | never activate patches; no code in RT callback |
| Generic multi-agent patch admission, attribution, coordination | **Flow candidate** | no duplicate long-term coordinator |
| Generic Git trees, publication intent, PR/forge integration | **Flow candidate** | native Git remains independently usable |
| Build/test via Cargo/tsc/pytest/CMake etc. | language adapter | existing toolchains, conservative invalidation |

**Ravum is active in another session. READ-ONLY for Flow agents.** Any P0 source finding should be sent as an evidence-based handoff (no commit, PR, branch changes).

## Prototype audit before code reuse
Candidates to inspect in Ravum: `ravum-snapshot`, `ravum-workspace-coordinator`, `ravum-daemon`, `ravum-protocol`, `ravum-unit-graph`, `ravum-collab-oracle`, `ravum-batch`, `ravum-rust-adapter`.
**Do not** misclassify `ravum-store`: it is the M4 Todo SQLite boundary, **not** a reusable generic CAS.

For each candidate record upstream SHA, API signatures, deps, tests, authorship, license/right to transfer, and who will own its future changes. Alternatives: stable adapter/protocol, shared legitimately licensed library or narrowly scoped clean-room implementation. No cyclic dependency or silent code copy.

## Historical baseline
Claude final review inspected parts of Ravum `dev@5408d9b3...` (2026-11-09). Rev could advance; verify HEAD and blob SHA before making a new claim. Original 42 Ravum ADRs are reproduced in [V6](reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md), with primary links to Ravum. They all remain `PROPOSED`.

## Handoff topics (not demonstrated vulnerabilities)
P0-TREE (verified source snapshot vs filtered published Git tree), P0-HOOK-ORDER (wrong ordering, pre-hook tree, actual working tree), P0-ENV (inherited environment), P0-LEGACY (V1 text-only AutoSafe). P1-FILES (UTF-8-only path/modes), P1-STORE (wrong reuse assumption). Reproduce in throwaway fixture, no real secrets and no live-source modifications.