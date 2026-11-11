# Quality wave 6 — the four remaining modules, and a randomised parser sweep

**Status: complete for what this host can prove.** Scope: `feat/flow-production-candidate`;
the mutation campaign runs on a dedicated worktree pinned at `aa59736` (the wave-5 head) and
this wave's own commits are on top of it. Host: Windows 11, Rust 1.95.0, `cargo-mutants 27.1.0`,
`cargo-llvm-cov 0.9.1` on `nightly-2026-05-06`. Nothing is merged, tagged or released, and
`Rwanbt/Ravum` is never written.

## 1. Why this wave exists

Two limits were left open by [wave 5](FLOW-QUALITY-WAVE5.md):

- four modules were still `NOT_RUN` — `state` (155 mutants), `gitcli` (104), `main` (73),
  `commands` (267). Wave 5 stopped before `state` rather than leave a partial module that could be
  mistaken for a result (§5 in that report);
- `cargo-fuzz` was `NOT_RUN`: a coverage-guided harness needs `libfuzzer-sys`, and the crate is
  deliberately zero-dependency with `--locked --offline` CI.

This wave closes the first and answers the second with the strongest thing the dependency
contract allows. It does not change product behaviour except where a test found a defect.

## 2. Campaign method (identical to waves 4–5, with one addition)

Same copy-mode rules: `--gitignore true`, `--jobs 2`, `--timeout 240`, no `--in-place`, and no
shared `CARGO_TARGET_DIR` (the script removes it; a shared one once made two concurrent mutants
link the same `flow-*.exe`, so a suite ran against the wrong binary — a contaminated run is not a
result).

The addition is provenance: the campaign runs from a **worktree pinned at a commit** instead of the
working tree, so the mutated sources are exactly a tree that can be re-checked out, and edits made
during the campaign cannot leak into a copy.

| module | suites selected (scope = lower bound, never a whole-crate score) |
|---|---|
| `state` | `bins`, `cli`, `error_paths`, `operator_surfaces`, `crash_run` |
| `gitcli` | `bins`, `isolation`, `bounds`, `error_paths` |
| `main` | `bins`, `cli`, `operator_surfaces` |
| `commands` | `bins`, `cli`, `git_fidelity`, `operator_surfaces` |

## 3. Campaign results

Two of the four runs were interrupted by the host: `state` stopped after 42 of its 155
mutants and `gitcli` after 93 of 104, with no summary line, no error from cargo-mutants,
and the debug log ending inside a `Test` phase (a build directory was left behind). A
partial module is not a result, so both were re-run with `--iterate` (which skips mutants
already caught or unviable) and one job instead of two. The table merges the two passes;
the merged totals are arithmetic over the tool's own counts, not a second classification.
On this host a `#[cfg(unix)]` or `#[cfg(not(windows))]` mutant is *not compiled*: it
builds and the suite passes, so it is reported as missed. Those lines are classified in
§5; none of them is counted as killed.

| module | mutants | unviable | applicable | caught | missed | timeout | strict |
|---|---|---|---|---|---|---|---|
| `state` | 155 | 12 | 143 | 125 | 18 | 0 | 87.4 % |
| `gitcli` | 104 | 18 | 86 | 62 | 24 | 0 | 72.1 % |
| `main` | 73 | 0 | 73 | 67 | 2 | 4 | 91.8 % |
| `commands` | 267 | 20 | 247 | 208 | 39 | 0 | 84.2 % |
| **all four** | **599** | **50** | **549** | **462** | **83** | **4** | **84.2 %** |

The four `main` timeouts are one defect: `index += 1`/`index += 2` replaced by `*=`, which
never advances the flag walk, so the process loops until the per-mutant timeout. A
watchdog test now turns that from a hung suite into a failure (§4). Every other non-caught
outcome is a `missed`, and the dominant cause is scope: the campaign suites are a subset,
so a line exercised only by `proposals`, `git_fidelity`, `bounds`, `concurrency` or
`secrets` is invisible to the run that picks the mutants. That is why §4 re-tests the
reported sites with the whole suite list instead of claiming the campaign rate.

## 4. Killchecks: the reported sites re-run with the whole suite

A survivor is a statement about the *selected scope*, not about the code: a suite that was not
selected may exercise the same line. Re-running each module with every suite costs hours, so a
killcheck re-tests only the reported sites with `cargo mutants --in-diff`:

- a probe comment is appended to each survivor line and `git diff -U0` produces a diff with no
  context, so the selection is the reported lines (cargo-mutants expands a changed line to its
  enclosing scope, so a site re-tests its function rather than one operator);
- the probe stays in the tree for the whole run: cargo-mutants checks that the diff's post-image
  matches the source it is about to mutate, and it copies the workspace per mutant;
- the tree is a committed throwaway worktree whose HEAD already contains the assertions that
  close the sites, so an uncommitted change cannot leak into a copy;
- the suite list is wider than the campaign's (`cli`, `proposals`, `git_fidelity`,
  `operator_surfaces`, `error_paths`, `isolation`, `bounds`, plus `crash_run` for `gitcli`), and
  `--bins` is always included so the unit tests that pin the pure helpers run.

| module | reported | caught by a killcheck | residual |
|---|---|---|---|
| `state` | 18 | 10 (rounds 1–2) | 8 |
| `gitcli` | 24 | 14 (rounds 1–2) | 10 |
| `main` | 6 | 6 (rounds 1–2) | 0 |
| `commands` | 39 | 15 (rounds 1 and 3) | 24 |
| **all four** | **87** | **45** | **42** |

Two findings are worth recording because they change what the campaign numbers mean:

- **`gitcli:515` was a false survivor.** The campaign recorded `resolve_program -> None` as missed,
  yet the existing `cli` test (`a_cmd_shim_on_path_is_found_by_a_bare_program_name`) asserts that a
  `.cmd` shim on `PATH` runs, and a dependency-free hand reproduction
  (`experiments/quality/mutation-w6/probe-515.ps1`, two `rustc` builds and a real repository) shows
  the mutant turns that obligation into `UNSUPPORTED`. The targeted run catches it. cargo-mutants 27
  does not retain a per-mutant test log in the artefact, so *why* the campaign run missed it cannot
  be read from the evidence; the site is reported as caught by the narrower run, and the campaign
  number is left as recorded.
- **The four `main` timeouts are caught once they are not paid for at full-suite cost.** They hang
  the whole `cargo test` invocation, so the per-mutant budget expires before the failing test is
  reported. Re-run with a libtest filter on the watchdog test, each is caught in about ten seconds.

- **The new containment test found a real product defect on Linux.** `a_timed_out_obligation_kills_the_descendants_it_started`
  failed on the Ubuntu job (run `42950415768`): `kill_tree`'s POSIX branch passed `-<pgid>` to `sh`'s
  `kill`, and dash parses that as an option ("Illegal option -9"), so the group was never signalled
  and only the direct child died. The fix passes the operand after `--` (verified with dash directly:
  without `--` the kill fails, with it the group dies), and the CHANGELOG records it under *Fixed*.
  This is why the wave exists: the test is worth more than the number it adds.

After the killchecks the scoped rates are 94.4 % (`state`), 88.4 % (`gitcli`), 100 % (`main`) and
90.3 % (`commands`) — still **lower bounds for each module**, because the 42 residual sites below
are not all equivalent: some are host-gated, some are boundaries the fixtures do not reach, and some
are simply not asserted by any test.

## 5. What is still not caught, each classified

The 42 residual sites fall into four classes. A site is only called *equivalent* when a proof is
given; otherwise it is listed as a **gap** with the assertion that would close it.

**Not compiled on this host (15).** `#[cfg(unix)]` / `#[cfg(not(windows))]` code: `state` 340 ×4
and 377 (the Unix hardlink and directory-fsync arms), `gitcli` 563 (the non-Windows
`resolve_program` stub), `commands` 996, 1000 ×3, 1006 ×3, 1007 ×2 (the executable-bit proof). The
mutated line is not built on Windows, so the mutant is "missed" by construction. They are not
counted as killed, and Linux mutation remains `NOT_RUN` (`H4`).

**Host-equivalent, with the proof (11).** `state:323` (`is_symlink() || reparse`): on this host a
directory link reports both bits true (junction probe), so `&&` refuses exactly the same paths; the
`||` is still required on a host where a reparse point is not a symlink. `gitcli:134` and `632`
(`Instant::now() + READER_GRACE` → `-`): with a stream still held open by a survivor the reader
cannot send before EOF, so both variants capture the same (empty) value — only the two-second wait
differs, and no test asserts latency. `gitcli:615` (`elapsed() > deadline` → `>=`): a nanosecond
boundary that no scheduler reaches deterministically. `gitcli:658` ×2 and `659` ×3 (`reap_bounded`):
a zombie is a Unix concept; on Windows the child is killed and the handle is closed, so not reaping
has no observable effect. `gitcli:153` (`exit.unwrap_or(-1)`): `None` requires a process killed by a
signal, which Windows does not report. `state:680:48` (`doe - doe / 146_096` → `/ doe /`): differs
only on the single day of a 146 097-day era where `doe / 146_096 == 1`.

**Boundaries the fixtures do not reach (3).** `commands:1077` (exactly 20 000 regular files),
`commands:1160` (exactly the 64 MiB content budget) and `commands:1587` (a 1 024-character ref)
differ only at the documented bound; building those fixtures costs 20 000 files, a 64 MiB blob and a
publication to a 1 024-character branch. Closing them is cheap in principle and is left as a
follow-up rather than claimed.

**Gaps: no test asserts the field (13).** `commands:1530` ×2 and `1531` ×3 (`preview`, which feeds
the receipt's log preview), `2116`, `2119`, `2183`, `2311` (pending-intent counting, reconcile's
`>`, explain's receipt filter), `863` and `873` (checkout type guards whose refusal still happens
through the byte comparison, with a different message), `868` (a gitlink in a *checkout* tree: the
refusal is pinned at admission, not at checkout), and `state:407` (the journal's `NotFound` arm:
only the message differs). These are real coverage gaps in the operator surfaces, not equivalent
mutants; each would need an assertion on the field or message it changes.

`gitcli:515` is not in this list: it is caught (§4). The full names are in
`experiments/quality/mutation-w6/merge_killchecks_w6.py`'s output.

## 6. A randomised parser sweep in place of `cargo-fuzz`

`cargo-fuzz` cannot be added without breaking the zero-dependency, `--locked --offline` contract,
so the parser boundary gets the strongest dependency-free equivalent: a deterministic randomised
sweep that runs on every build.

- A 6-line `xorshift64*` generator is defined in each test module (no dependency, fixed seed, so a
  failure is reproducible from the seed alone).
- Inputs are mixed adversarial bytes (`NUL`, tab, newline, `/`, `\`, `:`, `0xff`) and fully random
  bytes, plus deliberately well-formed shapes so both the accepted and the refused answer is
  exercised (the tests assert that each happens more than 100 times, so a vacuous sweep fails).
- `parse_tree_entries`, `parse_tree_dirs`, `parse_cat_file_batch` (2000 shapes each),
  `parse_tree_listing`, `parse_raw_diff` (2000 each), `ident` (4000) and
  `symlink_target_escapes` (4000) are covered. Invariants: never panics; every accepted value obeys
  its documented rule (paths are relative and safe, separators are normalised, identities are
  trimmed/bounded/charset-checked, an accepted batch returns one blob per requested id); the
  symlink rule is separator-symmetric and a safe target stays safe under a safe suffix.
- This is **not** coverage-guided fuzzing and is never presented as such: no corpus, no mutation
  guidance, no crash triage. It is a randomised sweep on a fixed schedule.

## 7. What is still not run

- **Linux mutation**: still a Windows-host measurement. A 2-core runner would need its own budget
  decision (`H4`), and five `state` mutants plus one `gitcli` mutant are `#[cfg(unix)]` code that
  this host does not compile at all (they are listed as *not compiled here*, never as killed).
- **macOS**: not measured.
- **`cargo-fuzz`**: not added, by contract; §6 is the substitute and is labelled as a sweep.
- **S9 confirmatory comparison**: still blocked by `H4`.
- Product decisions unchanged: sandbox `H3`, receipt signing (`PRF-01`), retention (`PRF-02/08`),
  external-`git`-writer fence.

## 8. Tests, floor and coverage

- **25 new test functions**, executed on both platforms (no `#[ignore]`, no skip that hides a
  shape). Twenty are unit tests inside the modules whose helpers they pin; five are integration
  tests (`isolation.rs`: the 8 KiB stream that must not be flagged truncated, the timed-out
  obligation whose descendant must not survive, the porcelain dirty/untracked split; `cli.rs`: the
  status counts and the case-collision refusal that must name the collision).
- The suite executes **459 tests on Windows and 456 on Linux, 0 ignored** (422/419 at the wave-5
  head) — both numbers measured in run `42951607307`. The CI floor is set to exactly those counts, so
  deleting a test fails the build even if no named gate is touched.
- **11 new named blocking gates** pin the new tests by name, so deleting or renaming one fails the
  build exactly like a wave-5 gate.
- The coverage gate is unchanged at **95 % lines / 90 % branches product-only** and the `coverage` job
  re-measured it on this head: **95.68 % lines (4389/4587), 92.84 % branches (791/852)** — both above
  the gate, with no waiver. None of the new tests is excluded from the product view (test code never
  enters it).
- CI on the final head (`4fb170c`): push run `42951607307` and PR run `42951614434`, **5/5 jobs each**
  (test Ubuntu + Windows, polyglot ×2, coverage). The two runs of the previous head failed on real
  defects — a gate name without its `tests::` prefix and the POSIX containment bug — not on flakes.
- The count is not the point: five of the new tests exist because a mutant survived without them
  (the retained-stream budget collapse, the straddling last chunk, the interrupt-only retry, the
  hyphenated-token tail, the non-advancing flag walk), and two more because a refusal named the
  wrong cause (the case collision, a single obligation selected once).

## 9. Reproduce

```bash
cargo test --locked --offline --all-targets
cargo clippy --locked --offline --all-targets -- -D warnings
cargo fmt --all --check

# mutation campaign (wave 6), from a worktree pinned at aa59736
pwsh -File experiments/quality/mutation-w6/run-mutations.ps1
python experiments/quality/mutation-w6/summarize.py
python experiments/quality/mutation-w6/merge_killchecks.py

# targeted killcheck for one module and its reported lines
pwsh -File experiments/quality/mutation-w6/killcheck.ps1 \
  -Repo <pinned-worktree> -Module gitcli -Lines 134,153,176,201,206
```
