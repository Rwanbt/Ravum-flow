# Ravum Flow
> **Status: evidence package `0.2.0-rc.1` on `feat/flow-production-candidate` (draft PR), gate verdict `PRODUCTION_PARTIAL` — local-trusted, not merged, not released.**  
> This branch ships a small local CLI (`flow`) that verifies an exact Git tree and publishes it behind a compare-and-swap receipt. It is **not** a VCS, a build scheduler or a sandbox, and no ADR/RFC is ratified.  
> No known P0/P1 finding is open in the local-trusted scope; the gate report names the evidence gaps that keep the verdict below `PRODUCTION_CANDIDATE` (coverage/mutation below their thresholds, whole-crate mutation and libFuzzer not run, reproducibility not proven, macOS not tested).  
> Planning baseline: **V6 consolidated after Claude review (9 November 2026)**. All Flow ADRs/RFCs remain **PROPOSED**, never automatically accepted.

**Ravum Flow** is an independent, local-first, Git-compatible and language-agnostic proposed workflow for humans and AI coding agents. Its purpose is to reduce **time to a verified deliverable**, preserve user edits, attribute concurrent proposals and avoid *mandatory persistent* Git worktrees **where safe and useful**.

It is **not** the [Ravum programming language](https://github.com/Rwanbt/Ravum), a compiler, a VCS replacement, a forge, a cloud prerequisite or an unconditional proof system. Ravum can later consume Flow via an optional adapter; Flow must work without Ravum.

## Minimal CLI — implemented and tested

```bash
cargo build --release                     # dependency-free, offline

flow init                                 # create .flow/ ; adopts the policy it creates
flow doctor                               # read-only: git, hooks, toolchains, limits, lock, adoption
git add -A
flow run                                  # verify the INDEX tree in a disposable copy
flow publish --expected-old <commit>      # ref CAS + durable intent/outcome journal
```

Two or three agents, without any provider, daemon or network:

```bash
BASE=$(git rev-parse HEAD)

# each agent edits its own copy and exports a Git patch (any tool, any model)
git diff --binary "$BASE" > /tmp/agent-a.patch      # generated against BASE

flow propose --agent A --task T1 --base "$BASE" --patch /tmp/agent-a.patch --key a1
flow propose --agent B --task T2 --base "$BASE" --patch /tmp/agent-b.patch --key b1
flow proposals list

flow assemble --proposal <idA> --proposal <idB>      # immutable candidate tree, nothing moved
flow run --candidate <candidate-id>                 # verify THAT tree
flow publish --candidate <candidate-id> --expected-old "$BASE" --dry-run
flow publish --candidate <candidate-id> --expected-old "$BASE"
```

If two agents touch the same hunk, `assemble` refuses with `CONFLICT` (or
`PATH_OPERATION_CONFLICT` for rename/delete/mode collisions) and names the
failing proposal. Resolution is explicit: publish the disjoint subset, then
re-propose against the new base. A stale base is `BASE_MISMATCH`, never a forced
update. `--dry-run` performs the whole preflight and prints a `PLAN` without
creating a commit, moving a ref or writing the journal. A boolean flag takes no
value: `--dry-run=true` is a usage error (exit 2) rather than a live publication.

| Command | State | Guarantee actually delivered |
|---|---|---|
| `init` | implemented | creates config/journal/receipts; adopts the policy it writes (`--adopt-policy` is required to adopt a `.flow/config.json` that arrived with a clone); never truncates a journal, a receipt or a proposal |
| `approve` | implemented | records local approval of the current policy digest; the approval is a gate, not a formality |
| `propose` | implemented | immutable, attributed proposal (agent, task, idempotency key, pinned base); applied only to a temporary index; same key + same bytes = same id, same key + different bytes = `IDEMPOTENCY_CONFLICT` |
| `proposals list/show` | implemented | deterministic listing of admitted proposals |
| `assemble` | implemented | immutable candidate tree from proposals sharing one base, in explicit order; ambiguity is refused, never merged silently |
| `run --candidate` | implemented | verifies the candidate tree with the same receipt engine; the receipt names the tree, the candidate and the effective policy |
| `publish --candidate` | implemented | requires a complete `PASSED` receipt for that tree, the candidate base equal to the CAS old OID, a real branch ref (no symref), and re-reads the ref after the CAS |
| `publish --dry-run` | implemented | full preflight, `PLAN` output, zero mutation |
| `init` | implemented | creates config/journal/receipts; refuses to overwrite without `--force` |
| `doctor` | implemented | read-only report; lists what is **not** guaranteed, plus the lock attribution and the policy adoption state |
| `status` | implemented | head/tree, dirty and untracked counts, latest receipt, pending intents; a corrupt journal is an error, never "0 pending" |
| `run` | implemented | exports the exact tree to a directory Flow owns (`.git` link removed), runs declared obligations, writes a receipt bound to the tree OID **and** to the effective policy digest. `--obligation ID` is diagnostic only: the run is `PARTIAL` and is never publishable |
| `publish` | implemented (ref-only) | requires a **complete** `PASSED` receipt for that exact tree whose policy digest still matches the current config; verifies the receipt's integrity and every required obligation, appends a durable intent, re-reads the ref, then `git update-ref` CAS, then reads the ref back. Destination must be a real `refs/heads/...` ref (a symref is `REF_IS_SYMREF`); a tree the ref already carries is `ALREADY_PUBLISHED`. Outcomes `PUBLISHED` / `REFUSED` with a machine `code`; a crash window is classified by `reconcile` as `RECONCILED_PUBLISHED`, `RECONCILED_NOT_PUBLISHED` or `NEEDS_RECONCILIATION` |
| `reconcile` | implemented | observes the ref and records `PUBLISHED` / `NOT_PUBLISHED` / `NEEDS_RECONCILIATION`; never moves a ref. An unreadable journal is `CORRUPT_JOURNAL` or `UNKNOWN_OUTCOME` with `manual_required: true` and a non-zero exit — never a successful empty reconciliation |
| `explain` | implemented | receipt + journal entries for a tree |
| `disable` / `enable` | implemented | mutating commands refuse while disabled |

**Guarantees:** receipts bind an exact Git tree OID *and* one effective policy digest (policy version, execution boundary, every obligation's id/program/args/required/timeout, and the obligation environment allowlist); a receipt altered on disk is refused; a required obligation that cannot run yields `UNSUPPORTED`, never `PASSED`; a receipt produced under a weaker or different policy is `STALE_POLICY`; publication is ref-CAS with an intent whose file is fsynced before the rename (and whose directory is fsynced on Unix; Windows has no directory fsync) and an outcome recorded after; dirty/untracked user files are never modified. A usage error exits 2 and never mutates.

**Non-guarantees (by design in this MVP):** no OS sandbox (obligations run with your rights and a scrubbed environment), no hook dispatch of its own — the Git plumbing it calls can still run `post-checkout` on `worktree add` and `reference-transaction` on `update-ref` — no completeness proof of the build context, no working-tree checkout after publishing, single local writer only. The receipt `integrity` digest detects accidental corruption and casual edits; it is not a signature, so anyone who can write `.flow/receipts` can forge a receipt. A tree this host cannot reproduce faithfully (case collisions, symlinks without `core.symlinks`, gitlinks) is refused rather than certified. The local advisory lock serialises Flow processes but does not fence an external `git` writer. Untrusted code execution is `UNSUPPORTED`.

**Polyglot verification** — no language adapter is claimed; Flow runs the commands the policy declares:

| Fixture | Obligations | Offline? |
|---|---|---|
| `examples/polyglot/rust` | `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --locked --offline` | yes (no dependencies) |
| `examples/polyglot/node` | `node --test` (built-in runner) | yes (no `npm install`) |
| `examples/polyglot/python` | `python -m unittest discover -v` | yes (stdlib) |
| `examples/polyglot/typescript` | `tsc --noEmit`, `node --test test/math.test.ts` | no — `tsc` is a one-time, version-pinned prerequisite installed by the demo/CI |

```bash
pwsh -File examples/polyglot/run-demo.ps1 -Fixture all   # Windows
bash examples/polyglot/run-demo.sh all                   # Linux
```

Each demo copies the fixture to a throw-away directory, runs `init --adopt-policy`,
verifies, publishes, applies the fixture's documented red edit, and shows that
`run` fails and `publish` is refused. Nothing is written into this repository.

**Tested platforms:** **163 tests executed on Windows 11, 162 on Linux, 0 ignored** (the difference is one Windows-only PATH/PATHEXT test), Rust **1.95.0** pinned by `rust-toolchain.toml` and by CI; Ubuntu and Windows in CI, where every GitHub Action is pinned to a commit SHA. A claim that is not in the report is not a claim.

See the [production gate report](docs/reviews/FLOW-PRODUCTION-GATE-REPORT.md), the [MVP 0.1 verification report](docs/reviews/FLOW-MVP-0.1-VERIFICATION-REPORT.md),
the [user guide](docs/MVP-USER-GUIDE.md), the [CLI/JSON contract](docs/CLI-JSON.md)
and the [foundation report](docs/reviews/FLOW-FOUNDATION-VERIFIED-REPORT.md).

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

## What Flow does not do

No OS sandbox (obligations run with your rights; untrusted code execution is
`UNSUPPORTED`). No hook execution of Flow's own (the Git plumbing may still run
`post-checkout`/`reference-transaction`; a blocking one aborts the command). No
completeness proof of the build context. The receipt digest is integrity, not a
signature. The lock serialises Flow processes but does not fence an external
`git` writer. `publish` moves refs only: HEAD, index and worktree can diverge
afterwards. File types this host cannot represent faithfully (case collisions,
gitlinks, escaping symlinks, reserved names) are refused, not repaired.

## Repository workflow

`main` is the documentary baseline; `dev` hosts proposed planning changes and later narrowly authorized experiments. Prefer feature branch → PR into `dev`, and reviewed promotion into `main`. No code execution or implementation is authorized merely by this README. Branch protections must be configured separately by an authorized admin.

## Human decisions still pending

H1 select a provisional option (OPEN); H2 licence — **decided by the owner on 2026-11-09: MPL-2.0** (provenance/clean-room review still required before reusing external source); H3 Windows/Linux sandbox guarantee (OPEN); H4 S9 confirmatory thresholds and budget (OPEN — the exploratory pilot ran without them); H5 disposable R0 fixtures (**granted 2026-11-09** by the owner for this branch); H6 P0 handoff to the separate Ravum session (OPEN); H7 internal-first use (OPEN); H8 stop rule for documentary reviews (OPEN). See the decision register.

**License:** [Mozilla Public License 2.0](LICENSE) applies to original Ravum Flow material licensed by its rightsholders. Third-party materials retain their existing license terms. The H2 provenance/ownership and clean-room review remains open; no Ravum source may be copied into Flow without separate rights review.