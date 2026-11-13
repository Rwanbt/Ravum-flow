# FLOW QUALITY WAVE 3 — error-path coverage, honest measurement, bounded mutation

Status: **reviewed self-report, waiting for Lead/CI ratification.** No source code under
`crates/flow/src/**` was changed by this wave. One new test file and measurement artefacts were
added. Classification: `MEASURED` = executed on this host, command recorded; `CODE_READ` = read
from source, not executed; `NOT_RUN` = not attempted/blocked. Cross-links: baseline summary
[`summary-dfb941d-wave3.txt`](../../experiments/quality/coverage/summary-dfb941d-wave3.txt),
after summary [`summary-5077c50-wave3.txt`](../../experiments/quality/coverage/summary-5077c50-wave3.txt),
mutation [`experiments/quality/mutation-dfb941d-proposal/`](../../experiments/quality/mutation-dfb941d-proposal/).

## 1. Trees measured (MEASURED)

| Role | Commit | Working tree | Tests |
|---|---|---|---|
| baseline | `dfb941d` — checkout-fidelity fix (Lead patch + `git_fidelity.rs` committed) | clean, measured in a detached `git worktree` | 173 passed / 0 failed / 0 ignored |
| after | `5077c50` — executable-bit fidelity fix | `tests/error_paths.rs` new and untracked at run time; teammates' `crash_run.rs`, `secrets.rs` and test additions committed | 225 passed / 0 failed / 0 ignored |
| control (same tree, no new suite) | `5077c50` | same tree, `error_paths` binary explicitly excluded | 193 passed |

Drift after measurement: HEAD is `ccdbec9` (`test: cover the refusal paths …`), which commits
`crates/flow/tests/error_paths.rs` unchanged (49 885 bytes), plus uncommitted `commands.rs` work by
another agent (post-run checkout re-verification). **The coverage numbers below describe the
`5077c50` product source**; the in-flight `commands.rs` change was not measured (it was mid-edit,
and measuring it would be dishonest). A final gate run must be repeated on the release SHA.

## 2. Method and two measurement defects found (MEASURED)

Commands (power-shell syntax elided; the exact lines are in both summary headers):

```
CARGO_TARGET_DIR=…\wt-quality-target  CARGO_BUILD_JOBS=4
cargo +nightly llvm-cov clean --workspace
cargo +nightly llvm-cov --locked --offline --all-targets --branch \
    --ignore-filename-regex '[\\/]tests[\\/]' --no-report
cargo +nightly llvm-cov report --branch --ignore-filename-regex '[\\/]tests[\\/]'   # table
cargo +nightly llvm-cov report … --lcov/--json/--text --output-path …              # artefacts
```

1. **The ignore filter also discards `#[path]`-imported source modules.** A test file importing
   `#[path = "../src/scratch.rs"] mod scratch;` records code under
   `crates\flow\tests\..\src\scratch.rs`; `--ignore-filename-regex '[\\/]tests[\\/]'` excludes it.
   Everything `tests/isolation.rs` exercises in `scratch.rs`, and everything `tests/bounds.rs`
   exercises in `gitcli.rs`, is invisible to the gate metric. `scratch.rs`'s true exercised
   coverage is therefore higher than the 71–76 % reported; the gate number is structurally capped.
   (Proof: `isolation`-only run leaves `refusal_message` at count 0 although its test passes, while
   the JSON export contains a second function instantiation attributed to `tests\..\src\scratch.rs`.)
2. **Profile data accumulates across runs.** `*.profraw` files are not deleted by a plain re-run
   (measured 455 → 769 after one extra run), and `report` merges all of them. Stale profiles from an
   older binary/test set can hide or invent counts: two identical runs differed by 9 lines / 2 branch
   points until a `cargo llvm-cov clean --workspace` was added. Every number quoted here starts from
   a clean target/profile state.

Run-to-run variance after cleaning: ±1 branch counter on identical runs (`lock.rs` 68.42 vs 69.23
locally); this is quoted rather than rounded away.

## 3. Coverage results and the two targets that are not met

| Scope | Before (`dfb941d`) | After (`5077c50` + tests) | Delta | Target | Met |
|---|---|---|---|---|---|
| **global lines** | 83.27 % | **88.75 %** | +5.48 | ≥ 88 % | **yes** |
| **global branches** | 69.11 % | **75.62 %** | +6.51 | ≥ 78 % | **no** |
| `proposal.rs` lines | 74.94 | **90.91** | +15.97 | ≥ 85 | **yes** |
| `proposal.rs` branches | 55.48 | **80.14** | +14.66 | ≥ 80 | **yes** |
| `scratch.rs` lines | 71.04 | **75.76** | +4.72 | ≥ 85 | **no** |
| `scratch.rs` branches | 55.56 | **61.11** | +5.55 | ≥ 80 | **no** |
| `commands.rs` | 82.16 / 69.97 | 87.02 / 73.48 | +4.86 / +3.51 | — | — |
| `main.rs` | 75.00 / 80.43 | 93.75 / 82.61 | +18.75 / +2.18 | — | — |
| `state.rs` | 92.35 / 66.07 | 92.56 / 66.07 | +0.21 / 0 | — | — |
| `gitcli.rs` | 90.59 / 77.14 | 91.35 / 80.00 | +0.76 / +2.86 | — | — |

The full per-module table, regions/functions and denominators are in the summaries. The
same-tree control (same commit, same commands, only the new test binary excluded) measures
83.33 / 69.23, so the wave's tests alone account for +5.42 points of global line coverage.

### 3.1 Why `scratch.rs` cannot honestly reach 85/80 under the prescribed command

Of 67 uncovered lines, only a handful are reachable from the shipped binary; the rest require
calling the module directly, which the filter hides (§2.1). Exact uncovered regions
(`lcov-5077c50-wave3.info`):

- `84,87,91` — `Drop::still_owned` false paths (path deleted / replaced by a link / replaced by a
  non-directory between materialisation and drop). No external process can intervene in that window
  through the CLI; `isolation.rs` covers the unit-level `remove_tree_bounded` instead.
- `135-137` non-hex commit — `flow run` only passes object IDs returned by `commit-tree`.
- `155` candidate path inside the repository (the *base* check at 143-145 is covered).
- `163-167,172-175` `create_dir` failure / 128-attempt exhaustion — needs a failure injected into the
  filesystem, not reachable deterministically.
- `181-182,281-303` `verify_created_dir` refusals (symlink, non-directory, redirect identity).
- `186-190` non-UTF-8 scratch path; `203-205` `git worktree add` error; `219-223` marker write
  failure.
- `260` `canonical_dir` on a non-directory (only reachable through `materialise_in`).
- `320,323-326,329-332` `.git` link cannot be inspected / is a symlink / is a directory. The
  *missing* `.git` variant (314-318) **is** covered through a tracked `post-checkout` hook that
  removes the link (`run_refuses_a_scratch_whose_git_link_a_hook_removed`, MEASURED).
- `336-337` read-only-file retry (Windows-only fault), `351` prune on a vanished repo root,
  `376,393,405,408,412-414` cleanup deadline/entry budget/read-only cleanup branches.

Two CLI-reachable scratch paths were added and are covered: a host temp directory redirected into
the repository (`TMPDIR`/`TMP`/`TEMP`) is refused as `SCRATCH_REFUSED` with an empty base and no
receipt/worktree residue; and the missing-`.git` hook above. Reaching the rest through the binary
would require manufacturing internal failures, i.e. padding.

## 4. Tests added (`crates/flow/tests/error_paths.rs`, MEASURED)

28 own test functions; the binary reports 32 because the file imports `src/json.rs` for parsing and
its 4 unit tests re-run (same precedent as `bounds.rs` re-running `gitcli` unit tests). 0 ignored,
0 existing tests weakened or deleted, no threshold changed, no mock replaces product code. All tests
drive the shipped binary (`CARGO_BIN_EXE_flow`) with real Git repositories under the system temp
directory, cleaned on drop; the only shell fixture is a Git `post-checkout` hook, which exists on
Linux and on Git-for-Windows. Special file types are produced as **index-only** patches
(`git apply --cached`), so the tests do not need symlink privileges and a `160000` gitlink entry can
be refused on Windows too. Every assertion is an observable contract: refusal code, exit status,
record absent/unchanged, file/worktree preserved.

| Group | Tests | Contract asserted |
|---|---|---|
| proposals read side | 3 | listing is ordered and ignores non-records; a corrupt record is an error, not hidden; unknown/malformed ids are distinct codes |
| admission refusals | 9 | invalid identity (agent/task/key, 7 shapes) records nothing; base not a commit; unreadable patch/directory; 32 MiB+1 sparse patch refused *before reading*; escaping symlink (4 target shapes) refused, safe symlink admitted; 4097-byte symlink target; case collision; gitlink; 1 MiB patch admitted (bound pinned from below) |
| idempotency | 1 | same bytes without `--key` → same id + `idempotent:true` |
| assembly refusals | 8 | no proposal; delete+modify collision names both; differing modes; vanished recorded patch (no candidate); candidate schema mismatch not reinterpreted and not rewritten; candidate immutability preserves tampered record byte-for-byte; corrupt patch classifies `PATH_CONFLICT` / `APPLY_REFUSED` with no candidate; case-colliding candidate diagnosed |
| usage | 2 | usage/version/help/unknown command/flag errors are exit 2 and touch nothing; repository commands fail outside a repository |
| scratch | 2 | `TMPDIR` redirected into the repo refused, base empty, no receipt, no worktree registration; hook-removed `.git` link → `SCRATCH_REFUSED`, clean repo |
| publish receipts | 3 | 13 single-field tamper cases each hit their named check (schema, integrity, tree name, scope, missing required ×2, policy digest/version/boundary, context, obligation status, receipt status, commit tree) with the branch unmoved; second publication of the same tree → `ALREADY_PUBLISHED`; non-branch/invalid/unknown refs refused |

## 5. Mutation on `proposal.rs` (bounded; killed by order; MEASURED)

Command: `cargo mutants --file crates/flow/src/proposal.rs --timeout 120 --jobs 2 --no-shuffle
-o experiments/quality/mutation-dfb941d-proposal` (CARGO_TARGET_DIR in-repo at that moment).

| Outcome | Count |
|---|---|
| mutants found | **119** |
| tested before the 45-minute cap was reached (stopped at ~14 min on Lead instruction) | **13** |
| caught | 8 |
| missed | 2 |
| timeout | 0 |
| unviable (do not compile) | 3 |
| not run | 106 |

Baseline: 4 s build + 62 s test; viable mutants take 1 s build + 60–75 s test **each** because every
mutant runs the whole 225-test suite (hundreds of spawned `flow` processes). `--jobs 2` therefore
cannot reach 119 mutants within 45 minutes; a first attempt was also invalidated by a transient probe
file created mid-run (113 unviable) and its data was discarded. `cargo-mutants` copies the source
tree per run; in-repo `CARGO_TARGET_DIR` directories excluded only through `.git/info/exclude` are
copied too (Gigabytes; one retry exhausted the C: volume), so mutation must use an external or
properly git-ignored target dir.

Survivor classification (the only two):

| Mutant | Class | Action |
|---|---|---|
| `proposal.rs:38:39` `MAX_PATCH_BYTES 32*1024*1024` → `32*1024+1024` | **REAL GAP** — no test pinned the admission bound from below, so a 33 KiB limit passed | killed by `propose_admits_a_patch_between_one_mebibyte_and_the_declared_bound`; **verified** |
| `proposal.rs:38:46` same replacement | **REAL GAP** — same | same test; **verified** |

Verification: a targeted rerun of exactly these two mutants on the final suite
(`-F 'replace \* with \+'`, separate small worktree, external target dir) reports
**2 mutants tested in 3 min: 2 caught, 0 missed**
([`mutation-fc80af3-killcheck/`](../../experiments/quality/mutation-fc80af3-killcheck/)).
The other unviable/genre classes seen in the 13: 3 × `replace fn -> Default::default()` /
`Ok(Some(Default::default()))` (no `Default` for `Json`/`Proposal` — *not compiled*, intrinsic, not a
host issue); 8 × operator/guard mutations caught. 106 mutants are `NOT_RUN`; the partial table is
reported with its denominator instead of extrapolating a kill percentage.

Recommendation for the next wave: mutation on `proposal.rs` only makes sense with a scoped test
filter (e.g. `--test proposals --test error_paths` plus unit tests) or sharded runs; the
whole-suite-per-mutant cost is the limiting factor, not the mutant count.

## 6. Deliberately not tested (and why)

- `EMPTY_PROPOSAL` (`proposal.rs:669-673`): an empty patch is rejected by `git apply` first
  ("No valid patches in input", exit 128) → observable code is `PATCH_REFUSED`; the documented
  empty-proposal contract is unreachable through `flow propose` (MEASURED probe, CODE_READ). Either
  the apply call needs `--allow-empty` or the refusal is dead; not something a test can honestly
  assert today.
- `.git/` target refusal in `validate_path` (`proposal.rs:423-426`): `git apply --cached "…/.git/x"`
  refuses with `invalid path` before admission sees it (MEASURED probe) — defence in depth, unreachable
  from the CLI.
- `SYMLINK_UNREADABLE` blob read (`proposal.rs:446-456`): the blob was written by `write-tree` from
  the same index seconds earlier; unreadable here would require corrupting the object store mid-call.
- `parse_raw_diff`/`diff_changes`/`validate_tree` unreadable or unparsable Git output
  (`proposal.rs:363,372,399-404,488-515`): Git's own machine output; fault injection would need a
  fake `git`, i.e. a mock replacing real behaviour.
- seed/write-tree/atomic-write error paths (`proposal.rs:639-663,716,841-876`): internal failures
  after a successful `git` call; only reachable by replacing Git or the filesystem.
- `scratch.rs` internals listed in §3.1: direct-module tests exist in `tests/isolation.rs` for some of
  them; adding more would not move the prescribed metric (the import is filtered out) and would
  duplicate existing contracts.
- Candidate-tree-pruned (`CANDIDATE_TREE_MISSING`), candidate/base mismatch on publish, doctor,
  lock, journal and reconcile branches: reachable but outside the two target modules; the wave's time
  budget went to `proposal.rs`, which had the weakest branch coverage. They remain uncovered and are
  visible in `lcov-5077c50-wave3.info`.
- Windows-only cleanup faults (`scratch.rs:336-337,412-414`): would need a Windows-only fixture; kept
  out to keep the suite cross-platform.

## 7. Behaviour that looks wrong (suspect defects)

### D1 — `--dry-run` (and every boolean flag) is accepted and ignored by commands that do not implement it (MEASURED)

`main.rs::validate_flags` accepts any `BOOL_FLAGS` entry for every command, while `--dry-run` is
documented only for `publish` and read only by `cmd_publish`. Minimal reproduction:

```
git init -q repo && cd repo && git commit -q --allow-empty -m base
flow init
# write .flow/config.json with one required green obligation, then:
flow approve
flow run --dry-run          # exit 0, executes the obligation, writes .flow/receipts/<tree>.json
```

Measured: exit 0, receipt written (`42d4c52….json`), i.e. a request for a plan performed the
verification and mutated proof state. The same shape applies to `flow propose --force` (accepted,
ignored; the usage error that follows is about missing proposal flags, not the unknown boolean).
This contradicts the stated reasoning in `validate_flags` ("silently ignoring `--dry-run` on a
program that does not know it would publish when the operator asked for a plan") and the usage text,
which lists `--dry-run` under `publish` only. Suggested fix (not applied — outside this wave's write
scope): per-command boolean sets, or reject any boolean flag not declared for the command.
Severity: low-to-medium (no file/ref mutation, but it runs verifier programs and writes a receipt a
later `publish` would accept).

### D2 — coverage filter blind spot (MEASURED, §2.1)

Not a product defect; it makes the gate under-report `scratch.rs`/`gitcli.rs` and hides real
isolation-test coverage. Fixing the gate means changing the ignore expression or teaching
cargo-llvm-cov about the imported copies; that is a measurement decision for the Lead, deliberately
not changed here (threshold changes are out of scope).

### D3 — profile accumulation / non-reproducible reports (MEASURED, §2.2)

Also measurement hygiene, not product code; requires `clean` before every gate run.

### D4 — unreachable `EMPTY_PROPOSAL` and `.git`-path refusals (MEASURED probes, CODE_READ)

Both refusal branches are unreachable through the intended CLI path because `git apply` refuses
earlier. Either redundant defence (fine, but then the lines cannot be covered) or a missing
`--allow-empty` in the apply invocation. Reported rather than worked around.

## 8. Artefacts

- `experiments/quality/coverage/summary-dfb941d-wave3.txt` — clean baseline, commands, table, caveats.
- `experiments/quality/coverage/summary-5077c50-wave3.txt` — clean after, commands, table, same-tree control.
- `experiments/quality/coverage/{lcov,coverage,text}-{dfb941d,5077c50}-wave3.*` — raw data for both measurements.
- `experiments/quality/coverage/run-dfb941d-baseline.log`, `run-5077c50-after.log`,
  `run-5077c50-without-errorpaths.log` — instrumented test runs.
- `experiments/quality/mutation-dfb941d-proposal/` — partial bounded run (partial `caught.txt`,
  `missed.txt`, `unviable.txt`, `outcomes.json`, per-mutant logs/diffs).
- `experiments/quality/mutation-fc80af3-killcheck/` — 2/2 targeted verification of the survivors.
- `crates/flow/tests/error_paths.rs` — the new suite (committed by the Lead as `ccdbec9`).

## 9. Cross-links

- Gate measurements: [`FLOW-PRODUCTION-GATE-REPORT.md`](FLOW-PRODUCTION-GATE-REPORT.md) §6.
- Test inventory/matrix: [`FLOW-PRODUCTION-TEST-MATRIX.md`](FLOW-PRODUCTION-TEST-MATRIX.md).
- The checkout-fidelity contract this wave's scratch tests touch:
  [`git_fidelity.rs`](../../crates/flow/tests/git_fidelity.rs).
- Related wave-3 reviews by sibling agents (kept separate, not merged):
  [`FLOW-P0-GIT-FIDELITY-REVIEW.md`](FLOW-P0-GIT-FIDELITY-REVIEW.md),
  [`FLOW-PRODUCTION-ADVERSARIAL-WAVE3.md`](FLOW-PRODUCTION-ADVERSARIAL-WAVE3.md),
  [`FLOW-PACKAGING-WAVE3.md`](FLOW-PACKAGING-WAVE3.md).
