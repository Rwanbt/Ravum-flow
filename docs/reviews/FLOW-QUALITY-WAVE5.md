# Flow — Quality wave 5: raise the bar (coverage 95/90, mutation beyond three modules)

Status: **measured on this host**, branch `feat/flow-production-candidate`.
Date: 2026-11-10. Previous wave: [FLOW-QUALITY-WAVE4.md](FLOW-QUALITY-WAVE4.md).
Product verdict unchanged: **`PRODUCTION_PARTIAL`** (see
[FLOW-PRODUCTION-WAVE3.md](FLOW-PRODUCTION-WAVE3.md)). Nothing is merged, tagged
or released; `Rwanbt/Ravum` is not written.

Wave 4 closed the *declared* evidence gaps but left two facts on the table: the
branch target (85 %) was met, not exceeded, and mutation only covered `json`,
`paths` and `lock`. Wave 5 does what the remaining list asked for — it raises the
thresholds and makes them executable — without deleting a test, lowering a gate
or changing a security property.

## 1. What changed in the product (and why it had to)

The uncovered code was concentrated where a function both **runs Git** and
**parses its answer**. A hostile or corrupted Git answer could not be produced
from a test without a fake binary, so the refusals were untestable by
construction. Five pure extractions (no behaviour change, every one called by the
same paths as before) make the untrusted half directly testable:

| Module | Extracted | What is now pinned byte for byte |
|---|---|---|
| `commands.rs` | `parse_tree_entries`, `parse_tree_dirs`, `parse_cat_file_batch` | unparsable tab-less entry, empty path, non-UTF-8 path, entry budget, non-blob object, unparsable/overflowing size, truncated body, byte budget |
| `proposal.rs` | `parse_tree_listing`, `symlink_target_escapes`, `path_conflicts` | gitlink, unsafe path, absolute/parent-escaping symlink targets, rename/delete/mode/symlink collisions between proposals |
| `gitcli.rs` | `resolve_program_in` | PATHEXT search, explicit extension, empty PATH (arguments instead of process-global environment) |
| `scratch.rs`, `state.rs`, `main.rs` | — (tests only) | creation refusals, reparse-point guard, flags/exit codes |

Two robustness defects were found **by writing those tests**, and fixed:

1. `parse_cat_file_batch` computed `end = start + size` and sliced
   `bytes[start..end]`. A hostile header (`size` near `usize::MAX`) overflowed in
   debug and panicked; the parser now uses `checked_add` and refuses.
2. The same loop advanced `position = end + 1` and then did `bytes[position..]`
   for the next requested oid. A batch that ends exactly between two entries —
   a truncated pipe, a killed Git — made `position == len + 1` and panicked
   instead of refusing. The test
   `cat_file_batch_never_panics_when_the_answer_ends_exactly_between_entries`
   reproduces it; `bytes.get(position..)` now refuses.

A panic is not a refusal: it skips `Drop` cleanup and looks like a crash to the
operator. Both are one-line fixes with a regression test, not a redesign.

## 2. Coverage: the two views, before and after

Command (exact, reproducible; `clean` is mandatory because profiles accumulate):

```
CARGO_TARGET_DIR=<external dir> CARGO_BUILD_JOBS=4
cargo +nightly llvm-cov clean --workspace
cargo +nightly llvm-cov --locked --offline --all-targets --branch \
  --ignore-filename-regex '[\\/]tests[\\/]' --no-report
cargo +nightly llvm-cov report --branch --ignore-filename-regex '[\\/]tests[\\/]'
cargo +nightly llvm-cov report --branch --ignore-filename-regex '[\\/]tests[\\/]' \
  --lcov --output-path experiments/quality/coverage/lcov-w5-final2.info
python experiments/quality/analyze_lcov.py \
  experiments/quality/coverage/lcov-w5-final2.info --product-only
```

The prescribed command instruments the unit tests that live in `src/`, so the
plain number counts test code as covered product. Both views are reported; the
gate uses the **product-only** one, and the wave-4 report already established
that distinction. Artefacts: `lcov-w5-baseline.info` (before), `lcov-w5-final2.info`
(after), `run-w5-baseline.log`, `run-w5-final2.log`.

### 2.1 Product-only (the gate's view)

| Module | Lines before | Lines after | Branches before | Branches after |
|---|---|---|---|---|
| `commands.rs` | 93.59 % | **95.67 %** | 87.27 % | **92.55 %** |
| `gitcli.rs` | 90.32 % | **93.19 %** | 82.86 % | **87.14 %** |
| `json.rs` | 96.12 % | 96.12 % | 97.92 % | 97.92 % |
| `lock.rs` | 88.70 % | 88.70 % | 87.50 % | 87.50 % |
| `main.rs` | 97.20 % | **99.60 %** | 93.75 % | **97.92 %** |
| `paths.rs` | 98.00 % | 98.00 % | 100.00 % | 100.00 % |
| `proposal.rs` | 91.67 % | **96.71 %** | 80.82 % | **93.92 %** |
| `scratch.rs` | 77.51 % | **89.16 %** | 72.22 % | **88.89 %** |
| `sha256.rs` | 100.00 % | 100.00 % | 100.00 % | 100.00 % |
| `state.rs` | 98.85 % | **99.08 %** | 85.19 % | **90.74 %** |
| **Total** | **93.04 %** (4209/4524) | **95.74 %** (4380/4575) | **86.17 %** (729/846) | **92.81 %** (787/848) |

### 2.2 Raw llvm-cov view (test code included, for comparison)

`TOTAL lines 93.14 % → 94.96 %`, `branches 85.37 % → 88.02 %`,
`functions 89.78 % → 92.29 %`. The raw view is *lower* on lines and higher on
functions than the product-only view for the same run: the test code in `src/` is
larger than the product lines it exercises, and generic instantiations inflate the
lcov function denominator. Neither number is presented as the other.

### 2.3 The new thresholds

`docs/reviews/FLOW-PRODUCTION-WAVE3.md` §5.3 candidate targets were
**≥ 90 % lines / ≥ 85 % branches**; they are now **≥ 95 % lines / ≥ 90 %
branches, product-only**, and the numbers above exceed both. The gate is no longer
a report paragraph: the `coverage` CI job in
[`.github/workflows/flow-ci.yml`](../../.github/workflows/flow-ci.yml) exits 1
below either threshold. No waiver is used; every measured point counts.

Measured on the CI job itself (Linux, run `42896473842`):
**95.66 % lines (4382/4581), 92.72 % branches (790/852)** — `gate lines: ok`,
`gate branches: ok`. The Linux denominator is 6 lines larger than the Windows one
(six Unix-only lines, e.g. the executable-bit comparison) and lines are 0.08
points lower: the same code, measured where the gate runs.

## 3. Tests added

54 new test functions (422 executed on Windows, because a module's
unit tests are compiled into the integration suites that include it):
`crates/flow/src/{commands,proposal,scratch,gitcli,state,main}.rs` and
`crates/flow/tests/{operator_surfaces,error_paths,git_fidelity}.rs`.

They are grouped by the contract they pin, not by the line they cover:

- **Untrusted Git output** (16): every parser refusal listed in §1, including the
  two never-panic regressions and the 200 000-entry / 20 000-file / 64 MiB
  budgets. A budget is refused, never truncated into a certification.
- **Hostile repositories and state** (9): `.flow` as a reparse point is never
  written through; a failed Git call on `ls-tree` is a refusal even when the
  caller asks for a tree that does not exist; a candidate record whose
  attribution list changed is refused as immutable; a receipt path that is a
  directory is not evidence.
- **Disclosures** (4): doctor lists an absolute `core.hooksPath`, reports
  `cargo=missing`/`rustc=missing` when they really are off PATH, and `init`
  presents an optional obligation as `(optional)` before adoption.
- **Creation and cleanup guards** (10): a vanished or redirected scratch is never
  claimed, 128 collisions exhaust the retry budget instead of looping, a read-only
  file is cleared before removal, a wrong recorded identity is refused.
- **Flags and exit codes** (4): `--name value` vs `--name=value`, a trailing value
  flag, a stray positional argument, `USAGE` ⇒ exit 2.
- **Checkout fidelity** (1): a tree entry that declares a symlink and is checked
  out as a regular file (`core.symlinks=false`) is refused, with no receipt.

CI floor: **422** executed tests on Windows and **419** on Linux (the Linux count was three below Windows in run `42886979136`; the push that closes this wave verifies the projection), 0 ignored, and **17 new named gates** so none of the new
tests can silently disappear. The list of gates is in the workflow.

## 4. The gate was wrong once, and the run says so

The first `coverage` job (run `42886301582`) failed at compile time:

```
error: the option `Z` is only accepted on the nightly compiler
... 1 nightly option were parsed
```

cause: the repository pins `1.95.0` in `rust-toolchain.toml`, so
`cargo llvm-cov` picked `rustc 1.95.0` for the project even though the job had
installed a nightly, and `-Z coverage-options=branch` is nightly-only. The fix is
to name the toolchain explicitly: `cargo +nightly-2026-05-06 llvm-cov ...`. The
nightly is pinned by date, and `cargo-llvm-cov 0.9.1` is built from the crates.io
source with `--locked`; no third-party action decides what is measured. This
paragraph stays in the report because a gate that was red and then fixed is part
of the evidence, not something to overwrite.

After the fix, both events for the same commit are green: push run
`42886979136` and pull-request run `42886998986`, **5/5 jobs each** (test ×2,
polyglot ×2, coverage ×1). The final head (`3da1c46`) is green on both events too:
push `42896473842` and pull-request `42896483330`, **5/5 each**, with the measured
floor and the coverage totals quoted in §2.3 and §3.

## 5. Mutation: the remaining modules

`cargo-mutants 27.1.0`, scoped per module to the suites that exercise it (a
survivor may still be killed by a suite that was not selected, so each rate is a
**lower bound for that module**, never a whole-crate score). Same copy-mode rules
as wave 4: `--gitignore true`, `--jobs 2`, `--timeout 240`, no `--in-place`, and
**no shared `CARGO_TARGET_DIR`** (each mutant gets the private target directory
cargo-mutants creates in its copy — a shared one made two concurrent mutants link
the same `flow-*.exe`, see §4).

| Module | Mutants | Unviable | Applicable | Campaign caught | Campaign missed | Campaign timeout | Campaign strict | After killchecks |
|---|---|---|---|---|---|---|---|---|
| `proposal` | 126 | 18 | 108 | 102 | 6 | 0 | 94.4 % | **108/108 = 100 %** |
| `scratch` | 54 | 5 | 49 | 38 | 7 | 4 | 77.6 % | **44/49 = 89.8 %** |
| **Both** | **180** | **23** | **157** | **140** | **13** | **4** | **89.2 %** | **152/157 = 96.8 %** |

The killchecks are targeted re-runs of exactly the reported sites, after the
assertions that close them (`mutation-w5/killcheck-*`, merged by
`merge_killchecks.py`). They are not a second campaign and do not rewrite the
campaign numbers; both states are reported.

**Resolved by the killchecks** (12 of the 17 non-caught outcomes):
`TempIndex::drop`, the 128-character identity bound, the 4096-byte symlink-target
bound, the 32 MiB patch bound, both `path_conflicts` `||` operators, the
`AlreadyExists`/`add.status == 0` guards in `materialise_in`, `next_sequence`, and
`abandon`'s deadline arithmetic. The four `crash_run` timeouts were **detections,
not hangs**: that suite waits up to 60 s per phase for a fixture that the mutant
never reaches, so the mutation was killed by the test but exceeded the per-mutant
budget; re-run against `--bins --test isolation` it is caught in seconds.

**Still missed, each classified (no "equivalent" label without a proof):**

- `scratch.rs:86` (`||` → `&&` in `still_owned`): equivalent. `symlink_metadata`
  reports `is_symlink() == true, is_dir() == false` for a link on both hosts
  (verified with a junction probe), so both operands are `true` and both operators
  refuse; for a regular file the second gate (the ownership marker cannot exist
  under a file) refuses anyway.
- `scratch.rs:383` (`processed += 1` → `*= 1`): equivalent below the 50 000-entry
  cleanup budget; the removal is still bounded by the wall-clock deadline and the
  result is identical. Distinguishing it needs a tree of more than 50 000 entries,
  which the campaign does not build.
- `scratch.rs:402` (`make_writable` → `()`): unobservable on this host —
  `DeleteFileW` on a read-only file in a writable directory succeeds on Windows 11
  / NTFS (probed directly: `remove = Ok(())`), and the fallback exists for hosts
  where it does not.
- `scratch.rs:162` and `scratch.rs:314` (guards widened to `true`): the refusal
  still happens; only its message changes when `create_dir`/`symlink_metadata`
  fails for a reason other than the guarded one (disk full, permission denied),
  which this host cannot inject deterministically.

**Not run in this wave**: `state` (155 mutants), `gitcli` (104), `main` (73) and
`commands` (267). They are listed in
`experiments/quality/mutation-w5/run-mutations.ps1` with their scopes; the
campaign was stopped before `state` rather than leave a partial module that could
be mistaken for a result. `commands` in particular needs its own budget decision:
267 mutants × the CLI suites is a multi-hour run on this host. **[Wave 6](FLOW-QUALITY-WAVE6.md)
ran all four from a worktree pinned at this report's head commit; its §3 supersedes this
paragraph, and its §5 classifies every survivor that remains.**

Limits: mutants that live in a `#[cfg(unix)]` block are "not compiled on this
host" on Windows, not "killed"; `Default::default()` replacements are unviable by
construction; a timeout is a hang and is counted separately from both. The
`--bins`-only scoping mistake of wave 4 (`lock` at 60 %) is why every module here
names its integration suites.

## 6. What is still not run, with the reason

- **Linux mutation**: the campaign above is a Windows-host measurement. A 2-core
  GitHub runner would take hours for the same list and needs its own budget
  decision (H4), so no manual workflow was added that could not be finished.
- **`cargo-fuzz`**: a coverage-guided harness needs `libfuzzer-sys` as a dev
  dependency and a resolving network step; the crate is intentionally
  zero-dependency and `--locked --offline` in CI. The deterministic parser tests
  above cover the same refusals, including both historical panics, but they are
  not coverage-guided fuzzing and are not claimed as such. [Wave
  6](FLOW-QUALITY-WAVE6.md) adds a randomised (not coverage-guided) sweep over the
  same boundary and keeps the same limit.
- **macOS**: not measured; the platform-gated tests describe what is expected,
  not what was observed there.
- **S9 confirmatory comparison**: still blocked by `H4` (human threshold/budget
  decision); wave 5 does not consume that decision.
- Product decisions unchanged: sandbox `H3`, receipt signing (`PRF-01`), evidence
  retention (`PRF-02/08`), and the external-`git`-writer fence.
- Uncovered branch points that remain are listed per module by
  `analyze_lcov.py --file <module>`; they are of the same classes as wave 4
  (Git machine output that a fake Git must produce, injected I/O faults, races,
  and OS-impossible values such as NUL in `argv`), and the 92.81 % branch result
  carries a 2.81-point margin above the gate.

## 7. Reproduce

```bash
cargo test --locked --offline --all-targets        # 422 executed on Windows
cargo clippy --locked --offline --all-targets -- -D warnings
cargo fmt --all --check

# coverage (needs a nightly with llvm-tools-preview; see §2)
# mutation (see experiments/quality/mutation-w5/run-mutations.ps1)
# gate (same command the CI job runs)
python experiments/quality/analyze_lcov.py \
  experiments/quality/coverage/lcov-w5-final2.info \
  --product-only --fail-under-lines 95 --fail-under-branches 90
```
