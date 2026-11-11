# Capabilities and their evidence

**Purpose.** Separate what Flow *implements*, what is *demonstrated by executed
evidence*, what is only *specified*, and what is *not available*. This is the
document the production gate reads instead of inferring capability from prose.

**Status vocabulary** (no other value is allowed in the table):

| Status | Meaning |
|---|---|
| `AVAILABLE` | implemented and reachable from the CLI |
| `DEMONSTRATED` | `AVAILABLE` **and** an executed test or CI step proves it on a named SHA and OS |
| `SPECIFIED_ONLY` | a contract or document describes it; nothing enforces it |
| `EXPERIMENTAL` | implemented, exercised ad hoc, no repeatable gate |
| `NOT_AVAILABLE` | absent; do not plan on it |

**Evidence pointer.** Dates, SHAs and CI run ids are in
[`docs/reviews/FLOW-PRODUCTION-WAVE3.md`](reviews/FLOW-PRODUCTION-WAVE3.md) and
[`docs/reviews/FLOW-PRODUCTION-GATE-REPORT.md`](reviews/FLOW-PRODUCTION-GATE-REPORT.md).
When a row names a test, `cargo test --locked --test <file> <name>` reproduces it.

## 1. Core verification and publication

| Capability | Status | Evidence (executed) | Limits |
|---|---|---|---|
| Receipt bound to one exact Git tree OID and one effective policy digest | `DEMONSTRATED` | `run_passing_obligation_binds_the_index_tree`, `p0b_*` staleness gates; CI Ubuntu+Windows | a receipt is integrity-protected, **not signed** |
| The verification checkout equals the tree: every regular file byte for byte, no missing file, no extra file or directory, executable bit on Unix | `DEMONSTRATED` | `git_fidelity.rs` (11 refusal gates + a positive control, plus two probe children), `a_converted_checkout_is_refused_not_certified`, CI both OSes | above the 64 MiB / 20 000-entry budget the run *refuses*; Windows does not carry the executable bit |
| A declared file is re-proved after the obligations ran | `DEMONSTRATED` | `a_delayed_write_during_the_obligations_is_refused`; `checkout_recheck: OK` in a clean receipt | detection only: a write reverted before the re-check, or after it, is not seen |
| Ref publication by compare-and-swap from the expected old OID | `DEMONSTRATED` | `publish_moves_the_ref_with_cas_and_records_the_journal`, `eight_concurrent_publishes_publish_at_most_once` | an external `git` writer is not fenced |
| Durable intent/outcome journal, crash observation, non-destructive reconcile | `DEMONSTRATED` | `crash_*_intent`, `crash_after_ref_*`, `p0c_*`; CI | `FLOW_TEST_CRASH` exists only in debug builds |
| Multi-agent proposals (attributed, immutable, idempotent) and exact-tree candidates | `DEMONSTRATED` | `m1`–`m10`, `proposal.rs`/`error_paths.rs` gates; CI | no LLM or network component; proposals are Git patches |
| Killed verification leaves no proof and the repository recovers | `DEMONSTRATED` | `a_run_killed_during_verification_leaves_no_proof_and_recovers` | the killed process's disposable scratch directory is not removed |

## 2. Trust boundary, secrets, confinement

| Capability | Status | Evidence | Limits |
|---|---|---|---|
| Obligation environment scrubbed to a documented allowlist | `DEMONSTRATED` | `an_unlisted_environment_variable_is_not_inherited_by_an_obligation`, `env_allowlist_is_sorted_and_unique`, `hijacked_git_environment_does_not_redirect_the_cli` | the obligation can still read files as the same user |
| Secret-shaped values redacted **before** persistence in logs/receipts | `DEMONSTRATED` | `a_secret_shaped_value_is_redacted_before_it_is_persisted` (GitHub and AWS shapes), unit tests | a secret read from a file, with no matching variable and no known shape, is **not** redacted |
| Git plumbing cannot be redirected by `GIT_*` variables | `DEMONSTRATED` | `hijacked_git_environment_does_not_redirect_the_cli` + control | — |
| Writes below `.flow/` refuse symlinks, Windows reparse points and Unix hardlinks | `DEMONSTRATED` | `a_symlinked_journal_is_never_written_through`, `a_hardlinked_journal_is_not_written_through`, `m9` | TOCTOU between the guard and the write is documented |
| OS sandbox / capability confinement of obligations | `NOT_AVAILABLE` | `doctor` says `local-trusted`; `execution_boundary` is in every receipt | untrusted code execution is `UNSUPPORTED`; the receipt is not a sandbox claim |
| Hook suppression | `NOT_AVAILABLE` | `doctor` `non_guarantees`, `git_fidelity.rs` hook gates | Git plumbing still runs `post-checkout` and `reference-transaction`; hook **effects on the checkout are detected and refused**, not prevented |
| Per-candidate human approval | `NOT_AVAILABLE` | adversarial report §10.1 (capability note) | whoever can run `run --candidate` and `publish --candidate` can publish once the policy is approved |
| Receipt authentication (signature) | `NOT_AVAILABLE` | `PRF-01`, accepted limitation | integrity ≠ authentication |

## 3. Legacy and compatibility labels

| Capability | Status | Statement |
|---|---|---|
| `AUTO_SAFE` mode | `NOT_AVAILABLE` | Flow has **no** `AUTO_SAFE` mode and no label of that name. Nothing in this repository is enabled, promoted or published on the basis of a textual definition. |
| Ravum V1 `AUTO_SAFE` imported into Flow | `SPECIFIED_ONLY` | The documented mapping is `TEXTUALLY_APPLICABLE` only — never `VALIDATED_FOR_SCOPE`, never a receipt, never a promotion. There is no importer in this repository, so the mapping is a contract, not a capability. |
| `--execution-boundary` values other than `local-trusted` | `NOT_AVAILABLE` | Only `local-trusted` is meaningful; the field is recorded and compared for staleness. |

## 4. Platform, release, measurement

| Capability | Status | Evidence | Limits |
|---|---|---|---|
| Windows (x86_64-pc-windows-msvc) build, test and install smoke | `DEMONSTRATED` | CI `test windows-latest` (4/4 jobs green on the candidate branch); `dist/` provenance | — |
| Linux (x86_64-unknown-linux-gnu) build, test and install smoke | `DEMONSTRATED` | CI `test ubuntu-latest`; local WSL run of the full suite | built on a different host than the artefact provenance records — see the packaging report |
| Reproducible build, per host and toolchain | `DEMONSTRATED` | CI "reproducible build (Windows, two clean archives)" on every Windows run | **cross-host identity is not claimed**; Linux reproducibility not claimed |
| macOS | `NOT_AVAILABLE` | `NOT_TESTED_MACOS` in the gate report | no artefact, no claim |
| Coverage | `DEMONSTRATED` (product-only measurement) | `experiments/quality/` summaries, cargo-llvm-cov; CI `coverage` job on the wave-8 head | 95.88 % lines (4402/4591) / 93.56 % branches (799/854) measured on the candidate head, above the 95/90 gate; test code never enters the product view |
| Mutation score | `DEMONSTRATED` on a bounded module subset (Windows) **and per module on Linux**, including the Unix-gated sites | `experiments/quality/mutation-*/`, `experiments/quality/mutation-w7-linux/`, `experiments/quality/mutation-w8-linux/` | whole-crate mutation `NOT_RUN`; Linux per-module rates are scoped lower bounds (`state` 95.7 %, `commands` 96.7 %, `gitcli` 83.0 %, `main` 93.9 % but partial — three `collect_values` hang mutants not run); 8 `#[cfg(windows)]` mutants are host-gated on Linux; the directory-fsync rule is pinned as a predicate, not as a crash proof |
| Productivity benefit over plain Git / worktrees | `SPECIFIED_ONLY` | `docs/BENCHMARK_S9.md` protocol; exploratory `n=5` pilot | no confirmatory campaign; treat as `UNPROVEN` |
| Adversarial campaign | `DEMONSTRATED` (v2 head) | `experiments/mvp-qa/results*.json`, `FLOW-PRODUCTION-ADVERSARIAL*.md` | 3 scenarios `NOT_RUN`; a scenario that cannot reach its target is never counted as a pass |

## 5. What this document refuses to claim

- That a green CI is a production qualification.
- That the checkout gate prevents a hook or an obligation from mutating the
  checkout *after* the fidelity check, or from writing outside it.
- That any platform other than the two above is supported.
- That a `PASSED` receipt is a security proof: it is a statement about one tree,
  one effective policy and one execution, on a `local-trusted` boundary.
