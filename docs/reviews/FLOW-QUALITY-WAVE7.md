# Flow — Quality wave 7: the last assertion gaps, the documented boundaries, and the Unix sites measured on Linux

Status: **measurement + tests + one refusal-message fix.** No gate was waived, no test removed, no
security property weakened. Nothing was merged, tagged or released; `Rwanbt/Ravum` was not touched.

## 1. Mandate and scope

[Wave 6](FLOW-QUALITY-WAVE6.md) measured the four mutation modules it had left `NOT_RUN` and closed
87 reported survivor sites: 45 killed by new assertions, 42 classified. Of those 42, three classes
were explicitly left as follow-ups:

- **13 assertion gaps** — real coverage gaps in the operator surfaces, each needing a test on the
  field or message its mutant changes.
- **3 documented boundaries** the fixtures did not reach (20 000 regular files, a 64 MiB batch, a
  1 024-character ref).
- **15 sites in `#[cfg(unix)]` / `#[cfg(not(windows))]` code** that Windows never compiles, listed as
  "not compiled here", never as killed or equivalent.

Wave 7 closes the first two classes with tests, measures the third on Linux, and fixes one refusal
that named the symptom instead of the cause. It also found — and fixed — a defect in one of the new
tests themselves (§3.2), which is reported rather than hidden: the test was red on Linux at the first
committed head and was caught by the Linux suite run, not by the Windows one (where it is
`#[cfg(unix)]` and therefore not compiled).

## 2. Method

- **Windows killcheck** (`cargo-mutants 27.1.0`): a throwaway worktree pinned at the committed test
  head, a probe that appends a trailing comment to each reported line so `git diff -U0` marks it, then
  `cargo mutants --file … --in-diff probe.diff --jobs 2 --timeout 240 --gitignore true -- --bins
  --test cli …`. The probe stays in the tree for the run (the tool validates the diff post-image) and
  is reverted afterwards. The scoped suite runs for every mutant, so a kill means *the suite* kills
  it, not one hand-picked test. A site that the diff also touches can select **collateral**
  whole-function mutants of the enclosing function; they are reported separately and never counted in
  the site's own result.
- **Linux measurement**: WSL2 Ubuntu, ext4, rustc/cargo 1.95.0, `cargo-mutants 27.1.0` installed from
  crates.io; a clean clone at the product head, the same probe/`--in-diff` method, `--jobs 2`. The
  Windows checkout is untouched by that run; the evidence is in
  [`experiments/quality/mutation-w7-linux`](../../experiments/quality/mutation-w7-linux/REPORT.md).
- **No dependency was added** to the crate: the tooling lives outside `Cargo.toml` and the crate still
  builds with `--locked --offline`.

## 3. The 15 Unix-only sites, measured on Linux (MEASURED)

The sites wave 6 could only record as "not compiled on this host" were measured in a WSL2 ext4 clone,
in two rounds (wave-6 head and wave-7 head) plus an addendum for the two cross-platform type guards
that no Windows test can distinguish.

### 3.1 Round 1 — wave-6 head `c6acccb`

17 mutants selected (the 15 plus two whole-function collateral mutants of
`guard_below_flow`/`write_atomic_bytes` — `--in-diff` selects by span overlap):

| Module / line | What the gate is | Mutants | Outcome |
|---|---|---|---|
| `state.rs:340` | Unix hard-link refusal (`nlink > 1`) | 4 | **3 caught, 1 missed** (`>`→`<` disables the guard) |
| `state.rs:377` | directory fsync after the atomic rename | 1 | **missed** (`delete !`) |
| `gitcli.rs:571` | non-Windows `resolve_program` stub | 1 | **caught** |
| `commands.rs:996,1000,1006,1007` | executable-bit proof of the checkout | 9 | **8 caught, 1 missed** (`1000:41`, which skips *declared-executable* files) |
| **total** | | **15** | **12 caught (80.0 %), 3 missed** |

All 15 compiled on Linux: wave-6's "not compiled on this host" was host-gating, not a phantom mutant.
Two of the three survivors were **real gaps**: the existing exec-bit test only makes a `100644` file
executable (never reaching the skipped `100755` branch), and the existing hard-link test is written to
pass with or without the Unix guard. Both were closed by tests (§4) and the measurement was repeated.

### 3.2 Round 2 — wave-7 head `308bf6a`

The same sites with the new tests present: **14 of the 15 intended sites caught**, the two gaps above
now killed by name:

- `commands.rs:1005:41` (old numbering `1000:41`, `!=`→`==`, skips declared-`100755` entries) —
  **caught** by `a_hook_that_removes_the_executable_bit_from_a_declared_script_is_refused`.
- `state.rs:340:43` (`>`→`<`, disables the link-count guard) — **caught** by
  `a_hardlinked_adoption_record_is_refused_on_unix`.
- `state.rs:377:52` (`delete !`, directory fsync skipped) — **still missed**. Its raw "caught" in the
  first pass was a **flake in an unrelated `scratch` unit test**; a single-mutant rerun gave `1
  missed`, and that adjudication is the recorded outcome. The mutant weakens the documented durability
  claim but changes nothing observable in-process: `sync_all` reports nothing to the caller, no fsync
  failure is injected, and making the parent disappear between `rename` and `fsync` is a race (the
  rename requires the parent). Killing it needs a crash/power-loss harness or an `LD_PRELOAD`/FUSE
  shim failing a directory `fsync`. **Not observable here; not claimed equivalent.**
- The 2 collateral mutants were caught in both rounds: 16/17 adjudicated at this head.

The test fixture defect found in this round is part of the record: at the first committed head, the
new 100755 test was **red on Linux** because `install_tracked_hook` stages everything with `git add
-A`, re-staging `script.sh` from the `0644` worktree file and resetting the index mode to `100644` —
the tree declared a non-executable file and the hook's `chmod -x` was a no-op. Fixed in `308bf6a` by
forcing the bit **after** the hook is staged; the test now passes and the site is killed. The Windows
job could not see this (`#[cfg(unix)]`), which is why the Linux suite run was required before calling
the head gateable.

### 3.3 Addendum — the two cross-platform type guards (`commands.rs:873`, `878`)

These are **not** `cfg`-gated; they are missed on Windows because no Windows test can materialise a
file symlink (SeCreateSymbolicLinkPrivilege), and the new killing tests are `#[cfg(unix)]`. Measured
on Linux in a separate probe: 7 selected (6 intended guard mutants — `with true`, `with false`,
`delete !` per line — plus one collateral); **6/6 intended caught, 0 missed**; the collateral
whole-function replacement is *unviable* (rustc 1.95 ICE in `check_mod_deathness`), not a kill. The
Windows run reports the two `true`/`false` guard mutants as missed, which is exactly this host limit.

## 4. The 13 assertion gaps, closed (MEASURED)

Every mutant named by wave 6 is now caught somewhere, and the killcheck evidence is at the committed
test head:

| Site (wave-6) | Test added | Killcheck evidence |
|---|---|---|
| `commands:1530` ×2, `1531` ×3 (`preview`) | `commands::tests::a_preview_keeps_exactly_the_limit_and_marks_only_what_it_cut` + the `stdout_preview` field asserted on a real run | 5/5 caught (`1535:2`, `1536:3`) |
| `commands:2116` (`+=`), `2124` (`>=`) `pending_intents` | `status_counts_an_intent_with_no_outcome_as_pending` | 3/3 caught (`2121:2`, `2124:1`) |
| `commands:2188` (`reconcile`, `>`→`>=`) | `a_duplicate_intent_that_is_fully_closed_is_not_reported_as_pending` | 3/3 caught on the line, `>=` included |
| `commands:2316` (`explain` filter) | `explain_lists_only_the_journal_records_of_the_explained_tree` | 1/1 caught |
| `commands:873` (symlink type guard) | `a_symlink_entry_checked_out_as_a_symlink_is_certified` (+ the existing `p0a_symlink_rewritten_as_a_file_is_refused`) | **caught on Linux** (addendum) |
| `commands:878` (regular-file type guard) | `a_hook_that_replaces_a_file_with_a_symlink_is_refused_with_its_own_reason` | **caught on Linux** (addendum) |
| `commands:868` (gitlink arm) | `a_tree_with_a_gitlink_is_refused_with_its_own_reason`; the arm is replaced by the pre-check of §5 | new guard `792: ==`→`!=` **caught** |
| `state:407` (journal `NotFound` arm) | `state::tests::an_unreadable_journal_is_an_error_not_an_empty_journal` | 4/4 caught (`guard→true`, `guard→false`, `==`→`!=`, whole-function collateral) |

The tests pin the field or message, not just the exit code: the pending counter is `1` for one open
intent and the journal is not rewritten; a fully closed duplicate id yields `RECONCILED` with an empty
`resolved`; `explain` never attributes another tree's record; the preview keeps exactly 400 characters
and marks only what it cut; the two type guards name the type mismatch instead of a later mode/content
mismatch; the journal arm errors with "cannot read journal", not "cannot rename".

Aggregate of the wave-7 Windows killchecks: **51 mutants selected, 45 caught, 2 missed (both the
Unix-symlink guards above, caught on Linux), 4 unviable** (whole-function replacements of
`cmd_publish`/`pending_intents`/`cmd_reconcile`/`cmd_explain`, which do not compile because `Json` has
no `Default`). Linux: **24 selected, 22 caught, 1 missed** (`state.rs:377`), **1 unviable** (rustc
ICE). The only survivor left in these modules is the fsync mutant, with its reason.

## 5. A refusal that named the symptom, not the cause (Fixed)

`verify_checkout_fidelity` ran the presence check before the type check. A tree naming a gitlink
therefore failed as *"the checkout is missing 1 of N tree entries"* — true of the checkout, but not
the cause: submodule content is never materialised by a checkout, so the entry can never be present.
The gitlink is now refused **first**, with its own message, and the now-unreachable `160000` match arm
is gone (one source of truth). The regression test asserts the gitlink reason, exit 1, `REFUSED`,
`UNSUPPORTED_CHECKOUT` and the absence of a receipt; its mutant (`==`→`!=`) is caught, and it is
caught because *every other* checkout test would refuse a tree with no gitlink (the faithful-checkout
control), not merely because the new test exists.

This is a message/ordering fix, not a policy change: a gitlink tree was refused before and is refused
now. It is listed under *Fixed* in the [CHANGELOG](../../CHANGELOG.md).

## 6. The three documented boundaries, closed (MEASURED)

| Boundary | Test | Killcheck |
|---|---|---|
| `commands:1082` — exactly 20 000 regular files | `commands::tests::a_checkout_with_exactly_the_entry_budget_is_compared_not_refused` | 3/3 caught (the mutants refuse at exactly 20 000) |
| `commands:1165` — exactly 64 MiB retained | `commands::tests::cat_file_batch_accepts_a_total_of_exactly_the_content_budget` | 5/5 caught on the line pair (`1164:2`, `1165:3`) |
| `commands:1592` — a 1 024-character ref | `a_ref_of_exactly_the_documented_limit_reaches_the_receipt_gate` | 5/5 caught |

The 20 000-entry test does not build a 20 000-file Git fixture: it calls the comparison with 20 000
references to one real blob, so the budget check runs and the comparison still reads and matches a
real checkout. That keeps the suite's runtime bounded (the whole test file still runs in seconds).

## 7. Tests, floor and coverage

- **13 new test functions** (9 cross-platform, 4 `#[cfg(unix)]`): 3 unit tests in `commands.rs`,
  1 unit test in `state.rs`, 3 operator-surface tests, 4 checkout-fidelity tests, 2 CLI tests, and one
  assertion added to the existing `run_passing_obligation_binds_the_index_tree`.
- **468 executed on Windows, 469 on Linux, 0 ignored**, both measured at `308bf6a` locally; the CI
  floor is raised to exactly those counts (`minimum=469`, Windows `468`). Wave 6 measured 459/456.
- **13 new named blocking gates**; the 4 Unix-only ones are not claimed on Windows (the workflow skips
  them there, as it already does for the other Unix-only gates), so a gate is never a false green.
- No test, gate or security property was removed or weakened; no waiver is used. The only product
  change is the gitlink refusal message/order (§5).
- The coverage gate is unchanged (**95 % lines / 90 % branches, product-only**) and is re-measured by
  the `coverage` job on this head; the new tests do not enter the product view.

## 8. What is still not run, and the limits

- **macOS**: not measured.
- **`cargo-fuzz`**: not added, by contract (zero dependencies, `--locked --offline`); the deterministic
  randomised sweep of wave 6 remains the substitute and is labelled as a sweep, never as fuzzing.
- **S9 confirmatory comparison**: still blocked by `H4`.
- **`state.rs:377`**: the only mutation survivor left in these modules, unobservable without a
  crash-consistency harness (§3.2).
- **Linux module rates**: this is a second measurement of the same 15 sites plus the two type guards,
  not a module-wide Linux mutation campaign; per-module Linux rates remain `NOT_RUN`.
- Product decisions unchanged: sandbox `H3`, receipt signing (`PRF-01`), retention (`PRF-02/08`),
  external-`git`-writer fence.

## 9. Reproduce

```bash
cargo test --locked --offline --all-targets
cargo clippy --locked --offline --all-targets -- -D warnings
cargo fmt --all --check

# Windows killcheck of the closed sites (worktree pinned at the committed test head)
pwsh -File experiments/quality/mutation-w7/killcheck.ps1 \
  -Repo <pinned-worktree> -Module commands \
  -Lines 873,878,1082,1165,1534,1536,1592,2121,2124,2188,2316 \
  -OutDir experiments/quality/mutation-w7
pwsh -File experiments/quality/mutation-w7/killcheck.ps1 \
  -Repo <pinned-worktree> -Module commands -Lines 792 -OutDir experiments/quality/mutation-w7
pwsh -File experiments/quality/mutation-w7/killcheck.ps1 \
  -Repo <pinned-worktree> -Module state -Lines 407 -OutDir experiments/quality/mutation-w7

# Linux measurement of the fifteen cfg(unix) sites and the two type guards: see
# experiments/quality/mutation-w7-linux/REPORT.md (§2 for the exact commands)
```
