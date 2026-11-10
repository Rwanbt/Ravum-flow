# FLOW production candidate — wave 3: fidelity, secrets, crash recovery, artefacts

**Date:** 2026-11-10 (Europe/Paris) · **Branch:** `feat/flow-production-candidate` · **PR:** #54 (draft)
**Start head:** `0de8c21` (`PRODUCTION_PARTIAL`, see the [gate report](FLOW-PRODUCTION-GATE-REPORT.md))
**Code head of this wave:** `7a034ff` · **Branch head (docs/scripts only after it):** `de86b3e`
**Verdict of the wave:** `PRODUCTION_PARTIAL` still, but with the audit's P0 fidelity class
**measured closed**, the secrets boundary pinned, a killed verification recovered, and
release artefacts built with provenance on both platforms. The remaining gaps are
**evidence gaps and product decisions**, listed in §9; none is a known P0/P1 defect in the
`local-trusted` scope.

This document is the authoritative record of wave 3. The detailed sub-reports are:

| Subject | Document |
|---|---|
| Independent attack on the P0 fix (19 scenarios, the four defects it found) | [`FLOW-P0-GIT-FIDELITY-REVIEW.md`](FLOW-P0-GIT-FIDELITY-REVIEW.md), then [`FLOW-P0-GIT-FIDELITY-RECHECK.md`](FLOW-P0-GIT-FIDELITY-RECHECK.md) |
| Adversarial campaign replay on this wave | [`FLOW-PRODUCTION-ADVERSARIAL-WAVE3.md`](FLOW-PRODUCTION-ADVERSARIAL-WAVE3.md) |
| Coverage and mutation measurements | [`FLOW-QUALITY-WAVE3.md`](FLOW-QUALITY-WAVE3.md) |
| Artefacts, provenance, SBOM, smoke tests | [`FLOW-PACKAGING-WAVE3.md`](FLOW-PACKAGING-WAVE3.md) |
| Capability matrix (available / demonstrated / specified-only / not available) | [`../CAPABILITIES.md`](../CAPABILITIES.md) |

---

## 1. Mandate and scope

The owner's mandate for this wave is autonomous recovery and hardening of *Ravum Flow*:
reproduce and fix blocked or failing work, strengthen the security and reliability
guarantees, and move the project towards a version that is objectively qualifiable. It does
**not** authorise merging PR #54, publishing a release, changing branch protection,
ratifying an ADR, or writing to `Rwanbt/Ravum` (another session owns that repository; it was
never opened). The vault records the mandate (see §10).

Everything below is bounded to this repository, disposable fixtures under the system temp
directory and fictitious canaries. No credential, private path or vault content is copied
into this public repository.

## 2. The P0 class the audit named, and what closed it

The gate report left two fidelity weaknesses open. Both were reproduced on `0de8c21`
**before** any product change, and both are now closed with gates that fail if the code
regresses:

### 2.1 A local attribute source converted the checkout undetected

`verify_checkout_content` compared bytes only when a conversion "looked possible" —
`core.autocrlf`, `core.eol`, or an in-tree `.gitattributes`. A conversion driven by a
**local** attribute source (`.git/info/attributes`, `core.attributesFile`, a `filter.*`
driver named by one of them) was invisible, while `git worktree add` applied it anyway: the
receipt could say `PASSED` for content the obligations never saw.

Reproduced (red-first, on the pre-fix build): `.git/info/attributes` + `text eol=crlf`,
`core.attributesFile` (outside the repository), and a real `smudge` driver that rewrites
bytes at equal length — all three gave `PASSED`, exit 0, and a written receipt.

Fix: the comparison is now **unconditional** for every regular file, through one bounded
`git cat-file --batch`; above 64 MiB or 20 000 entries the tree is **refused**, never
skipped. The gate no longer depends on a guess about the host configuration.

### 2.2 A hook could change the checkout and still be certified

`verify_checkout_fidelity` rejected only *missing* entries. `git worktree add` runs
`post-checkout` (including through a `core.hooksPath` pointing at a directory the tree
itself ships), so a hook could add a file, add an empty directory, rewrite a file at equal
length, chmod it, or delete it.

Fix: the checkout must be, when checked, **exactly the tree** — every regular file byte for
byte, the executable bit on Unix, no missing file, no file the tree does not declare, no
directory the tree does not declare. Only the scratch ownership marker and the `.git` link
(already removed) are Flow's own.

### 2.3 A delayed write after the one-shot check was certified

The independent reviewer (A07) reproduced a **P1**: a tracked `post-checkout` hook that
spawns a writer changing a declared file after the check. The obligation then reads content
that is not in the tree and the run is certified. Fix: after the last obligation,
`verify_checkout_unchanged` re-lists the tree and re-proves every declared file (bytes and,
on Unix, mode). A change refuses the run (`CHECKOUT_MUTATED_DURING_RUN`), no receipt is
written, and a clean receipt carries `checkout_recheck: "OK"`.

This is **detection, not isolation**: a write that is reverted before the re-check, or one
that happens after it, is not seen. `doctor` states that limit, and the capability matrix
repeats it. The honest claim is: *a change to a declared file while the obligations run
cannot be certified*.

Two smaller findings from the same review were also closed: the file-set check skipped
every root name starting with `.flow-scratch` (only the exact marker is Flow's now), and a
non-UTF-8 tree path was folded to `U+FFFD` and misdiagnosed as a case collision (it is now
refused with "not valid UTF-8").

### 2.4 Architecture: alternatives considered

| Strategy | Decision |
|---|---|
| A. Validate the final Git tree | **used** — the receipt binds the tree OID, and the published commit carries exactly that tree (`commit-tree` + `rev-parse ^{tree}` before CAS) |
| B. Build the commit from an isolated index | **used** — proposals and candidates are applied with `git apply --cached` into a temporary index and frozen with `write-tree`; no working tree, no hook, no filter |
| C. Verify objects and the tree before publication | **used** — `cat-file -e`, the tree check, the receipt re-derivation, the CAS and the post-CAS read-back |
| D. Isolate untrusted hooks and filters | **not achieved** — Flow has no OS sandbox; hooks are **not suppressed**. Their effects on the disposable checkout are detected and refused, and the publication path does not depend on them (the published tree is built by Git plumbing, not by a checkout) |
| E. Combination | **chosen** (A+B+C + detection), with the residual TOCTOU documented rather than claimed away |

`--no-verify` is not used anywhere as a substitute for validation. The hooks are documented
as *not suppressed* and their role is stated in `doctor` and in `docs/MVP-USER-GUIDE.md` §11.

## 3. Secrets and the trust boundary

- **The obligation environment is scrubbed** to a documented allowlist, proven end to end:
  a canary exported by the operator is absent from the obligation process, and no durable
  file under `.flow/` holds it (`an_unlisted_environment_variable_is_not_inherited_by_an_obligation`).
- **Redaction happens before persistence** and now covers AWS access-key ids (`AKIA`/`ASIA`)
  and the names `*ACCESS_KEY*`, `*CREDENTIAL*`, `*AUTH_TOKEN*`, `*APIKEY*`. The pre-fix build
  leaked an AWS-shaped canary into `.flow/logs/`, which the new gate reproduced before the
  fix (`a_secret_shaped_value_is_redacted_before_it_is_persisted`).
- **The limit is stated, not hidden**: a secret read from a *file* by the obligation, with no
  matching environment variable and no recognisable shape, is not redacted. Flow has no
  sandbox and no secret broker, and `doctor` says so.
- **No `AUTO_SAFE` mode exists.** Nothing is activated, promoted or published on a textual
  definition; the documented mapping of the legacy Ravum V1 label is `TEXTUALLY_APPLICABLE`
  only, and there is no importer in this repository. This is now explicit in
  [`CAPABILITIES.md`](../CAPABILITIES.md) §3.

## 4. Concurrency and crash behaviour

The audit's mandatory scenarios and the executed test that covers each one:

| # | Scenario | Gate |
|---|---|---|
| 1 | Two agents modify the same file | `m2_three_agents_collision_is_diagnosed_and_resolved_explicitly` |
| 2 | Two agents modify independent zones | `m1_two_disjoint_agents_publish_exactly_the_verified_tree` |
| 3 | Two simultaneous commits/publishes | `two_concurrent_publishes_publish_at_most_once`, `eight_concurrent_publishes_publish_at_most_once` |
| 4 | An agent interrupted during a publication | `crash_before_intent_…`, `crash_after_intent_…`, `crash_after_ref_…` |
| 5 | An agent restarted after a crash | `p0c_reconcile_is_idempotent`, `p0c_status_refuses_to_guess_on_a_corrupt_journal` |
| 6 | A process killed during a validation | **new**: `a_run_killed_during_verification_leaves_no_proof_and_recovers` |
| 7 | An abandoned lock | `a_dead_owners_lock_is_recovered`, `a_live_old_lock_is_never_stolen` |
| 8 | A refused publication | `blocking_reference_transaction_aborts_publish_without_moving_the_ref` |
| 9 | An external Git actor moves the ref | `p0c_ref_moved_elsewhere_needs_reconciliation` |
| 10 | The authorised scope changes after validation | `p0b_adding_a_required_obligation_makes_the_receipt_stale` and the other `p0b_*` staleness gates |
| 11 | A coordinator restart | there is no coordinator process: the state is the journal, and `reconcile`/`status` refuse to guess (`p0c_*`) |
| 12 | A cancellation during a build or test | `obligation_timeout_is_timeout_not_pass`, plus #6 |

The killed-run gate asserts what matters: no receipt (a killed run proves nothing), no
publication lock, an empty journal, and a repository that still verifies and publishes on the
next attempt. Its honest limit is documented in the test and in §9: the killed process's
disposable scratch directory is not removed (its `Drop` never ran).

## 5. Verification evidence

### 5.1 Executed tests (final tree, both platforms)

| Platform | Total | Suites |
|---|---|---|
| Windows 11, rustc 1.95.0 | **232 passed / 0 failed / 0 ignored** | unit 37, bounds 24, CLI 55, concurrency 9, crash_run 6, error_paths 32, git_fidelity 16, isolation 27, proposals 18, secrets 8 |
| Linux (WSL Ubuntu, rustc 1.95.0) | **233 passed / 0 failed / 0 ignored** | same, CLI 54 (PATH/PATHEXT test is Windows-only) and git_fidelity 18 (two Unix-only gates) |

The 69 new tests of this wave: 6 original fidelity gates + 1 smudge + 1 Unix mode + 2
containment + 1 delayed-write + 1 non-UTF-8 + 1 control (`git_fidelity`), 8 secrets
(`secrets`), 6 killed-run (`crash_run`), 32 error paths (`error_paths`), 1 usage-error gate
(CLI), and 2 unit tests for `redact()`.

### 5.2 CI (GitHub Actions, `.github/workflows/flow-ci.yml`)

Every run executes `cargo fmt --check`, `cargo clippy -D warnings`, the **gated test step**
(executed-test floor + 79 named gates, 0 ignored), `cargo build --release`, the Windows
reproducible-build check, the install smoke on both OSes, the release crash-hook proof, the
R0-a self-tests and the polyglot job — and, since this wave, **packages the release artefact
from the exact commit** and uploads it.

| SHA | run (push) | result |
|---|---|---|
| `369b3e0` | `42796083386` | 4/4 success |
| `8cbefb2` | `42796534520` | 4/4 success |
| `5077c50` | `42797621168` | 4/4 success |
| `ccdbec9` | `42799312898` | 4/4 success |
| `d7e1015`, `2e9b122` | `42800468915`, `42800689970` | **failure** — the wave-3 tests failed because the implementation of §2.3 was not in the commit (see §7) |
| `7a034ff` | `42801612167` | Ubuntu 4/4 job success; Windows test job failed at the **packaging** step (a release-script bug, fixed in `de86b3e`) |
| `de86b3e` | recorded in the closing report | the run that qualifies the artefact |

The floor and the gate list are part of the workflow, so deleting or renaming a gate fails the
build; `cargo test --all-targets` counts *executed* tests, so a deleted test cannot pass
silently.

### 5.3 Coverage and mutation (measured on the final source, see `FLOW-QUALITY-WAVE3.md`)

| Scope | Gate report (`997f983`) | Wave-3 baseline (`dfb941d`) | **Final (`7a034ff`)** |
|---|---|---|---|
| Global lines | 83.19 % | 83.27 % | **88.68 %** |
| Global branches | 68.64 % | 69.11 % | **75.67 %** |
| `proposal.rs` | 74.94 / 55.48 | 74.94 / 55.48 | **90.91 / 80.14** |
| `scratch.rs` | 71.04 / 55.56 | 71.04 / 55.56 | 75.42 / 59.72 |

The candidate gates (90 % lines / 85 % branches) are **not met**. The growth is from real
refusal tests (32 error-path tests, +5.4 points on their own), not from assertions added for
the percentage. `scratch.rs` cannot honestly reach 85 % under the prescribed command: the
`tests/` filter also discards source modules imported by integration tests through
`#[path]`, so the direct-module paths are invisible to the metric (documented in the quality
report with the exact uncovered lines).

Mutation: bounded to `proposal.rs`, **13 of 119 mutants tested in ~14 min (8 caught, 2
missed, 3 unviable, 0 timeout, 106 NOT_RUN)** — every viable mutant re-runs the whole suite,
so the full run does not fit the session budget. Both survivors were real gaps
(`MAX_PATCH_BYTES` not pinned from below); the new 1 MiB admission test kills them, verified
by a targeted rerun (**2/2 caught**). Whole-crate mutation remains `NOT_RUN`.

### 5.4 Adversarial campaign

Wave 2 declared 119 scenarios; this wave replayed them and added three (`W3-01..03`), then
implemented the two that were previously `NOT_RUN`:

- Final campaign, `122` declared: **115 REFUTED / 6 REPRODUCED / 1 NOT_RUN / 0 ERROR**, no
  P0. The six reproduced are the known accepted ones (`FIL-20` P2 hard-link journal, `PRC-10`
  P2 debug-only crash hook, `PRF-01` P2 integrity ≠ signature, `PRF-02`/`PRF-08` P3 evidence
  retention, `CMP-01` P3 `help` is text). `DUR-09` (crash before the intent append) and
  `FIL-06` (reserved names in a verified tree) are now executed and **refuted**; `PRF-09` (a
  NUL byte in an argument) is restated as untestable — the OS rejects the argv before Flow
  starts — and is not counted as a pass.
- A harness defect was found and fixed during the re-check: `W3-03` asserted the old message
  substring `different bytes`, so a correct typed refusal (`UNSUPPORTED_CHECKOUT`, reason
  `changed during checkout`) was scored as a reproduced defect. The scenario now accepts the
  new wording; with the corrected assertion the campaign is **6 REPRODUCED / 115 REFUTED / 1
  NOT_RUN**, unchanged from wave 3.
- The independent reviewer's 19-scenario attack on the P0 fix is in
  [`FLOW-P0-GIT-FIDELITY-REVIEW.md`](FLOW-P0-GIT-FIDELITY-REVIEW.md): 15 refuted, 4 defects
  (the P1 of §2.3 and three P3s), all fixed and re-checked in
  [`FLOW-P0-GIT-FIDELITY-RECHECK.md`](FLOW-P0-GIT-FIDELITY-RECHECK.md).

## 6. Distribution artefacts

Both platforms are packaged by the same scripts a human runs
(`scripts/release/build-windows.ps1`, `build-linux.sh`), and CI runs them on every platform
run. Each artefact directory carries the binary, `LICENSE`, `SHA256SUMS.txt`,
`PROVENANCE.json` (commit, target triple, toolchain, exact build command, measured test
counts, licence block with the normalised hash) and `SBOM.json` (single component, no
dependencies, declared licence). The exact hashes, the reproducibility verdict per platform
and the 14-case clean-room smoke matrix are in
[`FLOW-PACKAGING-WAVE3.md`](FLOW-PACKAGING-WAVE3.md). Reproducibility is claimed **per host
and toolchain only**; cross-host identity is not claimed, and macOS is not tested.

## 7. What went wrong (kept in the record)

`bae7390` shipped the tests and the documentation of §2.3 **without its implementation**. The
cause: while proving the delayed-write gate red, the working copy was restored from `HEAD`
(which was the pre-fix commit), and that restore overwrote the uncommitted fix; the later
commit staged the restored file. CI failed on `d7e1015` and `2e9b122`, the independent
reviewer and the release agent independently identified the same gap, and the release script
refused to package a red tree. `7a034ff` re-lands the implementation and CI is green on it.
The gate did its job; the mistake was human process, recorded here rather than deleted.

A second, smaller incident in the same wave: the first CI run of the new packaging step
failed on windows-latest because the script piped `git archive` into MSYS `tar`, whose
argument conversion mangled the Windows destination. The fix (in `de86b3e`) invokes the
**System32 `tar.exe`** (bsdtar) explicitly, which does not mangle the path; CI produces valid
artefacts with it. To keep this record matching the code: the commit message of `de86b3e`
describes a zip/`Expand-Archive` variant, which was one of the attempted fixes but is **not**
what the committed script does. The behaviour is what matters and it is measured in CI.

## 8. CI cost and why no impact-based selection was added

Measured on the successful `8cbefb2` run: Windows test job 2.9 min (the gated `cargo test`
step 1.9 min), Ubuntu 0.9 min, polyglot 0.5–1.1 min, whole run 3.0 min. After this wave the
packaging step adds ~1–2 min per platform. At that size an impact-analysis gate (running a
subset for "small" changes) would trade a real risk of missing a cross-cutting regression for
a couple of minutes; the conservative rule in the mandate — *when in doubt, run more* — is
already the cheaper option. The pipeline is cached only where it is deterministic (pinned
toolchain, no dependencies, no registry cache to warm); `concurrency` cancels superseded runs
on the same ref.

## 9. Remaining risks and gaps (none is a known P0/P1 defect)

| # | Gap | Severity | Why it is not closed here |
|---|---|---|---|
| 1 | A hook/obligation can write to the checkout *outside* the re-check window (write-and-revert, or after the re-check); a hostile obligation runs with the user's rights (no OS sandbox) | P1 (product decision) | Requires a real sandbox or a read-only snapshot; both are product/architecture decisions (`H3`), not a defect to paper over |
| 2 | A receipt is integrity-protected but not signed: a local writer with the same rights can forge one (`PRF-01`) | P2 (accepted) | Needs a key/authentication model (a product decision) |
| 3 | Evidence retention: deleting `.flow/logs` is displayed as incomplete evidence but publication is unaffected (`PRF-02`/`PRF-08`) | P3 (documented) | Needs a retention/rotation policy |
| 4 | Coverage 88.68 % / 75.67 % and mutation 75 % (subset) are below the candidate gates; whole-crate mutation and the Linux mutation run are `NOT_RUN`; `cargo-fuzz` is `NOT_RUN` | Evidence gap | The full mutation run needs ~24 h of CPU; the coverage gap is dominated by `scratch.rs` paths invisible to the prescribed metric |
| 5 | Productivity benefit over plain Git/worktrees is `UNPROVEN`: no confirmatory campaign on this head (the exploratory `n=5` pilot remains the only data) | Evidence gap | The pre-registered protocol needs a quiet machine and a fresh campaign; it is not a code defect |
| 6 | macOS is `NOT_TESTED_MACOS`; cross-host reproducibility is not claimed | Scope limit | No macOS runner in scope; the reproducible-build flags are MSVC-specific |
| 7 | An external `git` writer is not fenced (only Flow's own writers are serialised) | Documented | Would need repository-level hooks or server-side enforcement |

## 10. Vault and traceability

The private vault records this wave: the session note, the project index entry, the
initiative evidence and the resume memory are updated with the SHA, the measured numbers and
the remaining gaps. `vault.py check` must be green before the commit; the previous session's
draft note is marked as an intermediate state rather than rewritten, so history is preserved.
No vault path or content is copied into this repository.

## 11. How to reproduce this wave

```bash
# gates
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --offline                     # 232 Windows / 233 Linux

# coverage (see experiments/quality/coverage/summary-7a034ff-wave3.txt)
cargo +nightly llvm-cov clean --workspace
cargo +nightly llvm-cov --locked --offline --all-targets --branch \
  --ignore-filename-regex '[\\/]tests[\\/]' --no-report
cargo +nightly llvm-cov report --branch --ignore-filename-regex '[\\/]tests[\\/]'

# adversarial campaign
python experiments/mvp-qa/attacks.py --flow target/debug/flow[.exe] \
  --release-flow target/release/flow[.exe] \
  --out experiments/mvp-qa/results-wave3.json \
  --markdown experiments/verify-p0/verdict-tables-wave3.md --base-sha <sha>

# artefacts (one pipeline, also used by CI)
pwsh -NoProfile -File scripts/release/build-windows.ps1 -Sha <sha>
RUN_TESTS=0 TEST_LOG=<log> bash scripts/release/build-linux.sh <sha>
```
