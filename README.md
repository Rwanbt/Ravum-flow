# Ravum Flow
> **Status: minimal MVP on `feat/flow-verified-foundation` — not production ready.**  
> This branch ships a small local CLI (`flow`) that verifies an exact Git tree and publishes it behind a compare-and-swap receipt. It is **not** a VCS, a build scheduler or a sandbox, and no ADR/RFC is ratified.  
> Planning baseline: **V6 consolidated after Claude review (9 November 2026)**. All Flow ADRs/RFCs remain **PROPOSED**, never automatically accepted.

**Ravum Flow** is an independent, local-first, Git-compatible and language-agnostic proposed workflow for humans and AI coding agents. Its purpose is to reduce **time to a verified deliverable**, preserve user edits, attribute concurrent proposals and avoid *mandatory persistent* Git worktrees **where safe and useful**.

It is **not** the [Ravum programming language](https://github.com/Rwanbt/Ravum), a compiler, a VCS replacement, a forge, a cloud prerequisite or an unconditional proof system. Ravum can later consume Flow via an optional adapter; Flow must work without Ravum.

## Minimal CLI — implemented and tested

```bash
cargo build --release
./target/release/flow init          # create .flow/ (config, journal, receipts)
./target/release/flow doctor        # read-only: git, hooks, toolchains, guarantee limits
git add -A
./target/release/flow run           # verify the INDEX tree in a disposable copy
./target/release/flow publish       # ref CAS + durable intent/outcome journal
```

| Command | State | Guarantee actually delivered |
|---|---|---|
| `init` | implemented | creates config/journal/receipts; refuses to overwrite without `--force` |
| `doctor` | implemented | read-only report; lists what is **not** guaranteed |
| `status` | implemented | head/tree, dirty and untracked counts, latest receipt, pending intents |
| `run` | implemented | exports the exact index tree to a directory Flow owns (`.git` link removed), runs declared obligations, writes a receipt bound to the tree OID **and** to the effective policy digest. `--obligation ID` is diagnostic only: the run is `PARTIAL` and is never publishable |
| `publish` | implemented (ref-only) | requires a **complete** `PASSED` receipt for that exact tree whose policy digest still matches the current config; verifies the receipt's integrity and every required obligation, appends a durable intent, re-reads the ref, then `git update-ref` CAS. Destination must be a full `refs/heads/...` ref; a tree the ref already carries is `ALREADY_PUBLISHED`. Outcomes `PUBLISHED` / `REFUSED` with a machine `code`; a crash window is classified by `reconcile` as `RECONCILED_PUBLISHED`, `RECONCILED_NOT_PUBLISHED` or `NEEDS_RECONCILIATION` |
| `reconcile` | implemented | observes the ref and records `PUBLISHED` / `NOT_PUBLISHED` / `NEEDS_RECONCILIATION`; never moves a ref. An unreadable journal is `CORRUPT_JOURNAL` or `UNKNOWN_OUTCOME` with `manual_required: true` and a non-zero exit — never a successful empty reconciliation |
| `explain` | implemented | receipt + journal entries for a tree |
| `disable` / `enable` | implemented | mutating commands refuse while disabled |

**Guarantees:** receipts bind an exact Git tree OID *and* one effective policy digest (policy version, execution boundary, every obligation's id/program/args/required/timeout, and the obligation environment allowlist); a receipt altered on disk is refused; a required obligation that cannot run yields `UNSUPPORTED`, never `PASSED`; a receipt produced under a weaker or different policy is `STALE_POLICY`; publication is ref-CAS with a fsynced intent before and an outcome after; dirty/untracked user files are never modified.

**Non-guarantees (by design in this MVP):** no OS sandbox (obligations run with your rights and a scrubbed environment), no execution of Git hooks, no completeness proof of the build context, no working-tree checkout after publishing, single local writer only. The receipt `integrity` digest detects accidental corruption and casual edits; it is not a signature and does not stop a writer who can rewrite the config and the receipt together. The local advisory lock serialises Flow processes but does not fence an external `git` writer. Untrusted code execution is `UNSUPPORTED`.

**Tested platforms:** Windows 11 + Rust 1.95 (local, see the verification report). Linux is exercised by CI only; a claim that is not in the report is not a claim.

See [the verified-foundation report](docs/reviews/FLOW-FOUNDATION-VERIFIED-REPORT.md) for evidence, measured limitations and residual risks.

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
- [User stories](docs/USER_STORIES.md)
- [Live GitHub issues: H1–H6, RF-001..026 and US-001..015](docs/ISSUE_MAP.md) · [main/dev branch workflow](docs/BRANCH_WORKFLOW.md) · [26 historic RF tasks](docs/BACKLOG.md)
- [S9 experiment design](docs/BENCHMARK_S9.md) · [Security test matrix](docs/SECURITY_TESTS.md)
- [Ravum / Flow boundary](docs/RAVUM_BOUNDARY.md) · [Traceability](docs/TRACEABILITY.md)
- **[DeepSeek multi-agent GOAL — first review/decision phase](docs/DEEPSEEK_GOAL.md)**

## Repository workflow

`main` is the documentary baseline; `dev` hosts proposed planning changes and later narrowly authorized experiments. Prefer feature branch → PR into `dev`, and reviewed promotion into `main`. No code execution or implementation is authorized merely by this README. Branch protections must be configured separately by an authorized admin.

## Human decisions still pending

H1 select a provisional option (OPEN); H2 licence — **decided by the owner on 2026-11-09: MPL-2.0** (provenance/clean-room review still required before reusing external source); H3 Windows/Linux sandbox guarantee (OPEN); H4 S9 confirmatory thresholds and budget (OPEN — the exploratory pilot ran without them); H5 disposable R0 fixtures (**granted 2026-11-09** by the owner for this branch); H6 P0 handoff to the separate Ravum session (OPEN); H7 internal-first use (OPEN); H8 stop rule for documentary reviews (OPEN). See the decision register.

**License:** [Mozilla Public License 2.0](LICENSE) applies to original Ravum Flow material licensed by its rightsholders. Third-party materials retain their existing license terms. The H2 provenance/ownership and clean-room review remains open; no Ravum source may be copied into Flow without separate rights review.