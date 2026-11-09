# Ravum Flow

**Status: DESIGN / PLANNING — NOT IMPLEMENTED**

Ravum Flow is a proposed **local-first, language-agnostic and Git-compatible** workflow for people and AI agents to collaborate on code with traceable changes, exact-tree verification, and no *mandatory persistent* Git worktree per agent.

- **Project:** [Ravum Flow](https://github.com/Rwanbt/Ravum-flow) (independent tool; not a programming language)
- **Companion language project:** [Ravum](https://github.com/Rwanbt/Ravum) (separate repository and implementation lifecycle)
- **Current decision:** `DOCUMENT-CONDITIONAL` after Claude review. Do **not** interpret this as authorization to develop or benchmark.
- **Default branch:** `main` for approved planning baseline; `dev` for proposed ADRs, research, and future scoped work.
- **V6 source:** Ravum Flow V6 consolidated review (2026-11-09), including 42 Ravum ADRs, 12 original Flow RFCs, threat model and benchmark gates.

## Product objective

Shorten **time to verified deliverable** without silently losing changes or hiding required E2E failures. Compare the no-new-engine option (Git/copies/worktrees) with existing tools and a minimal Flow design before building a proprietary CAS/VFS.

## Governance and safety

All Flow ADRs are **PROPOSED**, not ratified. The 42 historical Ravum ADRs also remain **PROPOSED**. No Ravum code is copied into this repo. **License: [Mozilla Public License 2.0](LICENSE)** for original project material whose contributors have the relevant rights. This does not override licenses or notices attached to third-party content. The H2 ownership/provenance and clean-room review remains open; no Ravum source is licensed for copying into Flow by this decision.

The launch must be staged: **D0–D4 documentary gates** first, then separately authorized `GO_R0_FIXTURES` on disposable synthetic repositories. No permission for R1–R7 is implied.

For architecture, decisions, user stories, milestones and DeepSeek handoff, see the `docs/` directory on `dev` once initial planning content is committed.
