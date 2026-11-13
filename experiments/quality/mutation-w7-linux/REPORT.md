# Wave 7 — Linux mutation measurement, three rounds (final head `308bf6a`)

Status: **measurement + classification only.** No product source was changed on the Windows
checkout, nothing was committed/pushed, and the only Windows-side writes are inside this directory.
The probes were applied in disposable ext4 clones and reverted (`git reset --hard` to the measured
commit). Evidence is organised per round: `c6acccb/`, `342f92a/`, `308bf6a/`.

## 1. Environment

| Item | Value |
|---|---|
| host | WSL2 Linux 6.6.87.2, ext4 `/home/aes` |
| toolchain | rustc/cargo 1.95.0 (pinned by `rust-toolchain.toml`), git 2.43.0 |
| tool | `cargo-mutants 27.1.0` (installed from crates.io; `install.log`) |
| rounds | `c6acccb` (round 1), `342f92a` (round 2), `308bf6a` (final round) |
| tree drift | after the measurement the branch advanced to `6b520e5` (docs/CI only: `.github/workflows/flow-ci.yml`, `.gitignore`, CHANGELOG/docs — **no change under `crates/flow/src` or `crates/flow/tests`**), so these results describe the current product and test code |
| Windows checkout | untouched; no cargo/build there |

## 2. Round 1 — head `c6acccb` (MEASURED)

- Suite: **456 passed, 0 failed, 0 ignored** (11 suites).
- Probe: `state.rs:340,377`; `gitcli.rs:570/571`; `commands.rs:996,1000,1006,1007`.
- 17 selected (15 + 2 collateral), 2 min: **14 caught / 3 missed / 0 timeout / 0 unviable**;
  **12/15 intended caught** (missed: `commands.rs:1000:41`, `state.rs:340:43 >→<`, `state.rs:377:52`).
- Artefacts: `c6acccb/` (`outcomes-first-run.json`, `caught-first-run.txt`, `missed-first-run.txt`,
  `probe-first-run.diff`, `list-first-run.log`, `mutants-run-first-run.log`, `tests.log`).

## 3. Round 2 — head `342f92a` (MEASURED; had a red fixture, now historical)

At the unmodified head the suite was **red on Linux**: 469 executed, 468 passed, 1 failed —
`a_hook_that_removes_the_executable_bit_from_a_declared_script_is_refused` (`git_fidelity.rs:702`);
cargo-mutants refused to run (`cargo test failed in an unmutated tree`, exit 4). Root cause: the
test forced `script.sh` to 100755 before `install_tracked_hook`, whose `git add -A` re-staged the
0644 worktree file and reset the index mode to 100644, so the tree did not declare an executable
script and the hook's `chmod -x` was a no-op. A measurement-only 8-line fixture correction made the
suite green (469/0/0) and the sites measurable.

- 15-site run: 17 selected; baseline 18 s + 63 s; 2 min: **17 raw caught**.
  - **S3 flake audit:** the raw catch of `state.rs:377:52 delete !` came only from the unrelated
    `scratch::tests::materialise_gives_up_after_the_bounded_number_of_collisions` flaking
    (`fatal: not a git repository`); re-running that mutant alone gave **1 missed**. Adjudicated:
    **16 caught / 1 missed; 14/15 intended caught** (S1 and S2 closed).
- 873/878 addendum: 7 selected; **6 caught / 0 missed / 1 unviable** (intended 6/6).
- Artefacts: `342f92a/` (including `tests-342f92a-unmodified.log`,
  `mutants-342f92a-unmodified-abort.log`, the measurement-corrected canonical run,
  `s3-rerun-*`, the addendum run, and the pre-correction probe run kept as history).

## 4. FINAL ROUND — head `308bf6a` (unmodified; MEASURED)

`308bf6a` is the Lead's fix for the round-2 fixture bug (the `update-index --chmod=+x -- script.sh`
now runs after `install_tracked_hook`). No fixture correction was used in this round.

### 4.1 Suite

```
cargo test --locked --offline --all-targets   (unmodified head)
469 passed, 0 failed, 0 ignored, 11 suites
```

Log: `308bf6a/tests-308bf6a-unmodified.log`. **Green**, so mutation results below are trustworthy.

### 4.2 The 15 sites

Probe lines unchanged at this commit (verified with `sed`): `state.rs:340,377`; `gitcli.rs:570/571`;
`commands.rs:996` (whole-function mutant reported by cargo-mutants at `1001:5`), `1005`, `1011`,
`1012`. `--in-diff` selected the same **17** (15 intended + 2 collateral:
`state.rs:292 guard_below_flow -> Ok(())`, `state.rs:358 write_atomic_bytes -> Ok(())`).

Run: baseline 7 s build + 29 s test; `17 mutants tested in 2m: 1 missed, 16 caught`;
**16 caught / 1 missed / 0 timeout / 0 unviable**.

- **`crates/flow/src/commands.rs:1005:41: replace != with == in verify_checkout_modes` — CAUGHT**
  (killed by `a_hook_that_removes_the_executable_bit_from_a_declared_script_is_refused`).
- **`crates/flow/src/state.rs:340:43: replace > with < in guard_below_flow` — CAUGHT**
  (killed by `a_hardlinked_adoption_record_is_refused_on_unix`).
- The only survivor is **`crates/flow/src/state.rs:377:52: delete ! in write_atomic_bytes`** (S3).
  No adjudication rerun was needed: the raw run already reports MISSED (unlike the 342f92a run, no
  flake affected this campaign). Per-mutant log audit: every caught mutant has a substantive failing
  test; no scratch-flake failure appears in any 308bf6a log.
- Collateral `state.rs:292` and `state.rs:358` are caught.

### 4.3 Addendum — `commands.rs:873` and `:878`

Probe (`308bf6a/probe-addendum.diff`) selects 7: the 6 intended guard mutants (3 per line: `with
true`, `with false`, `delete !`) + 1 collateral whole-function mutant
(`commands.rs:786:5 verify_checkout_fidelity -> Ok(())`).

- **First attempt aborted** at the unmutated baseline due to a **new flake**:
  `scratch::tests::materialise_refuses_a_base_or_a_candidate_inside_the_repository` failed with
  `cannot materialise commit abcd …: fatal: not a git repository` (log:
  `mutants-addendum-aborted-flake.log`; 0 mutants tested). Same class as the S3 flake: these tests
  assume the process-global `next_sequence()` value they read is the one `materialise_in` uses next,
  and parallel tests consume it.
- **Retry with `RUST_TEST_THREADS=1`** (test execution serialised; product code and probe
  unchanged): baseline 5 s + 113 s; `7 mutants tested in 5m: 6 caught, 1 unviable`.
  **Intended 2 sites: 6/6 caught, 0 missed.** Kill tests: `873` →
  `a_symlink_entry_checked_out_as_a_symlink_is_certified` / `p0a_symlink_rewritten_as_a_file_is_refused`;
  `878` → `a_hook_that_replaces_a_file_with_a_symlink_is_refused_with_its_own_reason` plus broad
  suites. The 1 **unviable** is collateral, not a kill: rustc 1.95 ICEs in
  `check_mod_deathness` compiling the whole-function replacement
  (`mutants-addendum/log/…786_col_5.log`).

## 5. Three-round table — the 15 sites

| Site (final numbering) | `c6acccb` | `342f92a` (fixture-corrected) | `308bf6a` (unmodified) |
|---|---|---|---|
| `state.rs:340:27` `&&`→`\|\|` | CAUGHT | CAUGHT | CAUGHT |
| `state.rs:340:43` `>`→`==` | CAUGHT | CAUGHT | CAUGHT |
| `state.rs:340:43` `>`→`<` (S2) | **MISSED** | **CAUGHT** | **CAUGHT** |
| `state.rs:340:43` `>`→`>=` | CAUGHT | CAUGHT | CAUGHT |
| `state.rs:377:52` delete `!` (S3) | **MISSED** | **MISSED** (raw catch was a flake) | **MISSED** |
| `gitcli.rs:571:5` resolve_program → `Some(Default)` | CAUGHT | CAUGHT | CAUGHT |
| `commands.rs:996` whole fn (`1001:5` reported) | CAUGHT | CAUGHT | CAUGHT |
| `commands.rs:1005:21` `!=`→`==` | CAUGHT | CAUGHT | CAUGHT |
| `commands.rs:1005:33` `&&`→`\|\|` | CAUGHT | CAUGHT | CAUGHT |
| `commands.rs:1005:41` `!=`→`==` (S1) | **MISSED** | **CAUGHT** | **CAUGHT** |
| `commands.rs:1011:56` `&`→`\|` | CAUGHT | CAUGHT | CAUGHT |
| `commands.rs:1011:56` `&`→`^` | CAUGHT | CAUGHT | CAUGHT |
| `commands.rs:1011:64` `!=`→`==` | CAUGHT | CAUGHT | CAUGHT |
| `commands.rs:1012:27` `!=`→`==` | CAUGHT | CAUGHT | CAUGHT |
| `commands.rs:1012:36` `==`→`!=` | CAUGHT | CAUGHT | CAUGHT |

Counts: **12/15 → 14/15 → 14/15 intended caught**; S1 and S2 moved MISSED→CAUGHT at 342f92a and stay
caught at the final head; S3 remains the single survivor and is unchanged (not observable in-process
without a crash/power-loss harness; do not count the round-2 flaky raw catch as a kill).

Addendum (separate denominator):

| Head | Selected | Intended result | Collateral |
|---|---|---|---|
| `342f92a` (fixture-corrected) | 7 | 6 caught / 0 missed | 1 unviable (rustc ICE) |
| `308bf6a` (unmodified) | 7 | **6 caught / 0 missed** | 1 unviable (rustc ICE) |

## 6. Flake findings (test-suite stability, not product code)

Two `scratch` unit tests assume the next process-global sequence value is theirs
(`materialise_gives_up_after_the_bounded_number_of_collisions`,
`materialise_refuses_a_base_or_a_candidate_inside_the_repository`). Under mutation runs (large numbers
of short-lived test processes and parallel tests) they can fail with
`cannot materialise commit abcd …: fatal: not a git repository`. Observed effects:
round-2 342f92a raw S3 false catch (re-run: missed; §3) and the round-3 addendum baseline abort
(retried with `RUST_TEST_THREADS=1`; §4.3). The final 15-site campaign itself was unaffected.
Recommendation for future mutation campaigns: run the suite with `RUST_TEST_THREADS=1`, or make
these tests independent of the shared counter (e.g. hold `TEST_LOCK` in every sequence consumer or
derive the expected candidate path from the value returned by the call). The flake is not caused by
any mutated line.

## 7. Evidence layout

```
c6acccb/        round 1: outcomes-first-run.json, caught/missed-first-run.txt, probe-first-run.diff,
                list-first-run.log, mutants-run-first-run.log, tests.log (456 green)
342f92a/        round 2: canonical measurement-corrected run (outcomes.json, caught/missed/timeout/
                unviable, mutants.json, debug.log, diff/, log/, probe.diff, list.log, mutants-run.log),
                pre-correction probe run (outcomes-round2-precorrected.json, …),
                tests-342f92a-unmodified.log (469 executed / 1 failed), fixture-corrected log,
                mutants-342f92a-unmodified-abort.log, s3-rerun-* (flake audit),
                addendum-* (6 caught / 1 unviable)
308bf6a/        final round: tests-308bf6a-unmodified.log (469 passed / 0 failed / 0 ignored),
                mutants-15/ (17 selected: 16 caught, 1 missed), probe-15.diff, list.log,
                mutants-15-run.log, mutants-addendum/ (7 selected: 6 caught, 1 unviable),
                probe-addendum.diff, list-addendum.log, mutants-addendum-run.log,
                mutants-addendum-aborted-flake.log (first attempt, baseline flake)
REPORT.md       this file
install.log     cargo-mutants 27.1.0 install
```

## 8. Conclusion

At the final, unmodified product head `308bf6a` the Linux suite is green (469/0/0) and the campaign
is clean: **14/15 intended sites caught**, with both previously open gaps closed by real tests
(S1 via `a_hook_that_removes_the_executable_bit_from_a_declared_script_is_refused`; S2 via
`a_hardlinked_adoption_record_is_refused_on_unix`), and the addendum sites **6/6 caught**. The only
survivor is the directory-fsync mutant S3, which is unobservable without crash/fault injection. The
two flake observations are test-stability findings, not product or mutant-measurement results.
