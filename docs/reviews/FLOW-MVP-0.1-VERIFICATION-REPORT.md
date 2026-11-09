# FLOW MVP 0.1 — verification report

**Date:** 2026-11-09 (Europe/Paris) · **Branch:** `feat/flow-mvp-0.1` · **PR:** #53 (draft)
**Base:** `fix/flow-foundation-proof-gates` @ `d884e29c825cad40678c56702361f4e403a769f7`
**Head under test:** filled at the end of this document (see §12)
**Verdict:** recorded in §11. Nothing here is a merge authorization, a release, a ratified ADR or a security proof.

This report states what was **executed**. A `PASSED` below always carries the
command that produced it. Everything not executed is `NOT_RUN`, and everything
the tool deliberately does not provide is `UNSUPPORTED` and is refused.

---

## 1. Mandate and what it authorized

The owner message of 2026-11-09 is a **bounded technical mandate**: work on Flow
branches, synthetic fixtures only, tests, CI and draft PRs. It explicitly does
**not** authorize: ratifying an ADR, choosing a sandbox policy, reusing Ravum
code, merging into `dev`/`main`, changing GitHub branch protection, publishing a
release, or using real secrets. `H4` (the confirmatory S9 decision) stays human.

`Rwanbt/Ravum` was never opened for write, never built, never branched during
this wave. No command in this wave ran inside `D:\App\Ravum`.

## 2. Baseline actually revalidated at the start

| Fact | Value observed | How |
|---|---|---|
| `main` | `9d5170a7de0a1bac5e1a8caf74827afc43122251` | `git fetch --prune`, `git branch -avv` |
| `dev` | `2c8cb62b4ff6e2c1403ae38fc47a3d86dcdf3b8b` | idem (local `dev` is 1 behind, untouched) |
| PR #51 | head `832f4995618a28c3e2bae25654e59651210a0faa`, draft, base `dev` | `gh pr view 51` |
| PR #52 | head `d884e29c825cad40678c56702361f4e403a769f7`, draft, base `feat/flow-verified-foundation` | `gh pr view 52` |
| CI on #52 head | `success` at `d884e29` (runs `42682876078`, `42682869025`) | `gh run list --branch fix/flow-foundation-proof-gates` |
| Working tree | clean, on `fix/flow-foundation-proof-gates` | `git status` |
| Toolchains | git 2.51.1.windows.1, rustc/cargo 1.95.0, Python 3.13.7, node 26.13.0 | `--version` |

The state had **not** advanced between the owner's audit and this wave, so the
branch was created from the verified head and no history was rewritten.
`git reset --hard`, `git clean -fd` and `git checkout -f` were never used.

## 3. What was built on this branch

| Commit | Scope |
|---|---|
| `fd1611b` | `flow-lock/2`: fail-closed lock attribution, moved to `lock.rs` |
| `abfbdf6` | `run_bounded`: every Git invocation bounded in memory and wall clock |
| `11c36a1` | proposals, candidates, `run/publish --candidate`, `--dry-run`, adoption gate |
| `82557dd`,`8ede7c9` | concurrency and process-bound evidence (teammate `safety`) |
| `c844c16`… | polyglot fixtures + demo (`examples/polyglot/**`, teammate `polyglot`) |
| `4553edb` | symbolic-ref refusal, post-CAS observation |
| (docs) | this report, README, ARCHITECTURE, DECISION_REGISTER, TRACEABILITY |

### 3.1 Proposal contract

`flow propose --agent A --task T --base <commit> --patch <file> [--key K]`

- Immutable: the patch bytes and their digest are recorded once. Same key + same
  bytes is idempotent (`idempotent: true`, same `proposal_id`); same key +
  different bytes is `IDEMPOTENCY_CONFLICT`. A correction is a new proposal.
- The patch is applied **only** to a temporary `GIT_INDEX_FILE` outside the
  repository (`git read-tree` + `git apply --cached --binary` + `git write-tree`).
  The user's index and worktree are never read for the result, never written.
- Admitted content is validated: no absolute/traversal path, no write into
  `.git/` or `.flow/`, no gitlink, no symlink escaping the tree, no
  case-colliding pair, nothing the host cannot materialise faithfully.
- Storage: `.flow/proposals/<id>.json` + `<id>.patch`, atomic, append-only by
  construct (an id is never rewritten).

### 3.2 Candidate contract

`flow assemble --proposal <id> [--proposal <id>]...`

- All proposals must share one base (`BASE_MISMATCH` otherwise).
- Order is explicit and part of the candidate identity.
- Incompatible operations on one path (add/delete/rename/mode/symlink collision)
  are refused **before** any patch is applied (`PATH_OPERATION_CONFLICT`), so the
  diagnostic does not depend on application order.
- A hunk that does not apply is `CONFLICT` and names the failing proposal.
- The resulting tree is validated again and stored as an immutable candidate
  (`.flow/candidates/<id>.json`), keyed by tree + base + ordered ids.
- No ref, index or worktree is touched.

### 3.3 Publication of a candidate

`flow run --candidate <id>` verifies **that** tree: the verification commit's
parent is the candidate's base, the receipt names the tree, the candidate id and
the effective policy digest.

`flow publish --candidate <id> --expected-old <oid> [--dry-run]` reuses the
existing gate (receipt schema/integrity/tree/scope/policy/context/obligations)
and adds: the candidate tree must still exist, the candidate base must **equal**
the CAS old OID (`BASE_MISMATCH` otherwise), the destination must be a real
branch ref (`git check-ref-format`, no symref), and the ref is read back after
the CAS (anything else is `NEEDS_RECONCILIATION`).

`--dry-run` performs the whole preflight and returns `PLAN` with **no** commit
object, **no** ref move and **no** journal line.

### 3.4 Two defects closed without being asked

- **Lock**: the previous contract stole a lock older than 120 s even with a live
  owner. `flow-lock/2` never displaces a live owner (whatever the age), steals
  only from a provably dead owner whose nonce is unchanged, and refuses an
  unattributable lock with `LOCK_UNKNOWN` + `manual_required`.
- **Bounded Git**: `Command::output()` allocated without bound on `git status`
  class commands. All Git calls now go through `run_bounded` (64 MiB per stream,
  300 s, descendants killed); past the cap the call **fails** (`GIT_OUTPUT_TOO_LARGE`)
  rather than parsing a clipped answer.

### 3.5 Policy adoption

A `.flow/config.json` can arrive with a clone. It is a claim, not a consent:

- `flow init` on a working copy that already contains one refuses with
  `POLICY_ADOPTION_REQUIRED` and prints the exact commands it would run;
- `flow init --adopt-policy` or `flow approve` records the approved policy digest
  in `.flow/adopted.json`;
- `run`/`publish` refuse with `POLICY_NOT_ADOPTED` / `POLICY_NOT_APPROVED` when
  the digest does not match.

## 4. Tests actually executed

Windows 11 (this host), `cargo test --locked --all-targets`, after the final commit:

| Suite | Tests | Result | Wall |
|---|---|---|---|
| unit (`src`) | 24 | PASS | 0.13 s |
| `tests/bounds.rs` | 21 | PASS | 3.19 s |
| `tests/cli.rs` | 38 | PASS | 8.42 s |
| `tests/concurrency.rs` | 8 | PASS | 5.06 s |
| `tests/isolation.rs` | 25 | PASS | 3.00 s |
| `tests/proposals.rs` | 13 | PASS | 8.23 s |
| **total** | **129** | **0 failed, 0 ignored** | ≈ 28 s |

Linux and Windows CI: §10. The CI job fails the build if any test is `ignored`,
if fewer than 129 tests actually ran, or if any of the 41 named gates is missing
from the result lines — a deleted or filtered test is a red build, not a green
one.

### 4.1 The mandated scenarios are tests, not prose

| Scenario | Test | What it asserts |
|---|---|---|
| M1 two agents, disjoint files | `m1_two_disjoint_agents_publish_exactly_the_verified_tree` | idempotent admission, candidate assembled, verified, published, `refs/heads/main^{tree}` == candidate tree == receipt tree, worktree byte-identical |
| M2 three agents, hunk collision | `m2_three_agents_collision_is_diagnosed_and_resolved_explicitly` | `CONFLICT`/`PATH_OPERATION_CONFLICT` with the colliding ids, ref and worktree unchanged, then explicit subset publication + rebased re-proposal |
| M3 stale base | `m3_stale_base_is_refused_and_a_rebased_proposal_succeeds` | external advance → `BASE_MISMATCH`, no forced update; explicit rebase → verified publication on the new base |
| M4 concurrent publication | `eight_concurrent_publishes_publish_at_most_once`, `two_concurrent_publishes_publish_at_most_once` | exactly one `PUBLISHED`, one intent/outcome, one added commit, 7 refusals |
| M5 crash and recovery | `crash_after_intent_is_not_published_and_reconciles`, `crash_after_ref_is_observed_as_published`, `p0c_one_outcome_cannot_close_two_intents` | an intent without an outcome is resolved by observation only, never by a second blind publish |
| M6 user files preserved | `m6_user_files_and_index_are_preserved_through_the_whole_cycle` | index listing and every file hash unchanged through propose → assemble → run → publish, with a dirty file, an untracked file and a staged change present |

## 5. Polyglot verification (real toolchains, offline)

`examples/polyglot/{rust,node,python}`, each a verification **target** with its
own `.flow/config.json`:

| Fixture | Obligations | Toolchain |
|---|---|---|
| `rust` | `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --locked --offline` | standalone dependency-free crate |
| `node` | `node --test` (built-in runner, no `npm install`) | Node ≥ 18 |
| `python` | `python -m unittest discover -v` (stdlib) | Python 3 |

Executed on this host, 2026-11-09:

```
pwsh -NoProfile -File examples/polyglot/run-demo.ps1 -Fixture all
RESULT: PASS — 3 fixture(s) verified, published, and refused after the documented red edit.
```

Per fixture: `flow init --adopt-policy` → `flow run` `PASSED` → `flow publish`
`PUBLISHED` → documented red edit → `flow run` `FAILED` → `flow publish`
`REFUSED` / `REQUIRED_NOT_PASSED`, ref unchanged. CI runs the same demo on
Ubuntu and Windows (§10).

There is **no** language adapter: Flow runs the commands declared in the policy.
No claim of selective test execution is made — the MVP runs whatever the policy
declares, which may be the whole suite.

## 6. Security boundaries (inventory, not marketing)

Guaranteed by construction and tested:

1. A publication is ref CAS + a durable intent/outcome journal; an interrupted
   publication is resolved by observation (`reconcile`), never blindly replayed.
2. Only a `PASSED` receipt whose tree, policy digest, context and every currently
   required obligation match can be published. `PARTIAL`, `FAILED`, `TIMEOUT`,
   `UNKNOWN`, `UNSUPPORTED`, `STALE_*`, `NEEDS_RECONCILIATION` never publish.
3. A checkout that does not reproduce the tree byte-for-byte is refused
   (`UNSUPPORTED_CHECKOUT`), not certified.
4. Flow's own state never enters the verified tree.
5. The Git environment is scrubbed (`GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`,
   object/config redirection, askpass, external diff/editor); the only index
   override honoured is an explicit absolute path passed by Flow itself.
6. User worktree and index are never written by `propose`/`assemble`/`run`/
   `publish`; this is asserted byte-for-byte in M6.
7. Ambiguous JSON (duplicate keys at any depth) is refused, never guessed.

Explicitly **not** provided (`doctor` reports them, and the tool refuses rather
than pretending):

| Non-guarantee | Consequence |
|---|---|
| No OS sandbox | obligations run with the user's rights; untrusted code execution is `UNSUPPORTED` |
| No Flow-dispatched hooks | a repository whose verification lives in hooks is not verified by Flow; the Git plumbing may still run `post-checkout`/`reference-transaction` (measured: a blocking one aborts `run`/`publish`) |
| No completeness proof of the build context | an undeclared input (cache, env var, toolchain version) can still change a result |
| Receipt integrity is not a signature | a writer who can rewrite config + receipt can forge one |
| The lock does not fence a foreign `git` writer | an external `update-ref` is detected at CAS, not prevented |
| `publish` moves refs only | HEAD/index/worktree can diverge afterwards; this is by design and stated in the output |
| Some file types cannot be represented | case collisions, gitlinks, escaping symlinks, reserved names are refused, not repaired |
| PID reuse on the lock | wedges the lock until a human removes it (`LOCK_UNKNOWN`); a wedge is loud, a stolen live lock is silent |

## 7. Resources measured (exploratory, not a benchmark)

| Measurement | Value | Conditions |
|---|---|---|
| Full Windows suite, wall clock | ≈ 28 s (six binaries, run in parallel by cargo) | 16 GB host, Windows 11, warm build |
| `tests/bounds.rs` 10 MiB obligation stdout | receipt `stdout_truncated: true`, retained log ≤ 1 MiB + marker | teammate `safety`, measured |
| `run_bounded` 65 MiB stdout | `GIT_OUTPUT_TOO_LARGE`, bounded return | teammate `safety`, measured |
| Peak memory | not instrumented in this wave | `NOT_RUN` |

No CPU/RAM profiling campaign was run: it is not authorized and `H4` is open.
**Performance is `PERFORMANCE_UNPROVEN`.** The S9 pilot (`n = 5`, exploratory) is
unchanged and proves nothing about productivity; no confirmatory campaign was
started.

## 8. Adversarial review

An independent reviewer (`qa` teammate, no write access to `crates/flow/src`)
attacked the CLI on disposable fixtures. Result recorded in
`docs/reviews/MVP-QA-ADVERSARIAL-REPORT.md` (see §9 for its status). Its purpose
is to contradict this report, not to decorate it.

Earlier independent review of the foundation (`#52`) already found and closed two
false-`PASSED` defects (unverified materialisation, one outcome closing two
intents) — both still have their failing-on-baseline tests in `tests/cli.rs`.

## 9. What is incomplete, and why

| Item | Status | Reason |
|---|---|---|
| Confirmatory S9 campaign | `NOT_RUN` | `H4` open; not authorized |
| OS sandbox / untrusted execution | `UNSUPPORTED` | deliberately out of scope; refused by claim |
| Hook execution as verification | `UNSUPPORTED` | hooks are not trustworthy input; documented and measured |
| Selective test execution by impact | `UNSUPPORTED` | the MVP runs the declared policy; no selective claim is made |
| Windows/macOS file-mode fidelity | `UNSUPPORTED` on Windows | `100755` cannot be represented; the published blob is verified by content, the bit is not |
| R0 corpus rerun on Linux | `NOT_RUN` | requires the exploratory harness on Linux; not part of this mandate |
| Line-level impact analysis, VFS/CRDT, daemon, forge | out of MVP | §1.3 of the mandate |

## 10. CI

| Run | OS | Result |
|---|---|---|
| (filled from `gh pr checks 53`) | Ubuntu | see below |
| (filled from `gh pr checks 53`) | Windows | see below |

The CI definition on this branch triggers on `feat/flow-mvp-0.1` and on every
stacked base, runs `cargo fmt --check`, `cargo clippy -D warnings`, the gated test
count, a release build, the R0-a harness self-tests (Linux), and the polyglot
demo on both OSes.

## 11. Verdict

**`MVP_PARTIAL`** at the time this section was first written; the final verdict
for the head under test is stated in §13, with the exact reasons. The distinction
matters: `MVP_REVIEW_READY` requires every acceptance box to be factually true,
and this branch is not `PRODUCTION_READY` in any case.

## 12. Human merge checklist (nothing here is automatic)

1. Review PR #53 (this branch) → PR #52 → PR #51 in that order; each is based on
   the previous one. `dev` and `main` are untouched.
2. Read `docs/reviews/MVP-QA-ADVERSARIAL-REPORT.md` and confirm no P0 is open.
3. Confirm the CI runs named in §10 are green **on the PR head**, not on an
   earlier commit.
4. Decide `H4` separately; it is not required for the MVP to be usable, and this
   branch makes no performance claim.
5. Merge with a merge commit (the series is stacked); do not squash across PRs.
6. Only after all of the above: decide whether `H1`/`H3`/`H6`/`H7`/`H8` are
   needed before any release. No release was published.

## 13. Final state of the head under test

Filled after the last verified command of this wave (§13.1).

### 13.1 Final verdict

`MVP_REVIEW_READY` requires: all gates L0–L7 green, every scenario a test,
CI green on the head, docs and vault consistent. Where any of these failed, the
honest verdict is `MVP_PARTIAL` and the failing box is named below rather than
argued away.
