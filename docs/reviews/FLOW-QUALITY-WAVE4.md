# FLOW QUALITY WAVE 4 — the candidate coverage gates are met, mutation is widened

**Date:** 2026-11-10 (Europe/Paris) · **Branch:** `feat/flow-production-candidate` · **PR:** #54 (draft)
**Code head of this wave:** `3e03209` — two test-only commits (`42ed7e0`, `3e03209`); the product source is unchanged
**Verdict:** the two *evidence* gaps this repository carried since wave 3 — coverage below the
candidate gates and mutation bounded to one module — are closed or bounded honestly. The product
verdict of the [wave-3 record](FLOW-PRODUCTION-WAVE3.md) is unchanged: `PRODUCTION_PARTIAL`, no
known P0/P1 in the `local-trusted` scope, and every remaining item is a product decision or a
documented limit (§7).

Classification: `MEASURED` = executed on this host, command recorded; `CODE_READ` = read from
source, not executed; `NOT_RUN` = not attempted/blocked.

## 1. Mandate and scope

The mandate is the same autonomous goal as wave 3: strengthen the evidence behind the
`local-trusted` claims without weakening any gate. This wave changes **no product line**, **no
threshold**, and **no existing test**. It adds:

* `crates/flow/tests/operator_surfaces.rs` — 30 tests driving the shipped binary;
* unit tests inside six modules for private refusal paths the CLI cannot reach deterministically;
* `experiments/quality/analyze_lcov.py` — the lcov analyser used for every number below, with a
  `--product-only` view that removes each module's `#[cfg(test)]` block;
* the CI floor and gate list (349 executed tests, 11 new named gates);
* a per-module mutation campaign with its scoped denominators.

Nothing was merged, tagged, released or published. `Rwanbt/Ravum` was never opened.

## 2. Method (MEASURED)

The measurement command is unchanged from wave 3, so the comparison is like-for-like:

```
CARGO_TARGET_DIR=<external dir> CARGO_BUILD_JOBS=4
cargo +nightly llvm-cov clean --workspace          # mandatory: profiles accumulate
cargo +nightly llvm-cov --locked --offline --all-targets --branch \
  --ignore-filename-regex '[\\/]tests[\\/]' --no-report
cargo +nightly llvm-cov report --branch --ignore-filename-regex '[\\/]tests[\\/]'
cargo +nightly llvm-cov report ... --lcov --output-path lcov-w4-final2.info
```

Artefacts: `experiments/quality/coverage/lcov-w4-{baseline,after,after2,final}.info` and the
matching `run-w4-*.log`. Wave 3's two measurement defects still apply and are still respected:
the `tests/` filter also discards source modules imported by integration tests through
`#[path]`, and a run without `clean` merges stale profiles (two identical cleaned runs still
vary by one branch point).

**A third defect, found here.** The prescribed command instruments unit-test code that lives in
`src/`. Adding many unit tests therefore raises the line denominator with lines that are
"covered" by themselves, and adds *branch points for the tests' own assertions*. Two views are
published so neither hides the other:

| View | Lines | Branches | Functions |
|---|---|---|---|
| prescribed command | 93.14 % (5582 lines) | 85.37 % (861 points) | 89.78 % |
| product-only (`#[cfg(test)]` excluded) | 93.04 % (4524 lines) | 86.17 % (846 points) | — |

Both meet the candidate gates (90 % lines / 85 % branches / 85 % functions). The product-only
view is the one to quote for "how much of the product does the suite exercise"; the prescribed
view is the one CI can reproduce with one command. `--product-only` is a *reporting* flag, never
a gate: nothing in CI or in the acceptance criteria depends on it.

## 3. Results (MEASURED)

| Scope | Wave 3 (`7a034ff`) | **Wave 4 (`3e03209`)** | Δ branches | Candidate gate |
|---|---|---|---|---|
| **global lines** | 88.68 % | **93.14 %** | — | ≥ 90 % **met** |
| **global branches** | 75.67 % | **85.37 %** | +9.70 | ≥ 85 % **met** |
| **global functions** | — | **89.78 %** | — | ≥ 85 % **met** |
| `commands.rs` | 86.83 / 73.54 | 93.00 / 87.31 | +13.77 | — |
| `gitcli.rs` | 91.35 / 80.00 | 91.68 / 82.86 | +2.86 | — |
| `json.rs` | 87.24 / 81.25 | 96.56 / 97.92 | +16.67 | — |
| `lock.rs` | 84.47 / 71.05 | 91.62 / 76.32 | +5.27 | — |
| `main.rs` | 94.02 / 83.33 | 97.20 / 93.75 | +10.42 | — |
| `paths.rs` | 96.67 / 91.30 | 97.95 / 100.00 | +8.70 | — |
| `proposal.rs` | 90.91 / 80.14 | 91.04 / 80.82 | +0.68 | — |
| `scratch.rs` | 75.42 / 59.72 | 85.51 / 71.62 | +11.90 | — |
| `state.rs` | 92.56 / 66.07 | 96.50 / 81.03 | +14.96 | — |
| `sha256.rs` | 100 / 100 | 100 / 100 | — | — |

Executed tests: **232 → 349** on Windows (233 → 349 on Linux, same test set plus/minus the
platform-gated ones). Nothing was deleted, renamed away, weakened or `#[ignore]`d; the CI step
still refuses any ignored test and still requires 12 more blocking gates by name.

## 4. What the new tests pin (MEASURED)

`tests/operator_surfaces.rs` (30 tests; the binary also re-runs the 8 `json` unit tests it
imports, as `error_paths.rs` already does for `json`/`sha256`; real Git repositories, no mock,
no shell of the host):

| Group | Tests | Contract |
|---|---|---|
| `init` adoption | 4 | a committed policy is *shown* (command, required/optional, digest) and adopts nothing; `--adopt-policy` adopts exactly that digest; a torn JSON config or a foreign schema is `CONFIG_UNREADABLE` (never reinterpreted); a policy with no required obligation is `INVALID_POLICY`; `--force` preserves the journal byte for byte and records itself as the approval source |
| `doctor` disclosure | 6 | a policy changed since approval is `WARN` (exit 0); a corrupt adoption record or config is `FAILED` (exit 1, `overall=FAILED`); hooks, `.gitattributes`, `.git/info/attributes`, `core.attributesFile`, `core.ignorecase=true` and a live lock are named; a disabled policy is `WARN` and publication is refused by name; a missing hooks/receipts directory is not a failure; an unparseable lock is `FAILED`; a policy that was never adopted is `WARN` |
| `publish` refusals | 5 | an unusable ref name (space, `..`) is `REF_NOT_ALLOWED` before the lock is taken and before any journal line; a detached HEAD without `--ref` names the remedy; an unknown candidate is `UNKNOWN_CANDIDATE` (`--name=value` reaches the same gate); a candidate whose tree was pruned is `CANDIDATE_TREE_MISSING` for `publish` *and* for `run`, with no journal line and no lock residue; an unattributable lock is `LOCK_UNKNOWN` + `manual_required` and the file is preserved |
| `reconcile` | 4 | an intent missing `ref`, `old` or `new` is `INCOMPLETE_INTENT` and the journal is untouched; two intents sharing an id are both reported `DUPLICATE_INTENT_ID`; a decisive observation (`PUBLISHED`, `NOT_PUBLISHED`) becomes a durable outcome while an ambiguous one (`REF_MOVED`) keeps the run in `NEEDS_RECONCILIATION`; an unattributable lock is refused and a nested ref that vanished is `REF_ABSENT` |
| proposals | 4 | `proposals` on a repository with no state is an empty list; `proposals show` on an unknown id is `UNKNOWN_PROPOSAL`; `assemble` twice reuses the candidate and writes one record; `explain --candidate` unknown is `UNKNOWN_CANDIDATE` |
| status edges | 4 | a corrupt receipt file is not displayed as evidence; blank journal lines are layout, not intents; an unreadable adoption record is an error in `policy`, never "adopted: true"; a repository that was never initialised reports `policy: null` |
| approval and flags | 3 | `approve` refuses a policy that proves nothing or has duplicate ids and writes no adoption; a value flag without a value is a usage error (exit 2) that touches nothing; `--` is accepted as a separator |

Unit tests added inside modules (private contracts, `CODE_READ`-verifiable behaviour):

* `json`: writer shapes (empty containers, integral/non-integral numbers, control-character
  escaping), accessors answer "not this shape" instead of panicking, and the parser refuses
  `+1`, truncated literals, a missing colon and a trailing comma.
* `state`: a journal keeps a record's own schema and a missing journal is empty, blank lines are
  not records; an obligation without a program is refused; a config whose `obligations` is not a
  list keeps its declared defaults; a foreign adoption schema is not reinterpreted; a receipt can
  only be stored under an object id and a known slot; the civil calendar handles pre-epoch days.
* `lock`: a guard whose file vanished is dropped without panicking; a lock path that cannot hold
  a record is unattributable and preserved; a record that disappears while waiting is not
  attributed; an empty record is never attributed.
* `paths`: backslash-absolute and `./` shapes are refused; a path with no name cannot be
  resolved; the Windows junction helper reports failure instead of claiming success.
* `scratch`: `Drop` never deletes a directory without the ownership marker, a file where the
  scratch was, a marker that is itself a directory, or content behind a redirect that replaced
  the scratch; an owned marked tree *is* removed; `verify_created_dir` refuses a missing path and
  a file; `detach_git_link` refuses a missing link and a `.git` directory, and removes a
  read-only link; an expired cleanup budget removes nothing.
* `gitcli`: a secret-named variable whose value is too short is not replaced, and a bare token
  prefix with no payload is not a token.

## 5. Honesty notes

* **No threshold moved.** The gates are the ones wave 3 declared (90 / 85 / 85). The CI floor
  rose because more tests exist (349), not because a check was relaxed.
* **The product is unchanged.** `crates/flow/src` changes in this wave are `#[cfg(test)]` blocks
  only; the non-test diff is zero. This is checkable: `git show 3e03209 -- crates/flow/src`
  removes every hunk inside a `mod tests`.
* **Unit-test code is instrumented.** See §2: the prescribed number includes it. Both views are
  published, and the report never quotes the better one alone.
* **What remains uncovered is named, not rounded away** (§7.2): fake-`git` output parsing,
  argv-impossible values, release-only code paths, injected filesystem faults, and two branches
  this wave believes are dead (`INCOMPLETE` status; the `levels` of `run` when `scope_mode` is
  already `selected`).
* **The mutation numbers are scoped** (§6): each module is mutated against the binaries that
  exercise it. A survivor may still be killed by a suite that was not selected, so the rates are
  lower bounds per module, never a whole-crate claim.

## 6. Mutation campaign (MEASURED)

### 6.1 Method and scope

`cargo-mutants` 27.1.0, one module per run, two jobs, 240 s per mutant, copy mode with
`--gitignore true` (the repository `.gitignore` keeps build and coverage artefacts out of every
copy; `--in-place` cannot be combined with `--jobs`). Whole-suite mutation is ~70 s per viable
mutant because every test spawns real `flow` processes (~24 h for the crate), so three modules
were run in full instead, and the scope is published with the numbers.

`cargo-mutants` does **not** generate mutants inside `#[cfg(test)]` blocks — verified against the
listing: the highest mutant line in `json.rs` is 348 while its test block starts at 353, and in
`paths.rs` 101 vs 147. Every rate below is therefore product code only.

Scripts: `experiments/quality/mutation-w4/run-mutations.ps1` (campaign),
`run-killcheck.ps1` (targeted re-verification and the corrected lock run), `summarize.py`
(aggregation from `outcomes.json`).

### 6.2 Results

| module | mutants | unviable | not compiled on this host | tested | caught | missed | timeout | strict kill | detected |
|---|---|---|---|---|---|---|---|---|---|
| `json.rs` | 139 | 10 | 0 | 129 | **125** | 1 | 3 | **96.9 %** | 99.2 % |
| `paths.rs` | 31 | 0 | 0 | 31 | **31** | 0 | 0 | **100 %** | 100 % |
| `lock.rs` | 51 | 6 | 10 | 35 | **27** | 4 | 4 | **77.1 %** | 88.6 % |
| **total** | **221** | 16 | 10 | **195** | **183** | 5 | 7 | **93.8 %** | **97.4 %** |

"strict kill" = caught / tested; "detected" also counts the 7 timeouts, which are mutants that
make the suite *hang* (an infinite loop in the whitespace/number scanner or in the lock acquire
loop) and hit the per-mutant timeout — a detected failure, not a pass, but counted separately so
the reader can choose the stricter number. "unviable" mutants do not compile (e.g. substituting
`Json::default()`: `Json` implements no `Default`) and are excluded from every denominator.

`json` and `paths` include survivors re-verified by a targeted killcheck **after** the assertions
that close them were added: the campaign itself measured 113/129 (87.6 %) and 28/31 (90.3 %); the
killcheck re-tested exactly those sites and reports 22/26 then 5/5 caught, so the table's rows are
the post-fix state and the pre-fix numbers are stated here rather than hidden. `lock` is the
corrected re-run of §6.4.

### 6.3 Survivors, classified

| Mutant | Class | Why it survived / action taken |
|---|---|---|
| `json.rs:79` `n.abs() < 9.0e15` → `<=` | **equivalent** | the integral fast path and the float path emit identical text for every integral `f64` inside 9e15; `-0.0` is now pinned so the `>` and `==` variants are killed, this one changes no observable byte |
| `lock.rs:208` `<` → `<=` | equivalent (timing) | boundary compare of the 750 ms grace loop: one extra poll, no observable contract |
| `lock.rs:236` match guard → `true` | not covered (fault injection) | needs a lock-create error that is neither `AlreadyExists` nor a readable file (e.g. permission denied on the temporary name) |
| `lock.rs:239` match guard → `false` | not covered (race) | the creator vanishes between `create_new` and the read: a sub-millisecond window |
| `lock.rs:294` match guard → `true` | not covered (race) | a successor replaces the record between observation and removal; the guard's **drop** twin (`a_guard_never_removes_a_successors_lock`) is covered |
| 10 × `kill_liveness` / Linux-Unix `pid_alive` | **not applicable on this host** | `#[cfg(not(windows))]` / `#[cfg(all(unix, not(linux)))]`: the mutated text is never compiled on Windows, so the suite passes trivially. A Linux mutation run is `NOT_RUN`; the Windows liveness probe is covered by `liveness_is_answered_for_live_and_dead_pids` |
| 16 × `Default::default()`-style substitutions | unviable | do not compile; excluded from every rate |

No survivor is a known uncaught behaviour: three are fault/race injection, two are equivalent or
timing-only, ten are host-excluded, sixteen do not compile.

### 6.4 A scoping mistake, corrected rather than explained

The first lock run used `--bins` only. That suite exercises acquire/release, the live owner, the
unparseable record and its own guard's nonce — but **not** the dead-owner steal path, which is
reached from `tests/cli.rs` (`a_dead_owners_lock_is_recovered`) and the concurrency suites. The run
therefore reported 60 % (27 caught, 16 missed, 2 timeouts) and its four `lock.rs:294` survivors were
an artefact of the test selection, not of the code. `lock2` re-runs the same 51 mutants with
`--bins --test cli --test concurrency`: the `294` guard mutants are then caught or detected as
hangs. The wrong first number is kept in the artefacts (`lock/` next to `lock2/`) and recorded here,
so the record shows how the mistake was found instead of only its correction.

### 6.5 Limits

* Three modules, not the crate: `commands.rs` (177 mutants) and `scratch.rs`, `main.rs`,
  `proposal.rs`, `state.rs`, `gitcli.rs` are `NOT_RUN` for mutation this wave.
* No Linux mutation run: the ten host-excluded mutants are exactly the ones a Linux run would test.
* A scoped run can only under-kill: a survivor may be caught by a suite that was not selected, so
  the rates are lower bounds for their module.
* `cargo-fuzz` remains `NOT_RUN`; the seeded JSON/CLI fuzzers under `experiments/quality/fuzz/`
  are unchanged this wave.

## 7. Remaining gaps (none is a known P0/P1 defect)

### 7.1 Product decisions and documented limits (unchanged from wave 3)

A hostile obligation runs with the user's rights (no OS sandbox, `H3`); a receipt is
integrity-protected but not signed (`PRF-01`); evidence retention has no rotation policy
(`PRF-02`/`PRF-08`); an external `git` writer is not fenced; macOS is `NOT_TESTED_MACOS`;
productivity over plain Git/worktrees stays `UNPROVEN` because the confirmatory `S9` campaign
needs `H4` (a human threshold and budget). None of these is changed, hidden or re-scoped here.

### 7.2 Uncovered branches and why they are not covered

| Module | Points | Classes (exact lines in `lcov-w4-final2.info`) |
|---|---|---|
| `commands.rs` | 41 | Git's own machine output (`ls-tree`, `cat-file --batch` parsing), walker type mismatches that need a hook replacing a directory by a symlink, publish crash points and CAS re-reads that need a concurrent writer, a NUL inside `--ref` (argv cannot carry it), and dead or environment-destroying arms (`INCOMPLETE` status, `git_ok` false, `PATH` absent, pre-existing absolute hooks dir) |
| `proposal.rs` | 28 | unreadable/unparsable Git output; seed/`write-tree`/atomic-write failures *after* a successful Git call; the documented `EMPTY_PROPOSAL`/`.git`-path refusals that `git apply` refuses first |
| `scratch.rs` | 21 | the wave-3 §3.1 list: `Drop` false paths that need an external process in a sub-millisecond window, `create_dir` exhaustion, `worktree add` failure, marker-write failure, non-UTF-8 scratch paths, Windows read-only retry |
| `gitcli.rs` | 12 | process-runner timeout/truncation/`EINTR` sub-branches whose observable contract is already asserted, and the `resolve_program` arms that need the process-global `PATH` mutated mid-suite |
| `state.rs` | 11 | a path with no parent (unreachable), the Unix-only hard-link check and reparse arm, the civil-calendar pre-epoch arm, and serializer arms for shapes the caller never produces |
| `lock.rs` | 9 | a create error that is neither `AlreadyExists` nor a readable file, a `NotFound` race mid-acquire, a `tasklist` failure, and the boundary compare of the grace loop |
| `main.rs` | 3 | the `--name=value` form of a value flag that is last on the line, a positional for `proposals`, and one arm of the value-flag lookahead |
| `json.rs` | 1 | truncated UTF-8 inside a `&str`: impossible by construction (the parser takes a valid `&str`) |

Total: 126 branch points (14.6 % of 861). Four classes: Git's own machine output or a failure
injected into Git/the filesystem (the bulk of `commands.rs`, `proposal.rs`, `scratch.rs`,
`gitcli.rs`); values the OS cannot deliver (a NUL inside an argv entry, truncated UTF-8 inside a
`&str`); defensive branches this wave believes are dead (`INCOMPLETE` status, an unreachable
`PATH`-absent arm); and Windows-only cleanup faults and liveness probes that need a second host or
a race window. The analyser prints every one with source context:
`python experiments/quality/analyze_lcov.py lcov-w4-final2.info --repo . --context 1`.

## 8. Reproduction

```bash
# gates (all green on 3e03209)
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --offline            # 349 Windows / 349 Linux

# coverage (exact commands in run-w4-final2.log)
cargo +nightly llvm-cov clean --workspace
cargo +nightly llvm-cov --locked --offline --all-targets --branch \
  --ignore-filename-regex '[\\/]tests[\\/]' --no-report
cargo +nightly llvm-cov report --branch --ignore-filename-regex '[\\/]tests[\\/]'
cargo +nightly llvm-cov report --branch --ignore-filename-regex '[\\/]tests[\\/]' --lcov \
  --output-path experiments/quality/coverage/lcov-w4-final2.info

# the two views (same lcov file)
python experiments/quality/analyze_lcov.py experiments/quality/coverage/lcov-w4-final2.info \
  --repo . --product-only
python experiments/quality/analyze_lcov.py experiments/quality/coverage/lcov-w4-final2.info \
  --repo .

# mutation (one module per run; the script records every log)
pwsh -NoProfile -File experiments/quality/mutation-w4/run-mutations.ps1
```

## 9. Cross-links

* Wave-3 record and its remaining gaps: [`FLOW-PRODUCTION-WAVE3.md`](FLOW-PRODUCTION-WAVE3.md) §5.3, §9.
* Wave-3 coverage method and the unreachable lists: [`FLOW-QUALITY-WAVE3.md`](FLOW-QUALITY-WAVE3.md) §2–§3.
* Fidelity contract the new operator tests touch: [`../CAPABILITIES.md`](../CAPABILITIES.md),
  [`git_fidelity.rs`](../../crates/flow/tests/git_fidelity.rs).
* Adversarial campaign (unchanged this wave): [`FLOW-PRODUCTION-ADVERSARIAL-WAVE3.md`](FLOW-PRODUCTION-ADVERSARIAL-WAVE3.md).
