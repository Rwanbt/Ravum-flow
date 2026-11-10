# Flow production candidate — gate report (G10)

**Date:** 2026-11-10 (Europe/Paris)
**Base:** `ea181d4f112df4deef01016a041fcdfdac154264` (MVP 0.1, PR #53)
**Branch:** `feat/flow-production-candidate` · **PR:** #54 (draft, base `feat/flow-mvp-0.1`)
**Code head under test:** `997f983` (the last commit that touches `crates/flow`, `.github/`,
`Cargo.toml` or `rust-toolchain.toml`); everything after it is documentation only
**Verdict:** **`PRODUCTION_PARTIAL`** — reasons in §11.1. No merge, no release, no GO_PRODUCTION.

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
| G6 measurement | **PARTIAL** — coverage/mutation measured on the delivered head and **below the candidate gates**; fuzz found one real defect (fixed) | §6 |
| G7 CI / release candidate | PASS on what CI can prove (install smoke on both OSes, Actions SHA-pinned, toolchain pinned); **reproducibility not proven** | §7 |
| G8 performance | MEASURED (descriptive) / productivity `UNPROVEN` | §8 |
| G9 docs / vault | docs PASS; vault **repaired and green** (the four pre-existing `migrate` items are fixed, `vault.py check` reports 0 issue) | §9 |
| G10 independent review | adversarial replay **and** a second independent agent review; the review found one P1, fixed with a regression test | §10 |

**Tests executed:** **163 on Windows 11, 162 on Linux**, 0 failed, 0 ignored (the difference is one
Windows-only PATH/PATHEXT test). Per binary: 35 unit · 22 bounds · 54 CLI · 9 concurrency ·
25 isolation · 18 proposals. (The unit tests are also collected by the integration binaries through
`#[path]`, so the sum of "unique" tests is smaller; the gate counts what actually ran.)

## 2. CI on the head

`.github/workflows/flow-ci.yml`, both push and pull_request runs on the candidate branch and on the
promotion targets `dev` and `main`: Ubuntu and Windows, `cargo fmt --check`,
`cargo clippy -D warnings`, the gated test step (≥162 executed on Linux, ≥163 on Windows, 0 ignored,
66 named gates of which 65 are required per platform), `cargo build --release`, the release-binary
crash-hook proof, the **install smoke on both OSes** (private directory, `version`, `init`, `run`,
`publish`, nothing left behind), R0-a self-tests (Linux) and the polyglot job (four fixtures, the
TypeScript compiler installed by a pinned `npm install`). Every `uses:` is pinned to a commit SHA and
the toolchain is pinned to `1.95.0` both by `rust-toolchain.toml` and by the action input.
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

**Coverage** (cargo-llvm-cov on nightly, branch counters, **163 tests on the delivered code head**,
`--ignore-filename-regex '[\\/]tests[\\/]'` so the number is the product code; artefact
`experiments/quality/coverage/summary-997f983.txt`; the previous measurement at `ea181d4` is kept as
`summary.txt` for comparison):

| Scope | Lines | Branches | Candidate gate | Met |
|---|---|---|---|---|
| global | **83.19 %** (5016 lines) | **68.64 %** (845) | ≥90 % / ≥85 % | **no** |
| `commands.rs` | 81.66 % | 68.05 % | ≥95 % / ≥90 % | **no** |
| `state.rs` | 92.35 % | 66.07 % | ≥95 % / ≥90 % | **no** |
| `proposal.rs` | 74.94 % | 55.48 % | ≥95 % / ≥90 % | **no** |
| `lock.rs` | 84.16 % | 68.42 % | ≥95 % / ≥90 % | **no** |
| `gitcli.rs` | 91.68 % | 77.14 % | ≥95 % / ≥90 % | **no** |
| `paths.rs` | **96.67 %** | **91.30 %** | ≥95 % / ≥90 % | **yes** |
| `json.rs` | 87.24 % | 81.25 % | ≥95 % / ≥90 % | **no** |
| `scratch.rs` | 71.04 % | 55.56 % | ≥95 % / ≥90 % | **no** |
| `main.rs` | 75.00 % | 80.43 % | ≥95 % / ≥90 % | **no** |
| `sha256.rs` | 100 % | 100 % | — | yes |

Progress over the `ea181d4` measurement: global lines 80.76 % → 83.19 %, branches 65.26 % → 68.64 %;
`paths.rs` crossed its per-module gate (96.67 %/91.30 %) and `lock.rs` went from 72.52 % to 84.16 %
lines. The denominators also grew (4564 → 5016 lines), so this is not a smaller pie. Producing 90 %/85 % would need tests for the
error paths of `proposal.rs` (74.94 %) and `scratch.rs` (71.04 %), which is the honest next step, not
a label.

**Mutation** (`cargo-mutants`, bounded to `paths.rs` + `lock.rs`, **measured on the delivered head
`997f983`**, 20 min): **57 caught / 19 missed / 0 timeout / 6 unviable of 82 tested = 75.0 %**
(57 ÷ 76, timeouts counting as killed). Artefact: `experiments/quality/mutation-997f983/`
(`caught.txt`, `missed.txt`, `unviable.txt`, `outcomes.json`). The same subset scored 50.8 %
before the wave; the nine tests added aimed at exactly the guarantees that survived then
(`canonical_within`'s return value, the nonce, liveness, the acquire budget, the lock error
rendering). The 19 survivors are published with `file:line` and classified rather than excused:

| Class | Count | Detail |
|---|---|---|
| not compiled on this host | 10 | `kill_liveness` and the Linux / non-Linux-Unix `pid_alive` are `#[cfg]`-excluded on Windows, so a Windows run cannot kill them; the Linux job compiles and exercises that path through `liveness_is_answered_for_live_and_dead_pids`. A Linux mutation run is the next step and is **not** claimed here. |
| equivalent at the `Ok`/`Err` level | 3 | `paths.rs:44` (absolute rule), `:48` (drive rule), `:55` (`.`/`..` rule): the mutated code still refuses every input because a later rule catches it, only the message changes. Killing them would pin wording, not behaviour. |
| needs fault injection | 6 | five error-kind / TOCTOU guards in `lock.rs` (`:236`, `:239` ×3, `:294`) and one deadline boundary (`:208`): observable only by injecting an IO failure or winning a race, which this harness cannot do deterministically. |

Whole-crate mutation (960 mutants, ≈24 h serial) is `NOT_RUN`; so is a Linux run of this subset.

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
| Install / uninstall smoke | executed locally, and now **in CI on both OSes** against the same release binary the artefact is built from |
| Release crash hook | inert, proved locally and in CI |
| `cargo audit` | 0 vulnerabilities (1430 advisories, 1 crate) |
| Runtime dependencies | none |
| Reproducible build | **`REPRODUCIBILITY_NOT_PROVEN`** — two clean archive builds gave `978fefb3…` and `30e958d7…` |
| Actions pinned to SHAs | **yes** — every `uses:` is a 40-hex commit with its release in a comment (`checkout v4.4.0`, `rust-toolchain@b725eda…`, `setup-node v4.4.0`, `setup-python v5.6.0`, `upload-artifact v4.6.2`, `msvc-dev-cmd v1.13.0`) |
| Toolchain pinned | **yes** — `rust-toolchain.toml` (`1.95.0` + rustfmt/clippy), the CI action names `1.95.0`, the crate declares `rust-version = "1.95"` |
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

**Vault: repaired and green.** `vault.py check` was red on a pre-existing `migrate` check (four
"normalization hash mismatch" items in `projects/rimatrix` and `projects/seno-daw`, files this session
never touched). Root cause, proved rather than assumed: the v4 normalization plan was generated on
Windows by hashing working-tree bytes, so its 21 entries mixed the CRLF and LF conventions while Git
stores those blobs as LF (`core.autocrlf=true` rewrites them at checkout only); rewriting the four
failing files CRLF→LF reproduced each recorded hash byte for byte. With the authorisation to fix the
pre-existing problems, all 21 destinations were first proved unchanged since their single import
commit (`git log -- <destination>`), then the record was canonicalised to the LF convention and the
check now compares the same representation. `vault.py check` → `lint OK · links OK · idempotence OK ·
migrate OK · root-cleanup OK` (0 issue), commit `083adbc` pushed to `master` (fast-forward). The check
keeps its teeth: appending one line to a destination made it report exactly that file. The session
note for this wave is written and **still uncommitted** by the owner's instruction that the rest of
the vault work stays suspended.

## 10. G10 — independent review

- **Adversarial (agent G, no write access to `crates/flow/src`)**: see
  `FLOW-PRODUCTION-ADVERSARIAL.md` and the summary in §10.1.
- **Maintainability / supply chain (second independent reviewer)**: **executed** — `P1_FOUND`, 18
  findings (1 P1, 8 P2, 9 P3), all closed except one documented limit (§10.2).

### 10.1 Adversarial replay on the candidate head (executed)

The independent reviewer adapted its harness to the `0.2.0-rc.1` surface (fixtures now adopt their
policy, the seven `FUT-*` scenarios are enabled, `.flow` link/junction/hardlink shapes are added) and
re-ran it on `f237d57`; artefacts: `experiments/mvp-qa/results.json`, per-axis table in
`docs/reviews/FLOW-PRODUCTION-ADVERSARIAL.md`.

| Axis | Scenarios | Refuted | Reproduced | NOT_RUN |
|---|---|---|---|---|
| identity | 20 | 20 | 0 | 0 |
| scope | 11 | 11 | 0 | 0 |
| git | 12 | 12 | 0 | 0 |
| durable | 11 | 10 | 0 | 1 |
| files | 23 | 21 | 1 | 1 |
| process | 14 | 13 | 1 | 0 |
| proof | 9 | 5 | 3 | 1 |
| compat | 7 | 6 | 1 | 0 |
| 0.1 surface | 7 | 7 | 0 | 0 |
| inputs | 5 | 4 | 1 | 0 |
| **total** | **119** | **109** | **7** | **3** |

**0 P0.** Severity of the 7: P2 × 3, P3 × 4. What happened to them:

| Finding | Severity | Outcome |
|---|---|---|
| FIL-20 hard-linked `.flow/journal.jsonl` appends outside the repo on **Windows** | P2 | **fixed** in `3da89c4`: the journal is rewritten and renamed, so a hard link is replaced rather than appended to (test `a_hardlinked_journal_is_not_written_through`, now cross-platform); the outside file keeps its bytes |
| PRF-02 missing logs not visible in `status` | P3 | **fixed**: `status.latest_receipt` now carries `evidence_complete` and `missing_logs` |
| INP-03 case-colliding obligation ids share one log file | P3 | **fixed**: refused as `INVALID_POLICY` (`obligation_ids_that_differ_only_by_case_are_refused`) |
| PRC-10 `FLOW_TEST_CRASH` honoured by the **debug** binary (exit 89) | P2 | **accepted, documented**: the hook exists only where `debug_assertions` is on, never in a released artefact, and CI proves the release binary is inert. It is a test signal, not a product behaviour. |
| PRF-01 re-sealed forged receipt publishes | P2 | **accepted limitation**: integrity is not a signature; a local writer with the same rights can forge state. Documented in `doctor` and in this report. |
| PRF-02/PRF-08 retention (deleted logs) | P3 | **documented**: no retention policy; absence is now displayed as incomplete evidence, publication is unaffected |
| CMP-01 `help` prints text | P3 | **documented**: `help` is text by contract; machine commands emit JSON |

The reviewer also re-attacked every fix from §3 (identifier-as-path, tampered display, byte-exact
fidelity, symlink/junction guards, missing `tsc`) and refuted them all, and added a TOCTOU attempt
(`.flow/receipts` swapped during a 5 s obligation) which the guard refused.

**NOT_RUN, with reasons:** `DUR-09` crash *before* the intent append (the harness does not know the
`before-intent` injection point added in this wave — it is covered by
`crash_before_intent_leaves_no_pending_intent`); `FIL-06` reserved names / `..` in a verified tree
(Git refuses to build such an index); `PRF-09` a NUL byte in an argument (Windows `CreateProcess`
rejects it before Flow starts).

A capability note the reviewer raised and this report accepts: there is **no candidate-bound human
approval** — an actor able to run `run --candidate` and `publish --candidate` can publish once the
policy is approved. Per-candidate approval is a product decision, not a defect of this candidate.

### 10.2 Second independent review (executed)

A second reviewer with **no prior context** and **no write access to the code** was run against the
candidate head, read-only (no build, no test run, every claim classified as `CODE_READ`/`CI_READ`/
`DOC_READ`). Artefact: `docs/reviews/FLOW-PRODUCTION-SECOND-REVIEW.md` — 18 findings, `P1_FOUND`.

| ID | Sev | Finding | Disposition |
|---|---|---|---|
| R2-01 | **P1** | `flow publish --dry-run=true` validated and then **published for real**: `validate_flags` accepted an inline `=value` on a boolean flag, `has_flag` matched only the exact token | **fixed** in `e660cdb`: a boolean flag refuses an inline value as a usage error (exit 2). Regression test `a_boolean_flag_with_an_inline_value_is_a_usage_error_before_any_mutation` asserts the ref does not move and no intent is appended, for three spellings |
| R2-02 | P2 | "fsynced intent" overstated: the directory was never flushed | **fixed**: parent directory fsynced on Unix; the README says "file fsynced before the rename" and names the Windows limit |
| R2-03 | P2 | CI never covered `dev` or `main` | **fixed**: both are in the push and pull_request triggers |
| R2-04 | P2 | the toolchain floated (`toolchain: stable`) | **fixed**: `rust-toolchain.toml` + `rust-version = "1.95"` + `toolchain: 1.95.0` |
| R2-05 | P2 | a required gate could print SKIP and still report `... ok` | **fixed**: the link-containment gate now fails closed, and the journal-symlink gate fails closed on Unix while Windows states the privilege limit and the job no longer claims that gate there |
| R2-06 | P2 | four contradictory executed-test counts in the docs | **fixed**: one number per platform, tied to the head (§1), and the CI floor enforces it |
| R2-07 | P2 | the README claimed "production candidate" while its own gate report said `PRODUCTION_PARTIAL` | **fixed**: the README states the verdict and the gaps |
| R2-08 | P2 | `redact()` used `env::vars()`, which panics on a non-Unicode value | **fixed**: `vars_os()` with a lossy view |
| R2-09 | P2 | Unix liveness read only `kill -0`'s exit status, so EPERM counted as "provably dead" and a live foreign lock could be stolen | **fixed**: Linux asks `/proc/<pid>` first and otherwise accepts only a message that proves the process is gone; anything else is `Unknown`, never a steal |
| R2-10 | P3 | three identifier validators with divergent rules | **partly fixed**: `proposal.rs` now uses `paths::is_object_id`; `scratch`'s wider 4..=64 rule is documented as deliberate (Git accepts abbreviated ids there and the value never becomes a file name) |
| R2-11 | P3 | doc comments describing the wrong function | **fixed** |
| R2-12 | P3 | `version.schemas` omitted `flow-status/1` and `flow-proposal-list/1` | **fixed**: built from the `state::SCHEMA_*` constants |
| R2-13 | P3 | `--timeout 0`/`abc` silently dropped while the receipt proved the requested value | **fixed**: a usage error; regression test `a_timeout_that_cannot_be_applied_is_a_usage_error` |
| R2-14 | P3 | a failed `git diff` became an empty `worktree_diff` | **fixed**: the run refuses to certify |
| R2-15 | P3 | `save_receipt` built a path from an unvalidated `tree_oid` | **fixed** |
| R2-16 | P3 | `propose` returned `USAGE` with exit 1 | **fixed**: any document naming `USAGE` exits 2 |
| R2-17 | P3 | the polyglot job claimed "offline" while installing `tsc` | **fixed**: the wording and the README table say the TypeScript step needs the network |
| R2-18 | P3 | missing evidence could upload as a warning | **fixed**: `if-no-files-found: error` |

The reviewer also verified as sound, with the checks that prove it: zero dependencies, no build
script/`unsafe`/network, all 62 then-existing gate names resolving to real test functions, CAS
ordering and post-CAS read-back, typed journal failures, link-guarded `.flow` writes, the release
crash hook compiled out, env scrubbing, and the 64 MiB / 1 MiB bounds. It covered no runtime
behaviour, which is why its verdict is a complement to §10.1 and to CI, not a replacement.

**One finding is not closed and is accepted as a limit:** R2-09's fix makes an unprovable liveness
answer `Unknown` (the lock is then never stolen, which wedges until a human removes it). That is the
documented trade: a wedge is loud, a stolen live lock is a silent double publication.

## 11. Final numbers and the verdict

**Code and CI head under test:** `997f983` (the last commit that touches `crates/flow`,
`.github/`, `Cargo.toml` or `rust-toolchain.toml`). Everything after it is documentation only, so the
CI evidence below is the evidence for the shipped code. Measurements are quoted with the SHA they were
taken on; no number here is copied from an earlier wave.

| Evidence | Value |
|---|---|
| Tests executed | **163 on Windows** (35 unit, 22 bounds, 54 CLI, 9 concurrency, 25 isolation, 18 proposals), **162 on Linux** (the PATH/PATHEXT shim test is Windows-only by design) |
| Failures / ignored | 0 / 0 on both OSes |
| CI (push) | recorded in §11.2 after the last push |
| CI (pull_request) | recorded in §11.2 after the last push |
| Adversarial replay | 119 declared, 116 executed, 109 refuted, 7 reproduced (0 P0, 3 P2, 4 P3), 3 NOT_RUN |
| Second independent review | 18 findings: 1 P1, 8 P2, 9 P3 — P1 and all P2 closed with tests, 1 P3 partly closed by design |
| Coverage (head `997f983`, 163 tests) | **83.19 % lines / 68.64 % branches** global; `paths.rs` 96.67/91.30 and `lock.rs` 84.16/68.42 |
| Mutation (head `997f983`, `paths.rs` + `lock.rs`) | **75.0 % killed** — 57 caught / 19 missed / 0 timeout / 6 unviable of 82; 10 of the 19 are `#[cfg]`-excluded on this host, 3 are equivalent at the `Ok`/`Err` level, 6 need fault injection. Whole-crate run `NOT_RUN` |
| Release candidate | `0.2.0-rc.1`, 1 013 248 B, SHA-256 `d6edbec0…d19bc`, install smoke PASS locally and in CI on both OSes |
| `cargo audit` | 0 vulnerabilities |
| Reproducible build | `REPRODUCIBILITY_NOT_PROVEN` |
| Vault | **green** — `vault.py check` 0 issue after the repair commit `083adbc` |

### 11.1 Verdict

`PRODUCTION_PARTIAL`. The candidate is functional, **no known P0/P1 finding is open in the
local-trusted scope** (the second review's P1 was found and fixed with a regression test in this
wave), CI is green on both supported OSes, the release candidate builds and its install smoke runs on
both OSes, the supply chain is pinned (Actions by commit SHA, toolchain by `rust-toolchain.toml`), and
the trust boundary is explicit. It is **not** labelled `PRODUCTION_CANDIDATE` because:

1. coverage is measured **on this head** and still below the gates this mandate sets (83.19 %/68.64 %
   vs 90 %/85 % global) — `proposal.rs` (74.94 %) and `scratch.rs` (71.04 %) are the reasons;
2. mutation is measured on two modules (75.0 %, up from 50.8 %) and below the 90 % gate; **whole-crate
   mutation** (960 mutants, ≈24 h serial) and a Linux run of the subset are `NOT_RUN`;
3. `cargo-fuzz`/libFuzzer is `NOT_RUN` (a seeded, hand-rolled fuzzer found one real defect, F1, fixed);
4. reproducibility is `NOT_PROVEN`;
5. macOS is `NOT_TESTED_MACOS`, and performance/productivity is `UNPROVEN`.

Closed since the previous verdict: the second independent review (was `NOT_RUN`), the SHA pinning, the
Linux install smoke, the vault block, and the stale measurement on `ea181d4`. None of the remaining
items is a P0 or P1 defect; all are evidence gaps. That is the difference between "the product works
and is honest" and "the qualification dossier is complete", and only the latter justifies
`GO_PRODUCTION`.

### 11.2 CI runs on this head

Recorded after the last push of this branch, so the ids below are verifiable facts rather than
intentions. A run that is not green is written down as red.

