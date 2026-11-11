# Flow — Quality wave 8: the module rates on Linux, and the last survivors

Status: **measurement + five test/refactor changes.** No gate was waived, no test removed, no
security property weakened, no product behaviour changed (the only product edit is an extraction with
identical semantics, §2). Nothing was merged, tagged or released; `Rwanbt/Ravum` was not touched.

## 1. Mandate and scope

Wave 7 closed the last assertion gaps and measured the 15 Unix-only mutation sites it knew about, and
left three things open that an agent with this host can still close:

- **per-module mutation rates on Linux were `NOT_RUN`** (wave 6 measured four modules on Windows,
  waves 4–5 the other five; no Linux module rate existed);
- **one survivor in those modules**: `state.rs:377` (the directory `fsync` after an atomic rename),
  reported as unobservable in-process and never as equivalent;
- **macOS, `cargo-fuzz`, S9/H4 and the product decisions** — still blocked by the host, the
  zero-dependency contract, or a human decision, and unchanged in §8.

Wave 8 runs the four wave-6 modules on Linux with the same scoped suites (so the two platforms are
comparable), adjudicates every survivor with a wider-suite rerun, kills the survivor above, and closes
the four real test gaps the campaigns found. It also records a Windows stability campaign that failed
once, with its cause, instead of hiding it (§6).

## 2. The last survivor: `state.rs:377` (now `parent_to_flush`, line 396) — killed

The wave-7 classification was right: in-process, `sync_all` reports nothing to the caller, no fsync
failure is injected, and a crash harness would be needed to observe the missing directory flush. Wave 8
does not fake that observation. It extracts the *rule* the mutant changes into one named place and pins
it directly:

```rust
// before: if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty())
// after:  if let Some(parent) = parent_to_flush(path) { … }
fn parent_to_flush(path: &Path) -> Option<&Path> {
    path.parent().filter(|p| !p.as_os_str().is_empty())
}
```

The mutant (`delete !`) turns the filter into `filter(|p| p.as_os_str().is_empty())`, so a bare
relative name yields `Some("")` instead of `None`; the write would then fail on `File::open("")`. The
unit test
`state::tests::only_a_real_parent_directory_is_flushed_after_the_rename` asserts the parent of
`.flow/journal.jsonl`, the parent of an absolute journal path, and `None` for a bare name and for `/`.

Killcheck on Linux at `90640b8` (`--in-diff` probe of line 396, jobs 1, scoped suite, baseline 6 s +
14 s): **3 selected — 2 caught, 0 missed, 1 unviable**. The caught set includes the intended
`396:30 delete !` and the whole-function `396:5 parent_to_flush -> None`; the unviable one is the
`Some(Default)` replacement. What is now proved is the *rule*, not the crash behaviour: the durability
claim for a power cut remains measured only by construction and is still labelled that way.

The gate is Unix-only (`#[cfg(not(windows))]`), so CI now skips it on Windows instead of claiming a
false green, and the Linux floor moves from 469 to 470.

## 3. The two assertion gaps the state campaign found — killed

| Site (campaign head `9bbb02d`) | Mutant | Why the suite passed | Test added (assertions inside existing tests) |
|---|---|---|---|
| `state.rs:390:41` | `==`→`!=` in the journal schema stamp | no test appends a **schema-less object**, so "no record loses its stamp" was never asserted | the record must be stamped `SCHEMA_JOURNAL`, and each stored line has exactly one `"schema"` key |
| `state.rs:661:22` | `/`→`%` in `iso8601_now`'s day count | `days = secs % 86_400` gives years 1970–2206 and the test only asserted `year >= 2024`, so it caught the mutant only before ~05:29 UTC | the date is recomputed from the same `unix_now()` with `civil_from_days(secs / 86_400)` and compared, so the clock alone cannot make it pass |

Killcheck on Linux at `3b1e33e` (`--in-diff` on both lines, jobs 1, full suite): **7 selected —
7 caught, 0 missed, 0 unviable** — the two intended mutants plus five enclosing-function collateral
mutants. Test count is unchanged (assertions live in existing tests), so the floor stays 470/468.

## 4. The two gaps the commands and gitcli campaigns found — killed

| Site | Mutant class | Test added |
|---|---|---|
| `commands.rs:357` | `&&`→`\|\|` / `!=`→`==` in `cmd_doctor`'s `longpaths` check | the missing-directory doctor test now sets `core.longpaths=false` (unset would inherit the machine's global config) and asserts `WARN` on Windows, `OK` elsewhere |
| `gitcli.rs:354` | `allowed_env -> vec![]` / `-> vec![(Default, Default)]` | the allowlist exit test now also proves the positive half: `PATH` (allowlisted, always present) survives the filter, and no kept name is off the allowlist |

Both campaigns found these as *real* gaps, not host artefacts:

- **`commands:357`**: the only `longpaths` assertion used a repository with `core.longpaths=true`, so
  the short-circuit was never exercised; on Linux the `&&`→`||` mutant turned the check into a WARN
  and nothing noticed.
- **`gitcli:354`**: the secrets test proves an unlisted variable is not inherited, which an empty
  allowlist also satisfies; the positive direction was untested.

Killchecks at the head that carries the tests (`f5a19de`, Windows / `09c0b41`, Linux):

- Windows, `--in-diff` on `commands.rs:357`, jobs 1, baseline ok: **3 selected — 2 caught, 0 missed,
  1 unviable**. Both mutants are caught there: the existing `core.longpaths=true` test kills the
  `||` shape and the new WARN assertion kills the `==` shape; the unviable mutant is the whole-function
  `cmd_doctor -> Ok(Default::default())`.
- Linux, `--in-diff` on the two lines, jobs 1, one run after the other: **see §4.1**.

### 4.1 Linux killcheck result at `09c0b41`

- **`commands.rs:357`** (`--in-diff` on the line, jobs 1, scoped suites): **3 selected — 1 caught,
  1 missed, 1 unviable**. The caught mutant is `357:26 &&`→`||`, killed by the new non-Windows `OK`
  assertion. The missed one is `357:39 !=`→`==`: on Linux `cfg!(windows)` is the literal `false`, so
  the right-hand side is unreachable and the two programs are identical — that site is **caught on
  Windows** instead (measured above), not labelled equivalent. The unviable mutant is the
  whole-function `cmd_doctor -> Ok(Default::default())`.
- **`gitcli.rs:354`**: **2 selected — 2 caught, 0 missed, 0 unviable** — both `allowed_env`
  replacement mutants are killed by the positive allowlist assertions.
- Suite on Linux at this head: **470 passed / 0 failed / 0 ignored**.

Across the two hosts every one of the four sites in this section is killed; the only Linux "missed"
mutant is the one whose Linux code path is the constant `false` branch, and it is measured on the host
where it is observable.


## 5. Per-module rates on Linux (MEASURED, scoped)

Method is wave 6's: a disposable ext4 clone pinned at `9bbb02d`, the campaign's own suite scope per
module (a **lower bound**, never a whole-crate rate), `--jobs 2`, `--timeout 240`, no shared
`CARGO_TARGET_DIR`, one module at a time. Baseline before each campaign: `cargo test --locked
--offline --all-targets` → 469 passed / 0 failed / 0 ignored.

| module | mutants | unviable | applicable | caught | missed | timeout | strict | wall | scope |
|---|---|---|---|---|---|---|---|---|---|
| `state` | 155 | 15 | 140 | 134 | 6 | 0 | **95.7 %** | 6 m 13 s | bins, cli, error_paths, operator_surfaces, crash_run |
| `commands` | 267 | 22 | 245 | 237 | 8 | 0 | **96.7 %** | 17 m 13 s | bins, cli, git_fidelity, operator_surfaces |
| `gitcli` | 106 | 18 | 88 | 73 | 15 | 0 | **83.0 %** | 6 m 49 s | bins, isolation, bounds, error_paths |
| `main` | 73 | 4 | 69 | **62 measured** | 2 | 2 | **93.9 %** (62/66) | partial | bins, cli, operator_surfaces |
| **all four** | **601** | 59 | 542 | **506** | 31 | 2 | **93.9 %** of 539 | — | — |

The 93.9 % is the campaign's own count, before adjudication. After the five closures and the
adjudications of §3–§4, the same corpus reads **523 caught / 16 residual / 3 not run** — 97.0 % of the
539 tested-applicable mutants, or 96.5 % of 542 if the three not-run mutants stay in the denominator.
Both figures are cross-host: the 357:39 site is counted where it is observable (Windows).

`main` is **partial and labelled so**: three `collect_values` hang mutants (`360:19`, `372:15 *=`,
`372:15 -=`) are `NOT_RUN`. Two full passes were externally terminated (~7.5 min) and a third was lost
to a WSL restart; the row is not padded. The two `main` misses (`83:9` help arm, `87:9` version arm)
are **caught** by the unselected `error_paths` suite, and the two `+=`→`*=` timeouts are **caught** in
about ten seconds by the unit watchdog under a libtest filter (wave-6 method reproduced), which is why
the adjudicated `main` figure is 66/69.

### 5.1 Adjudication of every survivor (MEASURED, full-suite reruns)

| module | host-gated | equivalent / not observable | real test gap | killed by a wider suite |
|---|---|---|---|---|
| `state` | 3 (`316` reparse check, `#[cfg(windows)]`) | 1 (`377` fsync, now killed) | 2 (`390`, `661` — now killed) | 0 |
| `commands` | 0 | 1 (`357:39`, Linux short-circuit; caught on Windows) | 1 (`357:26` — now killed) | 6 |
| `gitcli` | 5 (`523` ×2, `539` ×2, `543`) | 8 (1 non-serialised exit value; 7 timing/zombie boundaries: `623`, `640`, `666` ×2, `667` ×3) | 2 (`354` — now killed) | 0 |
| `main` | 0 | 0 | 0 | 2 misses + 2 timeouts |

Adjusted per module after the wave-8 closures, excluding host-gated code and keeping the labels:

- `state` **137/137 = 100 %** of the Linux-compiled mutants (137/140 = 97.9 % if the three Windows-only
  mutants are kept in the denominator).
- `commands` **244/245 = 99.6 %** on Linux, the remaining site being Linux-equivalent by the
  `cfg!(windows)` short-circuit; that same site is **caught on Windows** by the new assertion
  (measured), so across platforms no commands site survives.
- `gitcli` **75/88 = 85.2 %**, i.e. **75/83 = 90.4 %** excluding the five Windows-only mutants; the
  residual 13 are the 5 host-gated, 1 non-serialised-exit equivalent and the 7 timing/zombie
  boundaries listed above, none of them claimed as equivalent-by-construction.
- `main` **66/69 = 95.7 %** after adjudication; the 3 `NOT_RUN` hang mutants need a watchdog around
  `collect_values` (named in §8, not claimed).

### 5.2 Linux versus Windows (wave 6 §3; directional only)

| module | Windows wave 6 | Linux wave 8 | note |
|---|---|---|---|
| `state` | 87.4 % | **95.7 %** | three Windows-only mutants count as missed on Linux |
| `commands` | 84.2 % | **96.7 %** (99.6 % adjusted) | — |
| `gitcli` | 72.1 % | **83.0 %** (90.4 % adjusted) | source grew by two mutants |
| `main` | 91.8 % | **93.9 %** measured (partial) | — |

The comparison is **directional, not a controlled experiment**: the heads differ (`aa59736` vs
`9bbb02d`), the Linux job compiles code the Windows job does not, and the Windows numbers come from a
campaign whose flaky/interrupted runs were merged in wave 6. What it does establish is the shape: on
the same scoped suites, Linux measures higher everywhere, and the difference is largest where the
`#[cfg(unix)]` code lives.

## 6. The stability campaign found a failure, and it is reported

Eight consecutive full-suite runs on Windows at `3b1e33e` (one with `RUST_TEST_THREADS=1`, seven at the
CI default): **7 green (468 passed / 0 failed / 0 ignored), 1 failed**. The failing run (`run 7`) lost
four `proposals.rs` tests to **Windows resource exhaustion**, not to a product defect: the failures are
the product's own wrapper around a failed process spawn — `error launching git: Ressources système
insuffisantes…` (`ERROR_NO_SYSTEM_RESOURCES`) and `Le fichier de pagination est insuffisant…`
(`ERROR_COMMITMENT_LIMIT`) — because the run was executed concurrently with the WSL mutation campaign
on a 16 GB host. Run 8, four minutes later, is green, and `cargo test --test proposals` alone passes
23/23. The full record, including the caveat that this is a stability sample and not a flake-freedom
proof, is in [`experiments/quality/stability-w8/NOTES.md`](../../experiments/quality/stability-w8/NOTES.md).

## 7. Tests, floor, coverage and CI

- **One new test** (`state::tests::only_a_real_parent_directory_is_flushed_after_the_rename`, Unix-only)
  and **four assertion groups added to existing tests** (journal stamp, `iso8601_now` date, doctor
  `longpaths`, environment allowlist). No test, gate or security property was removed or weakened.
- Counts: **468 executed on Windows, 470 on Linux, 0 ignored**, measured locally at `3b1e33e` and
  confirmed by CI on the same head (run `43021374347`: `executed_passed=470`/`468`, `ignored=0`). The
  CI floor is raised to exactly those counts (Linux `minimum=470`, Windows `468`), and the Unix-only
  gate is skipped on Windows rather than claimed there.
- The coverage gate is unchanged (**95 % lines / 90 % branches, product-only**) and the `coverage` job
  re-measured it on `3b1e33e`: **95.88 % lines (4402/4591), 93.56 % branches (799/854)** — above the
  gate with no waiver. The extracted helper adds three covered product lines, so the denominator moved
  by three and the ratio is unchanged.
- CI on `3b1e33e`: push `43021374347` and PR `43021382769`, **5/5 jobs each** (test Ubuntu + Windows,
  polyglot ×2, coverage).

## 8. What is still not run, and the limits

- **macOS**: not measured.
- **`cargo-fuzz`**: still excluded by the zero-dependency contract; the deterministic randomised sweep
  remains the substitute and is never called fuzzing.
- **S9 confirmatory comparison**: still blocked by `H4`.
- **Host-gated mutants**: 3 in `state` and 5 in `gitcli` are `#[cfg(windows)]` code that Linux does not
  compile; a Windows campaign would be needed to measure them (wave 6's numbers cover the same code at
  an older head, not these exact mutants).
- **`state`: 1 site is a rule test, not a crash test** — `377`'s durability claim is pinned as a
  predicate; no power-cut harness exists.
- **`gitcli`: 7 boundary mutants are not deterministically observable** (nanosecond equality, a
  reader-grace race, four zombie-reaping operators). They are recorded as boundaries, never as
  equivalent.
- **`main`: 3 `NOT_RUN` hang mutants** (`collect_values`); the fix is a watchdog around that walk, the
  same shape as the `validate_flags` watchdog.
- **Scope**: module rates are lower bounds for the selected suites; only the survivors were re-run with
  the full suite. No whole-crate mutation rate is claimed.
- **Host stability during the campaign**: the WSL VHD (on `C:`) reached 0 bytes free, which produced
  `os error 5`, relay `getpwnam` failures and two `E_UNEXPECTED` restarts; one `gitcli` checkpoint was
  lost and that module was re-run fresh (no `--iterate` merge), and `main` lost one pass. This is an
  environment fact of the measurement, recorded rather than smoothed.
- Product decisions unchanged: sandbox `H3`, receipt signing (`PRF-01`), retention (`PRF-02/08`),
  external-`git`-writer fence.

## 9. Reproduce

```bash
cargo test --locked --offline --all-targets        # 468 Windows / 470 Linux, 0 ignored
cargo clippy --locked --offline --all-targets -- -D warnings
cargo fmt --all --check

# Windows killcheck of the doctor site (worktree pinned at the committed test head)
pwsh -File experiments/quality/mutation-w7/killcheck.ps1 \
  -Repo <pinned-worktree-at-f5a19de> -Module commands -Lines 357 \
  -Tests cli,git_fidelity,operator_surfaces -Jobs 1 \
  -OutDir experiments/quality/mutation-w8

# Linux killchecks (disposable ext4 clone at the named heads)
bash experiments/quality/mutation-w8-linux/lead-linux-check.sh    # state.rs:396 at 90640b8
bash experiments/quality/mutation-w8-linux/lead-linux-check2.sh   # state.rs:403,674 at 3b1e33e
bash experiments/quality/mutation-w8-linux/lead-linux-check4.sh   # commands.rs:357 + gitcli.rs:354 at 09c0b41
```

Evidence layout: `experiments/quality/mutation-w8/` (Windows killcheck of `commands.rs:357`),
`experiments/quality/mutation-w8-linux/` (the four module campaigns in `state/`, `commands/`,
`gitcli/`, `main/`, each with `mutants.out/` and `killcheck/`, plus my `lead-396/`, `lead-gaps/`,
`lead-*` probe artefacts), `experiments/quality/stability-w8/` (the eight-run CSV, the failing run's
full log, `NOTES.md`). Per-mutant logs are regenerable and excluded by `.gitignore`.
