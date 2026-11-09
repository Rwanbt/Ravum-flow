# Flow production candidate — gate report (G10)

**Date:** 2026-11-10 (Europe/Paris)
**Base:** `ea181d4f112df4deef01016a041fcdfdac154264` (MVP 0.1, PR #53)
**Branch:** `feat/flow-production-candidate` · **PR:** #54 (draft, base `feat/flow-mvp-0.1`)
**Code head under test:** see §1 (the code head is the last commit that touches `crates/flow`; later
commits are documentation only)
**Verdict:** **`PRODUCTION_PARTIAL`** — reasons in §9. No merge, no release, no GO_PRODUCTION.

---

## 1. What was executed, on which SHA

| Step | Result | Evidence |
|---|---|---|
| G0 baseline reconciliation | PASS | `FLOW-PRODUCTION-BASELINE.md`; refs, PRs, CI run ids, worktree, dry tree |
| G1 invariant matrix | PASS (as documentation) | `FLOW-PRODUCTION-INVARIANTS.md`, 24 invariants each mapped to an executed test |
| G2 known findings | PASS — all closed with a regression test | §3 |
| G3 crash points | PASS — `before-intent` added; three injection points tested | `crash_before_intent_leaves_no_pending_intent`, existing `after-intent`/`after-ref` |
| G4 scenarios | PASS — M1–M10 exist as tests | §4 |
| G5 polyglot | PASS — Rust, Node, Python **and TypeScript**, green and red, on Windows; CI on Ubuntu | §5 |
| G6 measurement | **PARTIAL** — coverage/mutation measured and **below the candidate gates**; fuzz found one real defect (fixed) | §6 |
| G7 CI / release candidate | PASS with two open items (reproducibility not proven, Actions not SHA-pinned) | §7 |
| G8 performance | MEASURED (descriptive) / productivity `UNPROVEN` | §8 |
| G9 docs / vault | docs PASS; **vault `VAULT_SYNC_BLOCKED`** (pre-existing red check, other projects) | §9 |
| G10 independent review | adversarial replay: see §10; maintainability review: not run by a second agent | §10 |

**Tests executed:** 148, 0 failed, 0 ignored — Windows 11 local; Ubuntu + Windows in CI.
Per binary: 25 unit · 22 bounds · 49 CLI · 9 concurrency · 25 isolation · 18 proposals. (The `json`
unit tests are also collected by the `cli` and `bounds` binaries through `#[path]`, so the sum of
"unique" tests is smaller; the gate counts what actually ran.)

## 2. CI on the head

`.github/workflows/flow-ci.yml`, both push and pull_request runs on the candidate branch:
Ubuntu and Windows, `cargo fmt --check`, `cargo clippy -D warnings`, the gated test step (≥148
executed, 0 ignored, 57 named gates), `cargo build --release`, the release-binary crash-hook proof,
R0-a self-tests (Linux) and the polyglot job (four fixtures, TypeScript compiler installed).
Run ids are recorded in §11 after the last push.

## 3. G2 — the known findings and what closed them

| ID | Was | Now | Test |
|---|---|---|---|
| PRF-06 | `explain --tree ../../…` read arbitrary JSON; `proposals show` / `run --candidate` built paths from user input | every identifier is validated as a hex object/record id before a path exists (`INVALID_TREE_ID`, `INVALID_RECORD_ID`) | `explain_refuses_a_path_instead_of_a_tree_id` |
| PRF-04 | `status`/`explain` displayed a tampered receipt as `PASSED` | `receipt_trust`, `receipt_status_display=RECEIPT_INTEGRITY_INVALID`, `trust` in `latest_receipt` | `status_and_explain_qualify_a_tampered_receipt` |
| PRF-02 | deleted logs went unnoticed | `evidence_complete` + `missing_logs` (publication unchanged: the receipt is intact) | `explain_reports_missing_logs_as_incomplete_evidence` |
| PRF-05 | `doctor` exited 0 with `overall: FAILED` | exit 1, documented as `exit_contract`, tested | `doctor_exits_nonzero_when_a_guarantee_is_broken` |
| FIL-15 | a cloned policy ran with no provenance | `status.policy` shows path, digest, obligation count, adoption source and date; `approve` lists the exact commands | `status_reports_policy_provenance`, `m9_…` |
| PRC-10 | `FLOW_TEST_CRASH` honoured by release builds | compiled out of release (`cfg!(debug_assertions)`); proved on the release binary locally **and** in CI | CI step "the release binary ignores the test crash hook" |
| FIL-08 residual | a filter could rewrite bytes without changing size | exact bytes compared with the tree blobs through one bounded `git cat-file --batch` | `a_converted_checkout_is_refused_not_certified` |
| FIL-11 | a junction/reparse point inside `.flow` could redirect a write | every write below `.flow/` refuses symlinks, Windows reparse points and (Unix) hardlinks | `a_symlinked_journal_is_never_written_through`, `m9_…`; Unix hardlink variant |
| GIT-09/10 | a symref target could be moved while the alias was reported | `check-ref-format` + `REF_IS_SYMREF` + post-CAS read-back | `p1_a_symbolic_ref_is_never_a_publication_target` |
| F1 (new, fuzz) | `1e999` parsed to infinity and printed as bare `inf`, breaking the JSON contract; `timeout_s: 1e999` saturated silently | non-finite numbers refused at parse time | `a_non_finite_number_is_refused_not_printed_as_inf`, `a_non_finite_number_in_local_state_is_refused_not_printed` |

**Still open, and honestly labelled:**

| ID | Severity | Why it is not closed here |
|---|---|---|
| PRF-01 | P2 | receipt integrity is not a signature. Fixing it needs a key/authentication model, i.e. a product decision (`H3`-adjacent). Documented as a non-guarantee. |
| PRF-02 (retention) | P3 | there is no retention/rotation policy; logs are evidence only and their absence is now displayed |
| GIT-09 | P2 | a symbolic ref is now **refused**, so the reporting ambiguity can no longer happen; retained as a code path note |
| lock nonce uniqueness | P2 (mutation survivor) | the nonce is unique by construction (`pid + nanos + counter`) but **no test asserts it**; a mutation survives. Listed as a test gap, not a product defect. |
| `paths::canonical_within` containment | P2 (mutation survivor) | the mutation `Ok(Default::default())` survives: containment resolution is not asserted directly. Test gap. |
| macOS | — | `NOT_TESTED_MACOS` |

## 4. G4 — multi-agent scenarios

M1 (two disjoint agents), M2 (three agents, collision then explicit resolution), M3 (stale base),
M4 (2 and 8 concurrent publishers), M5 (three crash windows), M6 (byte-for-byte user files), M7
(agent handover), M8 (cross-base refusal), M9 (hostile cloned repository), M10 (foreign schema) are
**tests** in `crates/flow/tests/proposals.rs` and `tests/cli.rs`. `M7`-`M10` were added in this wave.
No AI provider, no network: proposals are Git patches produced by a throw-away worktree.

## 5. G5 — polyglot

| Fixture | Obligations | Windows local | CI Ubuntu | CI Windows |
|---|---|---|---|---|
| Rust | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --locked --offline` | PASS → PUBLISHED → red FAILED → REFUSED | PASS | PASS |
| Node | `node --test` | idem | PASS | PASS |
| Python | `python -m unittest discover -v` | idem | PASS | PASS |
| TypeScript | `tsc --noEmit`, `node --test test/math.test.ts` | idem | PASS (tsc installed in CI) | PASS |

`tsc` is a documented one-time prerequisite; without it the required obligation fails closed (the
demo exits non-zero with an explicit message, it does not skip). WSL2 with Node 20 is `UNSUPPORTED`
for `.ts` and says so.

## 6. G6 — measured test evidence (and where it falls short)

**Coverage** (cargo-llvm-cov on nightly, branch counters, 132 tests at `ea181d4`; artefact
`experiments/quality/coverage/`):

| Scope | Lines | Branches | Candidate gate | Met |
|---|---|---|---|---|
| global | 80.76 % (4564 lines) | 65.26 % (780) | ≥90 % / ≥85 % | **no** |
| `commands.rs` | 78.56 % | 65.14 % | ≥95 % / ≥90 % | **no** |
| `state.rs` | 92.90 % | 68.00 % | ≥95 % / ≥90 % | **no** |
| `proposal.rs` | 73.17 % | 52.05 % | ≥95 % / ≥90 % | **no** |
| `lock.rs` | 72.52 % | 61.11 % | ≥95 % / ≥90 % | **no** |
| `gitcli.rs` | 91.73 % | 79.03 % | ≥95 % / ≥90 % | **no** |
| `paths.rs` | 80.92 % | 78.95 % | ≥95 % / ≥90 % | **no** |
| `json.rs` | 86.64 % | 76.09 % | ≥95 % / ≥90 % | **no** |
| `scratch.rs` | 71.04 % | 55.56 % | ≥95 % / ≥90 % | **no** |
| `sha256.rs` | 100 % | 100 % | — | yes |

**Mutation** (`cargo-mutants`, bounded to `paths.rs` + `lock.rs`, 13 min): **50.8 % killed**
(30 caught / 28 missed / 1 timeout / 6 unviable of 65 tested). Whole-crate mutation (960 mutants) is
`NOT_RUN` — ≈24 h serial on this host. 28 survivors are listed with `file:line` in
`experiments/quality/mutation/missed.txt`; the two that matter are `paths::canonical_within`
containment (survives `Ok(Default::default())`) and the lock nonce (survives `"xyzzy"`).

**Properties** (seed 20261110, 30.7 s, 7/7 PASS): JSON round-trip, duplicate-key refusal, 20 000
dangerous paths refused, 100 000 proposal ids with 0 collisions, CLI-driven assembly-order property.

**Fuzz** (hand-rolled, seed 20261110): JSON parser 8 704 339 iterations in 60 s → 909 findings in 2
classes, one real defect (F1, fixed in `bc325a3`); CLI option parser 273 invocations, 0 findings.
`cargo-fuzz`/libFuzzer is `NOT_RUN` (needs a nightly target; deterministic seed replay was the
requirement).

**Flakiness:** concurrency suite 3× → 8/8, 8/8, 8/8 (6.94 s / 7.97 s / 7.02 s). Three runs cannot
prove absence; recorded as an observation.

**Conclusion for G6:** the numbers are real, and they are **below the candidate gates**. No label
pretends otherwise.

## 7. G7 — CI, supply chain, release candidate

| Item | State |
|---|---|
| CI on Windows + Ubuntu | green on the candidate head (§11) |
| Release binary | `flow 0.2.0-rc.1`, 1 013 248 B, SHA-256 `d6edbec0…d19bc` (artefact `dist/`, git-ignored) |
| Install / uninstall smoke | executed: private directory, `flow version`, `init`, `run`, `publish`, delete → nothing left |
| Release crash hook | inert, proved locally and in CI |
| `cargo audit` | 0 vulnerabilities (1430 advisories, 1 crate) |
| Runtime dependencies | none |
| Reproducible build | **`REPRODUCIBILITY_NOT_PROVEN`** — two clean archive builds gave `978fefb3…` and `30e958d7…` |
| Actions pinned to SHAs | **no** — tags only; open item before a public release |
| Branch protection | **absent** on `dev`/`main` (verified at G0); not changed by this branch; admin checklist in `FLOW-PRODUCTION-RELEASE-CHECKLIST.md` |

## 8. G8 — performance (descriptive, not a benchmark)

Windows 11, 16 logical CPUs, 15.7 GB RAM, warm build, single run per command (`flow 0.2.0-rc.1`
release binary):

| Command | Wall clock | Peak working set |
|---|---|---|
| `flow version` | 140–227 ms | ≈4.0 MB |
| `flow run` (1 obligation, small repo) | 1.2–2.4 s | ≈4.7 MB |
| `flow publish` | 0.8–2.1 s | ≈4.6 MB |
| candidate cycle (`propose`/`assemble`) | 97–705 ms per step, `PARTIAL` — the full cycle was not re-timed after the last fixes | ≈4.2–4.4 MB |

Assertions: Git output is bounded at 64 MiB/stream and obligations at 1 MiB/stream (tested with a
10 MiB obligation and a 65 MiB Git command); `.flow/` grows with receipts/proposals/candidates and
has **no retention policy** (documented, not silently compacted). **Productivity is `UNPROVEN`**:
`H4` is open, no confirmatory campaign was run, and the exploratory S9 pilot (`n=5`) proves nothing.

## 9. G9 — documentation and vault

Delivered: `FLOW-PRODUCTION-BASELINE.md`, `FLOW-PRODUCTION-INVARIANTS.md`,
`FLOW-PRODUCTION-COMPATIBILITY.md`, `FLOW-PRODUCTION-TEST-MATRIX.md`,
`FLOW-PRODUCTION-RELEASE-CHECKLIST.md`, `FLOW-PRODUCTION-ADVERSARIAL.md` (see §10), `CHANGELOG.md`,
`docs/QUICKSTART-10-MINUTES.md`, updated README, updated `docs/CLI-JSON.md` §10.

**Vault: `VAULT_SYNC_BLOCKED`.** `vault.py check` reports `lint OK · links OK · idempotence OK ·
root-cleanup OK` but `migrate FAIL (4)` on `projects/rimatrix` and `projects/seno-daw` — files this
session never touched. Contract v4 forbids committing a red vault without an explicit human
instruction, so the session note is written but **not committed**. The user can authorise an
exception explicitly; the four paths are listed in the check output.

## 10. G10 — independent review

- **Adversarial (agent G, no write access to `crates/flow/src`)**: see
  `FLOW-PRODUCTION-ADVERSARIAL.md` and the summary in §10.1.
- **Maintainability / supply chain (second independent reviewer)**: `NOT_RUN`. No second agent was
  available in this wave; the checklist in `FLOW-PRODUCTION-RELEASE-CHECKLIST.md` §7 is the input for
  that review. This is one of the reasons the verdict is not `PRODUCTION_CANDIDATE`.

### 10.1 Adversarial replay status

See §11 for the final numbers after the replay on the candidate head. The previous harness run
(`d884e29`, 93 executed) is preserved unchanged in `docs/reviews/MVP-QA-ADVERSARIAL-REPORT.md` as an
historical state; it was not retroactively edited.

## 11. Final numbers and the verdict

Filled by the final commit of this wave.

### 11.1 Verdict

`PRODUCTION_PARTIAL`. The candidate is functional, its known P0/P1 findings are closed with tests,
CI is green on both supported OSes, the release candidate builds and installs, and the trust boundary
is explicit. It is **not** labelled `PRODUCTION_CANDIDATE` because:

1. coverage and mutation are measured and **below the gates this mandate sets** (80.76 %/65.26 % vs
   90 %/85 % global; 50.8 % killed on a subset vs 90 %);
2. the whole-crate mutation score is `NOT_RUN`, and `cargo-fuzz` is `NOT_RUN`;
3. the second independent review (maintainability / supply chain) is `NOT_RUN`;
4. reproducibility is `NOT_PROVEN` and the Actions are not SHA-pinned;
5. `vault.py check` is red for pre-existing reasons outside this project, so the vault is
   `VAULT_SYNC_BLOCKED`;
6. macOS is `NOT_TESTED_MACOS`, and performance/productivity is `UNPROVEN`.

None of these is a P0 or P1 defect; all are evidence gaps. That is exactly the difference between
"the product works and is honest" and "the qualification dossier is complete".
