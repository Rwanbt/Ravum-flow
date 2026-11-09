# Ravum Flow
> **Status: planning / research only — no engine implemented.**  
> Current baseline: **V6 consolidated after Claude review (9 November 2026)**. All Flow ADRs/RFCs are **PROPOSED**, never automatically accepted.

**Ravum Flow** is an independent, local-first, Git-compatible and language-agnostic proposed workflow for humans and AI coding agents. Its purpose is to reduce **time to a verified deliverable**, preserve user edits, attribute concurrent proposals and avoid *mandatory persistent* Git worktrees **where safe and useful**.

It is **not** the [Ravum programming language](https://github.com/Rwanbt/Ravum), a compiler, a VCS replacement, a forge, a cloud prerequisite or an unconditional proof system. Ravum can later consume Flow via an optional adapter; Flow must work without Ravum.

## Work starts with evidence, not assumptions

- **O0 — no new engine:** Git + private copies / ephemeral worktrees + existing CI tools.
- **O1 — adopt existing tools:** study jj, GitButler, Scion Google, etc. against real requirements and licenses.
- **O2 — thin Flow:** managed patches, frozen Git candidate tree, exact-tree verification receipts, controlled Git publication.
- **O3 — hybrid:** O2 plus disposable isolated execution for ordinary tools, hooks and builds.
- **O4 — VFS/CRDT/distributed:** deferred unless an independently justified need is proven.

One physical workspace does **not** give arbitrary processes distinct isolated views. No untrusted test/build/hook should inherit private credentials. Failed or missing mandatory E2E checks must never become green.

## Where to start

- [Full, verbatim V6 archive (42 Ravum ADRs included)](docs/reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md)
- [Project charter](docs/PROJECT_CHARTER.md) · [Architecture](docs/ARCHITECTURE.md)
- [Decision register](docs/DECISION_REGISTER.md) · [Proposed Flow ADRs](docs/adr/README.md)
- [Historic Flow RFCs](docs/rfc/README.md) · [Roadmap/gates](docs/ROADMAP.md)
- [User stories](docs/USER_STORIES.md) · [26 historic RF tasks](docs/BACKLOG.md)
- [S9 experiment design](docs/BENCHMARK_S9.md) · [Security test matrix](docs/SECURITY_TESTS.md)
- [Ravum / Flow boundary](docs/RAVUM_BOUNDARY.md) · [Traceability](docs/TRACEABILITY.md)
- **[DeepSeek multi-agent GOAL — first review/decision phase](docs/DEEPSEEK_GOAL.md)**

## Repository workflow

`main` is the documentary baseline; `dev` hosts proposed planning changes and later narrowly authorized experiments. Prefer feature branch → PR into `dev`, and reviewed promotion into `main`. No code execution or implementation is authorized merely by this README. Branch protections must be configured separately by an authorized admin.

## Human decisions still pending

H1 select a provisional option; H2 code ownership/licensing vs clean-room; H3 Windows/Linux sandbox guarantee; H4 S9 thresholds and budget; H5 whether to allow disposable R0 fixtures; H6 whether to hand P0 findings to the separate Ravum session. See the decision register.

**License:** no project LICENSE has been selected or published. Do not presume an open-source grant or copy Ravum source into Flow without rights review.