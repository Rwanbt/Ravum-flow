# Flow — Quality wave 9: the host-gated mutants, the value-collector watchdog, and a clean stability baseline

Status: **measurement + eight test additions.** No gate was waived, no test removed, no assertion
weakened, and **no product code changed**: this wave edits tests, evidence and documentation only.
Nothing was merged, tagged or released; `Rwanbt/Ravum` was not touched.

## 1. Mandate and scope

Wave 8 closed the last assertion gaps and produced per-module mutation rates on Linux, and it named
three limits that an agent on this host could still close:

- **eight `#[cfg(windows)]` mutants were host-gated** — `state.rs:316` ×3 (the reparse-point attribute
  test in `guard_below_flow`) and `gitcli.rs:523:5` ×2, `:539:5` ×2, `:543:21` ×1 (the Windows program
  resolver). A Linux binary cannot execute them, so the tool counted them as missed and the Linux rate
  was reported both with and without them;
- **the `main` row was partial**: three `collect_values` index-advance mutants (`360:19`, `372:15` both
  shapes) were `NOT_RUN` after the WSL distro died mid-pass, and a walk that stops advancing hangs the
  suite, so the campaign could not report a kill;
- **the stability claim was entailed**: wave 8's run 7 failed with `ERROR_NO_SYSTEM_RESOURCES` /
  `ERROR_COMMITMENT_LIMIT` because ten full-suite passes ran concurrently with a mutation campaign on a
  16 GB host. The item was deliberately left unproven instead of relabelled.

Wave 9 measures the eight host-gated mutants on their native host, adds the watchdog and the missing
wrapper test, re-measures the mutants those tests touch, and re-runs the stability schedule on an
otherwise idle machine. It also records why the two weakest modules (`lock`, `scratch`) did not move
as much as their uncovered lines suggested.

Out of scope by host, contract or human decision, unchanged: macOS, `cargo-fuzz` (zero-dependency
contract), S9/H4, the product decisions (H3 sandbox, PRF-01 signing, PRF-02/08 retention), and every
merge/tag/release action.

## 2. Method

Each campaign pins `HEAD` in a detached worktree, appends a one-token comment to the exact target line,
takes `git diff -U0`, and runs

```
cargo mutants --in-diff probe.diff --jobs N --timeout 240 --gitignore true --output <dir> -- --bins
```

so the diff selects exactly those mutants (plus the whole-function mutants that share the line). The
probe is reverted with `git checkout --` and the worktree is verified clean afterwards. The scope is
`--bins` (the unit suite), so every rate below is **a lower bound on that suite, never a crate-wide
claim**. A flake is never counted as a kill: an outcome is a kill only when a test fails.

The scripts and their logs are committed: `experiments/quality/mutation-w9-win/`
(`run-win-hostgated.ps1`, `run-win-recheck.ps1`, `probe-win.diff`, `probe-recheck.diff`,
`mutants.out/{caught,missed,unviable,timeout}.txt`).

## 3. Results

### 3.1 The eight host-gated mutants (Windows, head `2ad5bf5`) — 6 caught, 2 missed

| target | mutants | caught | missed |
|---|---|---|---|
| `state.rs:316` (reparse attribute test) | 3 (`&`→`\|`, `&`→`^`, `!=`→`==`) | **3** | 0 |
| `gitcli.rs:539:5` (`path?` early return) | 2 (`-> None`, `-> Some(Default)`) | **2** | 0 |
| `gitcli.rs:543:21` (`!e.is_empty()` filter) | 1 (`delete !`) | **1** | 0 |
| `gitcli.rs:523:5` (`resolve_program` wrapper) | 2 (`-> None`, `-> Some(Default)`) | 0 | **2** |

A ninth mutant was selected as whole-function collateral and caught:
`state.rs:292:5 guard_below_flow -> Ok(())`. Totals: **9 selected — 7 caught, 2 missed, 0 unviable,
0 timeout.**

The reparse guard is therefore measured, not assumed: replacing the attribute mask or the comparison
makes `a_reparse_point_below_flow_is_never_written_through` refuse the write for the wrong reason or not
at all, and the suite notices.

### 3.2 A real gap: the Windows program resolver's wrapper was never executed

Both `resolve_program` mutants survived the entire unit suite. The cause is visible in the code: the
resolution *rules* live in `resolve_program_in(program, path, pathext)`, which the tests drive with
explicit arguments, while `resolve_program` — the thin wrapper that reads `PATH` and `PATHEXT` from the
real process environment — had no test at all. A `None` there makes every `.cmd`/`.bat` shim installed
by npm, npx or gradle invisible, so a tool the policy declares would be reported `UNSUPPORTED` while it
is installed.

The new test `gitcli::tests::resolve_program_reads_the_process_environment` drives the wrapper through
the real environment: it resolves `cmd` (a system tool that must be reachable through `PATH`), asserts
that the answer is a real file with stem `cmd`, and asserts that an unresolvable name stays `None` so
the caller keeps the declared name and reports `UNSUPPORTED` instead of spawning a placeholder. Both
mutant shapes are killed by these assertions, and the wrapper and its rules are now one function under
test.

### 3.3 The `collect_values` watchdog (`main`) — three `NOT_RUN` mutants, now killed

`collect_values` advances by two after a value flag (`360:19`) and by one otherwise (`372:15`). Turning
either `+=` into `*= 2` leaves the index at zero; turning `+= 1` into `-= 1` cycles between two indices
in debug. All three make the walk loop forever, which is why wave 8 could only report `NOT_RUN`: a hung
test binary is a timeout, not a failure.

Wave 9 adds a `watched(seconds, walk)` helper beside the existing `validate_flags` watchdogs and uses
it twice:

- `repeated_and_inline_values_keep_their_order` now runs under a **2 s** watchdog (short on purpose: a
  non-advancing walk pushes the same value forever, and the timeout bounds that growth);
- `a_value_walk_that_stops_advancing_is_caught` drives two **allocation-free** hangs — a value flag with
  no following value (`+= 2` mutant) and a stray token that matches no name (`+= 1` mutants) — under a
  10 s watchdog, so the failure is attributable without a growing vector.

### 3.4 Re-measurement at `8d21b7b`: 10 selected, 10 caught

| target | mutants | outcome |
|---|---|---|
| `main.rs:360:19` | `+=`→`*=` and `+=`→`-=` | **caught** |
| `main.rs:372:15` | `+=`→`*=` and `+=`→`-=` | **caught** |
| `gitcli.rs:523:5` | `-> None`, `-> Some(Default)` | **caught** |
| `main.rs:352:5` (collateral) | `collect_values` → `None` / `Some(vec![])` / `Some(vec![String::new()])` / `Some(vec!["xyzzy"])` | **caught** |

Totals: **10 caught, 0 missed, 0 unviable, 0 timeout.** The three `NOT_RUN` cells of the wave-8 `main`
row are closed, and the two wrapper survivors are closed by a test, not by an exemption.

### 3.5 Module rates after wave 9

Measured mutants only; each row names its head and its scope.

| module | applicable | measured caught | missed | timeout | rate | note |
|---|---|---|---|---|---|---|
| `state` | 140 | 137 | 3 | 0 | **97.9 %** | the 3 host-gated are now measured; the 3 remaining were closed by assertions at `3b1e33e` and are **not** re-measured here |
| `commands` | 245 | 237 | 8 | 0 | **96.7 %** | unchanged from wave 8; 6 of 8 killed by the full suite |
| `gitcli` | 88 | 78 | 10 | 0 | **88.6 %** | +5 host-gated (3 caught, 2 caught after §3.2); the 10 remaining are the timing/zombie boundaries |
| `main` | 69 | 65 | 2 | 2 | **94.2 %** | +3 `NOT_RUN` → caught; the 2 timeouts are killed by the unit watchdog in <15 s, so 67/69 = 97.1 % adjudicated |

Aggregate over the four modules: **517/542 = 95.4 %** measured caught (519/542 = 95.8 % with the two
timeouts adjudicated). No lower-bound caveat is dropped: the suites are still scoped, so this is a
module-subset rate, not a crate-wide mutation score.

### 3.6 Coverage (same toolchain as the CI gate)

Measured with `nightly-2026-05-06` + `cargo-llvm-cov 0.9.1`, `--all-targets --branch`,
`--ignore-filename-regex '[\\/]tests[\\/]'`, product-only view (`analyze_lcov.py --product-only`):

| head | lines | branches | gate |
|---|---|---|---|
| `8d21b7b` (before the new tests) | 95.85 % (4388/4578) | 93.41 % (794/850) | pass |
| `47af568` (after) | 95.87 % (4389/4578) | 93.29 % (793/850) | pass |

The delta is one line and minus one branch, and that is the honest result: the remaining uncovered
regions of `lock.rs` and `scratch.rs` are one-sided error arms (`create_dir` failing for a reason other
than `AlreadyExists`, `symlink_metadata` failing for a reason other than `NotFound`, `tasklist` failing
to spawn) that no test can reach without fault injection, and `lock.rs:239`
(`Err(NotFound) => continue`) is only reachable when a *concurrent* creator's file disappears between a
failed hard link and the read — so its coverage varies run to run (29/32 then 28/32) with no code
change. The tests added are kept for the behaviour they pin, not for a percentage: a state directory
that is a file, a lock attributed to a dead pid, a scratch base that is a file, a `.git` link that
`git worktree add` did not create, and a read-only `.git` file.

### 3.7 Stability, serialised (head `47af568`)

`experiments/quality/stability-w9/windows-stability.ps1`, ten full-suite passes, alternating the
default thread count and `RUST_TEST_THREADS=1`, on an idle host (no mutation, coverage or build campaign
concurrent), each run recorded in `windows-stability.csv`:

| runs | result | notes |
|---|---|---|
| 10/10 | **480 passed, 0 failed, 0 ignored** | `exit=0` every run; 73–100 s at the default thread count, 274–320 s single-threaded |

This replaces the wave-8 evidence that run 7 entailed. The claim is now "ten consecutive full-suite
passes on an idle host", which is what the wave-8 checklist item asked for; it is not a claim of
deterministic absence of flakes under any load.

## 4. What this wave does not close

1. **The five modules never measured on Linux** (`json`, `paths`, `lock`, `proposal`, `scratch`,
   ≈401 mutants). Still `NOT_RUN` on Linux; the Windows rates from waves 4–5 stand alone. A WSL2
   campaign of that size needs a quiet host and roughly an hour.
2. **The three `state` survivors closed by `3b1e33e`** (`377:52`, `390:41`, `661:22`) are not
   re-measured by this wave; they are labelled "closed by a later commit", not "measured".
3. **The ten `gitcli` survivors** (reader-grace race, exact-nanosecond boundary, zombie reaping) still
   need a crash/synchronisation harness, not a unit test.
4. **`dist/` provenance drift** (found by the G10 review in this wave):
   `dist/PROVENANCE.json` pins `f5bb271`, not the candidate head, and `dist/` is gitignored, so the
   provenance and SBOM are not versioned. Regenerating the artefacts at a publishable SHA and
   versioning the manifest is a build action that needs its own GO.
5. **macOS, `cargo-fuzz`, S9/H4, the product decisions, and merge/tag/release**: unchanged.

## 5. Evidence index

- `experiments/quality/mutation-w9-win/run-win-hostgated.ps1` + `run.log`, `probe-win.diff`,
  `mutants/mutants.out/{caught,missed}.txt` — §3.1, head `2ad5bf5`.
- `experiments/quality/mutation-w9-win/run-win-recheck.ps1` + `recheck/run.log`,
  `recheck/probe-recheck.diff`, `recheck/mutants/mutants.out/caught.txt` — §3.3–3.4, head `8d21b7b`.
- `experiments/quality/coverage/lcov-w9-baseline.info`, `lcov-w9-after.info`, `gaps-lock*.txt`,
  `gaps-scratch.txt` — §3.6.
- `experiments/quality/stability-w9/windows-stability.csv` — §3.7.
- Tests added: `main::tests::{a_value_walk_that_stops_advancing_is_caught,
  repeated_and_inline_values_keep_their_order}`,
  `gitcli::tests::resolve_program_reads_the_process_environment`,
  `lock::tests::{a_state_directory_that_is_a_file_is_an_io_error,
  inspect_attributes_a_lock_to_a_dead_pid}`,
  `scratch::tests::{a_scratch_base_that_is_a_file_is_refused_before_anything_is_created,
  detach_git_link_refuses_a_tree_without_a_git_link, detach_git_link_clears_a_read_only_git_file}`.

## 6. Verdict

Unchanged on the product: **`PRODUCTION_PARTIAL`**. What changed is the quality evidence: the
mutation matrix has no unmeasured host-gated mutant left, the `main` row is no longer partial, one real
Windows test gap is closed, and the stability claim has a clean, serialised basis.
