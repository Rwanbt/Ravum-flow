# Flow — verdict on the merged line (2026-11-13)

**Scope.** `dev` and `main` after the promotion of the candidate line (PR #59, #60), plus the
qualification branch merged on top (PR #61). **Verdict: `PRODUCTION_PARTIAL` → still `PARTIAL`, but
the three reasons that kept it there on 2026-11-10 are now closed or armed.**
**Every number is sourced in [MEASUREMENTS.md](../MEASUREMENTS.md).** This document decides nothing:
`GO_PRODUCTION`, the ADRs and the sandbox policy remain the owner's.

---

## 1. What changed since the gate report of 2026-11-10

The old report was bound to `06eb7c3` and listed four reasons for `PARTIAL`. Against the merged line:

| Old reason | State on 2026-11-13 | Evidence |
|---|---|---|
| 1. Coverage below the 95/90 gate (`83.19 %/68.64 %`) | **closed** — the gate is met in CI with no waiver | [MEASUREMENTS §2](../MEASUREMENTS.md) |
| 2. Mutation below the gate, whole-crate run `NOT_RUN`, no Linux run | **armed, first module measured** — the per-module campaign runs in CI on Linux (`paths`: **31/31 caught**, 2 min); no threshold enforced yet | [MEASUREMENTS §3](../MEASUREMENTS.md) |
| 3. `cargo-fuzz` not run | **open** — the zero-dependency seeded sweep is not libFuzzer | [CAPABILITIES](../CAPABILITIES.md) |
| 4. macOS not tested, productivity unproven | **half closed** — macOS runs the whole suite (479 executed); productivity still `UNPROVEN` | [MEASUREMENTS §1, §5](../MEASUREMENTS.md) |

Four more things became true, and none of them is a documentation change:

- **The branch line was merged.** `dev` and `main` carry the work (`main` = `7dcedc86`); the four
  stacked draft PRs that hid it were retired. Before this, "production ready" was a statement about
  an unmerged branch.
- **The real-project envelope was lifted.** `run` refused a tree above 20 000 regular files; the
  comparison now streams and covers **200 000 files**, and a 25 000-file repository was verified
  **and published** end to end ([pilot](../experiments/pilot-2026-11-13/results.json)).
- **The collaboration path was executed on a real project**: two attributed proposals on one
  immutable base → one verified candidate tree → published; a same-path collision was **refused by
  name** (`PATH_OPERATION_CONFLICT`), not merged.
- **macOS is measured** (479 tests, same count as Linux) and found a real limit — APFS refuses a
  non-UTF-8 file name, so that gate skips with its reason instead of panicking.

## 2. The owner's definition, answered

> *"prêt et efficace pour un usage en production sur de vrais projets et apporte de réels avantages
> par rapport aux autres systèmes déjà existants pour le workflow de travail en collaboration"*

### 2.1 Ready and effective on real projects

| Requirement | State | What is missing |
|---|---|---|
| Works on a real repository, not only fixtures | **yes** — 2 705 files verified and published; 25 000 files verified and published | a repository **above 200 000 files** is still refused (a documented limit, not a bug) |
| Cost of a verification cycle | **measured**: 28 s for 2 705 files with two real obligations; 172 s for 25 000 files with one no-op obligation | no performance claim beyond these points; no regression budget other than the 2 s/5 s tripwires |
| Installable, verifiable artefact | **yes, unsigned** — `v0.2.0-rc.2` is tagged and published with the CI artefacts of commit `e1022d86` (provenance, SBOM, checksums; reproducible per host) | **no code signature**: SmartScreen warns and there is no distribution trust chain |
| Safety net when things go wrong | **yes** — durable intent/outcome journal, `reconcile`, non-destructive rules, no `reset --hard`; the crash windows are now also **executed on the release binary** (Linux, three kill points) | retention is opt-in (`flow gc`), not a default; the Windows crash windows are still proved on the debug binary only |
| Supported platforms | Windows, Linux (tested + artefact), **macOS (tested, no artefact)** | macOS packaging; ARM Linux |
| Governance around the code | **yes** — the ruleset `protect main and dev` blocks deletion and force-push and requires six checks (Ubuntu, Windows, macOS, polyglot ×2, coverage) | `CODEOWNERS` and a review requirement are still absent |

### 2.2 Real advantage for collaborative work

What is **demonstrated** here, not promised:

1. **Attribution without a worktree per agent.** Two agents submit patches against one immutable
   base; each proposal is attributed, idempotent and stored outside the user's working tree. The
   user's dirty files are never touched.
2. **One exact candidate tree, verified before any ref moves.** The receipt names the Git tree OID,
   the effective policy digest and the execution boundary; publication is a compare-and-swap against
   the expected old commit. A green test suite on a *different* tree cannot publish.
3. **Collisions are refused, not merged.** Same path, same hunk, rename/delete/mode: the assembly
   returns a named refusal and the failing proposals. The pilot reproduced it on a real repository.
4. **A verifiable history.** Standard Git commits and refs: the result is readable with `git log`,
   reviewable in a forge, and reversible with `git update-ref`; there is no proprietary store.

What is **not** claimed, and is the honest limit of the "advantage" claim:

- **Productivity is `UNPROVEN`.** No confirmatory S9 campaign exists (`H4` is open). Flow is *not*
  shown to make a team faster than plain Git + ephemeral worktrees; the structural argument above is
  `PROPOSED`, and the historical S8 measurement was about *Ravum*, not Flow.
- **The comparison with jj / GitButler / plain Git + copies is documentary**, not measured on the
  same tasks and machines. Claiming an advantage there would be the exact defect this repository
  spent nine waves documenting.
- **No sandbox, no signed receipts, no per-candidate approval.** Anyone who can run `run` and
  `publish` can publish once the policy is approved. That is fine for a trusted local workflow; it
  is not a multi-tenant one.

## 3. What remains, by severity

| ID | S | Item | Owner |
|---|---|---|---|
| `PA-C6` | S1 | **done** — merged into `dev` and `main` | — |
| `PA-C7` | S1 | Branch protection, required checks, `CODEOWNERS` on `main`/`dev` | repository admin (human) |
| `PA-A2/A8` | S1 | `H7` answered by the owner; **`GO_PRODUCTION` not given** | human |
| `PA-A1/A3/A4` | S1 | `H1` architecture option, `H3` sandbox, `H4` S9 thresholds still open | human |
| `PA-B13` | S1 | Confirmatory S9 campaign (the productivity claim) | human decision + budget |
| `PA-C1/C3/C5` | S2 | Release head: tag, Release with the CI artefacts, **code signature** | human decision (signing) |
| `PA-B8` | S2 | Receipts are integrity, not signature; either a key model or a ratified limit | human decision |
| `PA-B9` | S2 | **done** — `flow gc --keep/--max-age-days`, opt-in; the journal, config, adoption record and proposals are never collected | — |
| `PA-B2/B3` | S2 | Mutation in CI: armed, first full matrix pending; no threshold yet | engineering |
| `PA-B6` | S2 | **done, Linux** — crash harness on the **release** binary (three kill points, recovery asserted); Windows still relies on the debug-only hook | — |
| `PA-B7` | S3 | libFuzzer target | engineering |
| `PA-A7` | S2 | `G10` independence: needs a reviewer outside the team | human |
| `PA-D*` | S3 | Documentation drift — **closed for the current numbers** by [MEASUREMENTS.md](../MEASUREMENTS.md) | — |
| `PA-E1/E2` | S1/S2 | Broken acceptance queue (board) and stale issue tracker | human + engineering |
| `PA-E3` | S2 | `experiments/` is 93.6 % of the tracked files | engineering |

## 4. Verdict

**`PRODUCTION_PARTIAL` on the merged line.** The product is functional on real projects, the
collaboration cycle is executed end to end, the quality gates are enforced and honest, and the two
things that blocked an *engineering* verdict (coverage, a usable envelope) are gone. It is not
`PRODUCTION_CANDIDATE` because the artefacts are **unsigned** and because the productivity advantage,
which is the reason Flow exists, is still a protocol rather than a measurement. The rest of the
delivery chain now exists: the line is merged, `main` and `dev` are protected by a ruleset requiring
the six CI checks, and `v0.2.0-rc.2` is published with the CI artefacts of the tagged commit.

**What would move it:** a tagged, signed release published from the CI artefacts of a protected
`main`, the mutation matrix run once with a threshold, and one confirmatory S9 campaign with the
thresholds the owner sets. Three of those are decisions, not code.

*This document does not grant `GO_PRODUCTION`; nothing here does.*
