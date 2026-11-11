# Wave 8 — Linux per-module mutation rates (modules wave 7 left NOT_RUN)

Status: **measurement only.** No product source changed on the Windows checkout, nothing committed or
pushed. Campaign runs in a disposable ext4 clone pinned at the measured commit; probes for survivor
adjudication are reverted. Evidence for each module is under `<module>/` (campaign) and
`<module>/killcheck/` (wider-suite re-run). All numbers below are MEASURED unless marked otherwise.

## Environment and method

- Host: WSL2 Ubuntu, ext4 `/home/aes`; rustc/cargo 1.95.0 (pinned); git 2.43; cargo-mutants 27.1.0.
- Tree: `git clone /mnt/d/App/Ravum-flow /home/aes/flow-w8`, `git checkout 9bbb02d` →
  `9bbb02dacd1051817ddab8bdcd0555a2d60207e0`.
- Green baseline before any campaign: `cargo test --locked --offline --all-targets` →
  **469 passed, 0 failed, 0 ignored** (`baseline-tests.log`).
- Method per module (wave-6 §2 / wave-7 method): copy mode (`--gitignore true`), `--jobs 2`,
  `--timeout 240`, **no shared `CARGO_TARGET_DIR`**, no `--in-place`, no `--iterate` as a primary
  pass, one module at a time. Suite scope is a **lower bound**, never a whole-crate rate.
- Survivor adjudication: a probe commit marks the survivor lines, `git diff -U0` builds an
  `--in-diff` selection, and the mutants are re-run with the **full suite** (all test binaries).
  The probe is reverted after the run. A raw "caught" is never accepted if it rests on a known
  flake; wave-7's `scratch` sequence-counter flake is watched for in every run.

## `state` — 155 mutants (scope: `bins`, `cli`, `error_paths`, `operator_surfaces`, `crash_run`)

Command (from the pinned clone):

```
cargo mutants --file crates/flow/src/state.rs --jobs 2 --timeout 240 --gitignore true \
  --output …/experiments/quality/mutation-w8-linux/state \
  -- --bins --test cli --test error_paths --test operator_surfaces --test crash_run
```

| mutants | unviable | applicable | caught | missed | timeout | strict | wall time |
|---|---|---|---|---|---|---|---|
| 155 | 15 | 140 | 134 | 6 | 0 | **95.7 %** (134/140) | 6 m 13 s (tool summary "7m") |

Baseline inside the campaign: 5 s build + 11 s test. `caught.txt` / `missed.txt` /
`unviable.txt` / `outcomes.json` and the console log are in `state/`.

### Survivor adjudication (full-suite `--in-diff` re-run; 13 selected, 7 collateral caught)

The four survivor lines (`316, 377, 390, 661`) expand to their enclosing functions, so the targeted
run selected 13 mutants: the 6 campaign survivors stayed **missed**, and 7 collateral
function-level mutants were caught. Baseline for the wider run: 7 s build + 28 s test. Evidence:
`state/killcheck/` (`probe.diff`, `mutants.out/`, `console.log`).

| survivor (exact mutant) | class | proof / needed kill |
|---|---|---|
| `state.rs:316:79 replace != with == in guard_below_flow` | **HOST-GATED** | line 316 is inside `#[cfg(windows)]` (`meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT`); on Linux the block is removed and the `#[cfg(not(windows))]` arm returns `false`, so the mutant cannot change a Linux binary. Not a test gap; only a Windows campaign can measure it. |
| `state.rs:316:48 replace & with \| in guard_below_flow` | **HOST-GATED** | same Windows-only block. |
| `state.rs:316:48 replace & with ^ in guard_below_flow` | **HOST-GATED** | same Windows-only block. |
| `state.rs:377:52 delete ! in write_atomic_bytes` | **UNOBSERVABLE** | skips the directory `fsync` after the atomic rename; the only observable difference is durability across a crash between rename and fsync. No test injects fsync failure; deterministic kill needs a crash/power-loss harness or an `LD_PRELOAD`/FUSE shim. Not claimed equivalent. |
| `state.rs:390:41 replace == with != in append_journal` | **REAL TEST GAP** | the unit test appends a record that already carries `schema` plus a non-object; it never appends a schema-less **object**. With the mutant, `!any(k != "schema")` is false for any object with a non-schema key, so the schema is not stamped. Manual repro: `if !pairs.iter().any(\|(k,_)\| k != "schema")` passes the existing test. Kill: append `Json::obj(vec![("event", Json::str("note"))])` and assert the stored record's `schema == "flow-journal/1"` (unit test, ~no cost). |
| `state.rs:661:22 replace / with % in iso8601_now` | **REAL TEST GAP (time-dependent)** | `days = secs % 86_400` yields 0–86 399 days → years 1970–2206; the test only asserts `year >= 2024`, so the mutant is caught only when run before ~05:29 UTC (days < 19 723). Manual repro at 12:xx UTC passes (year ≈ 2093). Kill: assert an upper bound on the year, or compare `now[..10]` with `civil_from_days(unix_now()/86_400)` (unit test). |

Strict scoped rate: **95.7 % (134/140)**. The three `#[cfg(windows)]` mutants are counted as
missed by the Linux tool; excluding them from the applicable denominator gives
**134/137 = 97.8 %** — labelled separately because they are not Linux code.
The campaign rate is a **lower bound** for the selected suites; the wider-suite killcheck above
already re-tested the six survivors against all suites and none became caught except the collateral
mutants. No campaign survivor is a product defect (the product behaviour is correct; two assertions
are missing and one check is inherently unobservable).

## `commands` — 267 mutants (scope: `bins`, `cli`, `git_fidelity`, `operator_surfaces`)

Command (from the pinned clone):

```
cargo mutants --file crates/flow/src/commands.rs --jobs 2 --timeout 240 --gitignore true \
  --output …/experiments/quality/mutation-w8-linux/commands \
  -- --bins --test cli --test git_fidelity --test operator_surfaces
```

| mutants | unviable | applicable | caught | missed | timeout | strict | wall time |
|---|---|---|---|---|---|---|---|
| 267 | 22 | 245 | 237 | 8 | 0 | **96.7 %** (237/245) | 17 m 13 s (tool summary "18m") |

Baseline inside the campaign: 12 s build + 11 s test. Evidence: `commands/` (`mutants.out/`
lists and `outcomes.json`; the ~1 GB per-mutant verbose log directory was not copied, only the
survivor logs/diffs under `commands/survivors/`).

### Survivor adjudication (full-suite `--in-diff` re-run; 19 selected)

Selection expanded the 7 survivor lines to 19 mutants (whole-function mutants of `adoption_gate`,
`cmd_doctor`, `cmd_publish` plus other operands of the same conditions). Full-suite baseline:
7 s build + 37 s test. Result: 14 caught, 2 missed, 3 unviable.
Evidence: `commands/killcheck/` (`probe.diff`, `mutants.out/`, `console.log`, survivor logs).

| campaign survivor | wider-suite outcome | class |
|---|---|---|
| `commands.rs:217:5 adoption_gate -> Ok(None)` | CAUGHT | killed by a wider suite (adoption tests outside the campaign scope) |
| `commands.rs:223:27` policy-digest guard → `false` | CAUGHT | killed by a wider suite |
| `commands.rs:1592:31` `\|\|`→`&&` in ref validation | CAUGHT | killed by a wider suite |
| `commands.rs:1751:28` required-status guard → `false` | CAUGHT | killed by a wider suite |
| `commands.rs:1778:8 delete !` (commit-tree check) | CAUGHT | killed by a wider suite (error_paths/proposals) |
| `commands.rs:1824:31` `!=`→`==` (candidate base) | CAUGHT | killed by a wider suite (proposal candidate tests) |
| `commands.rs:357:26 replace && with \|\| in cmd_doctor` | **MISSED** | **REAL TEST GAP (Linux)**: `if cfg!(windows) && longpaths != "true"`; on Linux the mutant warns about `core.longpaths` instead of reporting OK, and no test asserts the doctor `longpaths` check. Kill: a doctor test asserting the check's status is `"OK"` on a non-Windows host. |
| `commands.rs:357:39 replace != with == in cmd_doctor` | **MISSED** | **EQUIVALENT ON LINUX**: `cfg!(windows)` is `false`, so `false && X` short-circuits and both the original and the mutant produce `false`/OK. Only a Windows test can distinguish it (the expression is Windows-only semantics). Not counted as a test gap. |

After the wider-suite adjudication: **243/245 = 99.2 %** with 2 residual (one real Linux test gap,
one Linux-equivalent expression). Strict campaign rate remains the scoped **96.7 %**; the adjusted
number is reported separately because it is a re-classification, not the campaign's own count.
No product defect found; campaign not stopped.

## `gitcli` — 106 mutants (scope: `bins`, `isolation`, `bounds`, `error_paths`)

**Run history (environment failure, reported as it happened):** the first campaign was interrupted
when the host C: volume (which holds the WSL ext4 VHD) reached 0 bytes free; writes failed with
`Input/output error (os error 5)` and the distro relay died. A `--iterate` resumption was attempted
after a first host cleanup but failed for the same reason and overwrote the checkpoint
(`outcomes.json` was lost). After freeing host caches and obsolete WSL clones, the module was
**re-run fresh**; the numbers below are a single complete pass, not a merged/partial count.
The clone was verified clean at `9bbb02d` and the suite re-checked green (469/0/0) before the re-run.

| mutants | unviable | applicable | caught | missed | timeout | strict | wall time |
|---|---|---|---|---|---|---|---|
| 106 | 18 | 88 | 73 | 15 | 0 | **83.0 %** (73/88) | 6 m 49 s (tool summary "7m") |

Baseline inside the campaign: 7 s build + 14 s test. Flake audit: no caught mutant in this module
relied on the known `scratch` sequence-counter tests (checked every per-mutant log). Evidence:
`gitcli/` (`mutants.out/`, console log, survivor logs under `gitcli/survivors/`; the verbose
per-mutant log directory was not copied).

### Survivor adjudication (full-suite `--in-diff` re-run; 21 selected)

21 mutants selected (survivor lines expand to enclosing functions): **2 caught, 4 unviable,
15 missed**; baseline 7 s + 37 s. All 15 campaign survivors remain missed with the full suite.
Evidence: `gitcli/killcheck/` (`probe.diff`, `mutants.out/`, `console.log`, survivor logs).

| survivor(s) | class | proof / needed kill |
|---|---|---|
| `gitcli.rs:523:5 resolve_program -> None`, `:523:5 -> Some(Default::default())` | **HOST-GATED** | the mutated function is the `#[cfg(windows)] fn resolve_program` (diff shows it); on Linux that definition is compiled out and `#[cfg(not(windows))] fn resolve_program` (line 570) is used. Cannot affect a Linux binary. |
| `gitcli.rs:539:5 resolve_program_in -> None`, `:539:5 -> Some(Default::default())`, `:543:21 delete !` | **HOST-GATED** | all three are inside `#[cfg(windows)] fn resolve_program_in`. |
| `gitcli.rs:153:36 replace - with + in run_bounded` (`exit.unwrap_or(-1)` → `1`) | **EQUIVALENT (observable contract)** | every consumer of `Output.status` tests zero/non-zero or `.success()` (grep: `commands.rs:46,899,961,1094,1311,1598,1632,1942`, `paths.rs:212`, `lock.rs`, `scratch.rs`, `gitcli.rs:162`); `-1` and `1` are both non-zero, and the numeric status is never serialized. No input distinguishes them through the API. |
| `gitcli.rs:354:5 allowed_env -> vec![]`, `:354:5 -> vec![(Default::default(), Default::default())]` | **REAL TEST GAP** | no test asserts the positive side of the allowlist: the secrets test only asserts an *unlisted* variable is not inherited, which an empty allowlist also satisfies; the unit test covers `env_allowlist()` content, not `allowed_env()` filtering. Kill (cheap): a unit assertion that `allowed_env()` contains an allowlisted variable present in the host env and excludes a random one, or an obligation that echoes an allowlisted variable. |
| `gitcli.rs:623:38 replace > with >= in run_obligation` | **BOUNDARY, not observably testable** | differs only when `elapsed()` equals the deadline exactly (nanosecond); no deterministic input hits it. |
| `gitcli.rs:640:42 replace + with - in run_obligation` | **NOT DETERMINISTICALLY OBSERVABLE** | `reader_deadline = now - READER_GRACE` makes `receive` use a zero timeout; tests pass because the reader has already delivered the bytes when the child exits. A kill would need a synchronised reader (child holds the pipe open after writing); the full suite twice passed. |
| `gitcli.rs:666:5 reap_bounded -> ()`, `:666:35 +→-`, `:667:26 <→==`, `:667:26 <→>`, `:667:26 <→<=` | **UNOBSERVABLE (hygiene)** | `reap_bounded` only waits for a killed child to be reaped so no zombie accumulates; no `Outcome` field or return value depends on it, and `<=` differs only at exact equality. Only a `/proc`-based zombie check could see it (Linux-only, timing-sensitive); not a functional contract. |

After the wider-suite adjudication: **73/82 = 89.0 %**, excluding the 5 Windows-only host-gated
mutants and 1 provably equivalent mutant; residual 9 = 2 real test gaps + 7 non-observable/boundary.
Strict campaign rate remains the scoped **83.0 %**. No product defect found; campaign not stopped.

## `main` — 73 mutants (scope: `bins`, `cli`, `operator_surfaces`) — **PARTIAL, see environment**

**Run history (environment failure):** two full passes (one `334…`-style clean run, then one `--iterate`
resumption attempt after the first external kill) were terminated by an external **SIGTERM**
(`process_status=Signalled(15)`, both workers, at ~7.5 min) with no summary; a third fresh pass reached
62/73. The resumption overwrote the checkpoint (only 2 outcomes survived), so the module was completed
in short targeted passes after the host was restarted: 8 mutants at lines 382/389/403/407 (all caught,
40 s), the 2 `collect_values` hang sites (run aborted by another WSL crash), and the 2 help/version
misses adjudicated separately (14 s). Total measured wall time is therefore not a single-pass number;
the run was interrupted three times by host/C: failures.

| mutants | unviable | applicable | caught (measured) | missed (measured) | timeout (measured) | strict (measured) | not run |
|---|---|---|---|---|---|---|---|
| 73 | 4 | 69 | 62 | 2 | 2 | **93.9 %** (62/66 tested-applicable) | **3** |

If the 3 unmeasured mutants are charged as not caught against the full applicable denominator:
**62/69 = 89.9 %**. After adjudication of the measured subset (help/version and the two watchdog-caught
timeouts): **66/69 = 95.7 %**, 3 not run. All three labels are given; the row is **not claimed as a
complete pass**.

### Survivors and adjudications

| survivor | outcome | class |
|---|---|---|
| `main.rs:83:9 delete match arm "help" \| "-h" \| "--help"` | campaign MISSED → **CAUGHT** by `error_paths` (probe run: 2 caught, 2 unviable whole-fn collateral) | killed by a wider suite (`usage_and_version_contracts_do_not_touch_the_repository`) |
| `main.rs:87:9 delete match arm "version" \| "--version"` | same | killed by a wider suite |
| `main.rs:281:19 replace += with *= in validate_flags` | campaign TIMEOUT (240 s) → **CAUGHT** in ~10 s with a libtest filter on `tests::a_bare_double_dash_is_skipped_and_the_walk_terminates` | timeout is a suite-hang artefact: the CLI suites spawn `flow` processes that hang; the unit watchdog returns and fails fast. Wave-6 method reproduced on Linux (4 of the `+=`→`*=` mutants caught this way, including 300/343). |
| `main.rs:329:15` same | same | same |
| `main.rs:360:19`, `:372:15` `replace += with *= in collect_values` | **NOT RUN** (the targeted `collect_values` pass was aborted by the WSL crash before any mutant outcome) | CODE_READ: `index += 1`→`*=` never advances; infinite loop. Kill needs a watchdog test around `collect_values` (the existing watchdog covers only `validate_flags`/`flag_value`). Not claimed as timeout or kill. |
| `main.rs:372:15 replace += with -= in collect_values` | **NOT RUN** | CODE_READ: `usize` underflow panics in debug; a unit test on `collect_values` would likely catch it quickly. |

The 4 unviable mutants are whole-function `Default::default()` replacements that do not compile
(`Json`/`Outcome`/return types); they were never executed.

## Linux vs Windows (wave 6 §3) — same module scopes, different heads

Wave 6 measured head `aa59736` on Windows; this wave measured `9bbb02d` on Linux. The counts are not
identical because the source and host-gated mutants differ; the comparison is directional, not exact.

| module | Win: mutants / unviable / caught / missed / timeout / strict | Linux: mutants / unviable / caught / missed / timeout / strict | note |
|---|---|---|---|
| `state` | 155 / 12 / 125 / 18 / 0 / **87.4 %** | 155 / 15 / **134** / **6** / 0 / **95.7 %** | 3 Linux "missed" are `#[cfg(windows)]`; excluding them 97.8 % |
| `commands` | 267 / 20 / 208 / 39 / 0 / **84.2 %** | 267 / 22 / **237** / **8** / 0 / **96.7 %** | 6 of 8 survivors killed by the full suite; 99.2 % after adjudication |
| `gitcli` | 104 / 18 / 62 / 24 / 0 / **72.1 %** | 106 / 18 / **73** / **15** / 0 / **83.0 %** | source grew +2 mutants; 5 host-gated + 1 equivalent excluded → 89.0 % |
| `main` | 73 / 0 / 67 / 2 / 4 / **91.8 %** | 73 / 4 / **62 measured** / 2 / 2 / **93.9 % measured** | **partial**: 3 NOT_RUN; 66/69 = 95.7 % adjudicated |
| **all four** | 599 / 50 / 462 / 83 / 4 / **84.2 %** | 601 / 59 / 506 measured / 31 / 2 / **93.9 % measured** | aggregates are not like-for-like: Linux has 3 NOT_RUN and 8 host-gated survivors; no whole-crate claim |

Directional reading: on the same scopes the Linux suite kills more mutants per applicable mutant in
`state`, `commands` and `gitcli`, mainly because Windows wave 6 could not compile the Unix-only
mutants (they were counted as missed) and because the suite grew between the heads. `main` is within
noise of wave 6 once the timeout/watchdog handling is accounted for, but it is a partial row here.

## What this measurement does NOT cover

1. **Host-gated code.** `#[cfg(windows)]` mutants cannot affect a Linux binary and are counted as
   missed by the tool: `state.rs:316` ×3; `gitcli.rs:523` ×2, `:539` ×2, `:543` ×1. The Linux rate is
   reported both with and without them; only a Windows run measures them.
2. **Whole-module rates with the full suite.** The campaign scope is a lower bound by construction
   (selected suites). Only the *survivors* were re-tested with the full suite.
3. **Not observable / harness-dependent behaviour.** `state.rs:377` (directory fsync durability),
   `gitcli.rs:623/640/666/667` (exact-nanosecond boundary, reader-grace race, zombie reaping) survive
   even the full suite; killing them needs a crash/power-loss harness, synchronised reader, or a
   process-table assertion.
4. **`main` completeness.** 3 mutants (`collect_values` hangs) are NOT_RUN because the WSL distro
   crashed during the targeted pass; the row is partial and labelled as such.
5. **Later fixes.** This wave measured `9bbb02d`. The Lead reports `state:390`/`state:661` fixed by
   assertions at `3b1e33e` and `commands:357:26` at `f5a19de`; those are **not** measured here and are
   not re-opened. `commands:357:39` remains Linux-equivalent by short-circuit (the Windows branch is
   now asserted, but no Windows mutation kill is claimed).

## Environment incidents (MEASURED; they shaped the numbers)

- The host C: volume (holding the WSL ext4 VHD) reached **0 bytes free**; writes failed with
  `Input/output error (os error 5)` and the WSL relay died (`getpwnam failed`,
  `CreateProcessCommon:805`). This killed the first `gitcli` pass and the first `main` passes.
- `--iterate` resumptions were attempted but the same I/O failure overwrote the checkpoints
  (`outcomes.json` lost for gitcli and main), so those modules were **re-run fresh** (gitcli) or
  completed in short targeted passes (main). `--iterate` was never used as a primary pass; both uses
  were resumption attempts and are labelled as such.
- Two `main` passes were externally SIGTERMed (`Signalled(15)`) at ~7.5 min; the distro also crashed
  twice with `Wsl/Service/E_UNEXPECTED`. Freeing host caches/obsolete WSL clones allowed the short
  passes to complete; the `main` collect-values pass still died mid-run.
- No partial pass is presented as a complete row; the `main` row is explicitly partial.

## Provenance notes

- Measured tree: `9bbb02d`. After the campaign the branch advanced (current head at report time
  `09c0b41`, later commits include the assertions the Lead reports at `3b1e33e`/`f5a19de` and a
  gitcli allowlist test at `09c0b41`); **none of those later commits is measured in this report**.
- `lead-396/` and `lead-gaps/` inside this evidence directory were created by the Lead for their own
  later-commit probes; they are **not** part of this wave's campaign and are not used in any rate
  above.
- All campaign/probe runs used the pinned clone; probes were reverted (`git reset --hard 9bbb02d`);
  no product source was changed on the Windows checkout and nothing was committed/pushed by this
  wave.
