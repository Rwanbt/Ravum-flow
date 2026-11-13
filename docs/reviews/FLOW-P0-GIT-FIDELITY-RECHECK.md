# FLOW P0 checkout-fidelity — independent re-check

**Date:** 2026-11-10 (Europe/Paris) · **Reviewer:** `verify-p0` (independent; did not write the product)
**Branch:** `feat/flow-production-candidate`
· **Pinned SHA (final):** `f5bb271ef7bfdb2d4adf429b31c4740d58c350ef` (branch head when pinned; docs only on top)
· **Head at report time:** `6525c46b595b8ae98df1df1ff589561fc23f2d8b` (docs only; `git diff
f5bb271..HEAD -- crates/` is empty, so the measured binary is still the product code)
· **Product code under test:** `d6d7ea9da7ec80e123b7c53a9ceae4b01c559dfc` ("fix: never skip a `.git` entry in
the verification checkout") — `git diff 7a034ff..f5bb271 -- crates/flow/src` is exactly one change:
`collect_entries` no longer skips a root `.git` (plus the matching `doctor` wording)
· **Earlier pins:** `7a034ff` (implementation of the post-run re-check; 19/19 P0 REFUTED, one residual
`.git` walker skip) and `d7e1015` (the round that found the `bae7390` implementation absent: tests and
docs only, gates red). Both are kept in the history below.
· **Harness:** `experiments/verify-p0/{p0_attacks.py, recheck_probes.py, recheck_defeat_probes.py,
linux_recheck_probe.sh}` + the committed campaign `experiments/mvp-qa/attacks.py`
· **Results:** `experiments/verify-p0/{results-p0-f5-a20.json, results-recheck-probes-f5.json,
results-recheck-defeat-f5.json}` (summaries, committed) and the verdict table
`experiments/verify-p0/verdict-tables-f5.md`. The multi-megabyte raw dumps
(`results-p0-f5.json`, `results-campaign-f5.json`) are regenerable with the commands in §"Reproduce"
below and are excluded by `.gitignore`; they are not cited as committed evidence.

| Item | Value |
|---|---|
| Pinned source | `git archive f5bb271` → `cargo build --locked --offline` (debug + release) |
| Debug binary | sha256 `50c92cd80be2947724be64da437ec19b7bd882e4af6f796c30af4767d2ee414d` |
| Release binary | sha256 `92b51555b209b4a15d0128829ffbc9a921579423098419ebf8c8c45c54214eae` |
| Pre-fix baseline (regressions) | `0de8c21` debug sha256 `2556febb27ac6f75c94aa5a641dce1cf14eed26c7ff2958d9d618ef8f4cc52ce` |
| Host | Windows 11 `10.0.26100`, Git `2.51.1.windows.1`, Python 3.13.7, rustc 1.95.0; WSL Ubuntu (rustc 1.95.0, git 2.43.0) |
| `git status` relevant to the build | no `crates/flow/src/**` modification in the checkout; the binary is the committed `f5bb271` content |

## 0. Pin history

| Pin | What it was | Result of that round |
|---|---|---|
| `d7e1015` | claimed `bae7390` fix | **implementation absent**: `git show --stat bae7390` = tests + two docs only; no `CHECKOUT_MUTATED_DURING_RUN`/`verify_checkout_unchanged`/`checkout_recheck` in `crates/flow/src`; its own new gates failed (Windows 13/3 of 16, Linux 14/4 of 18); A07/A08/A09 reproduced, `doctor` unchanged. |
| `7a034ff` | "land the checkout re-check implementation that `bae7390` lost" | implementation present; **19/19 P0 REFUTED** (A07 `CHECKOUT_MUTATED_DURING_RUN` no receipt; A08/A09 typed `UNSUPPORTED_CHECKOUT`; A16 `not valid UTF-8`; A14 `checkout_recheck: "OK"`; A18 truthful); one residual: the walker still skipped a root `.git` (probe (e), P3). |
| `f5bb271` (product `d6d7ea9`) | `.git` walker fix + docs | **this document.** `collect_entries` now skips only `.flow-scratch-owned`; a `.git` present at check time is an undeclared entry and is refused (A20). The `doctor` guarantee names its single exception. |

At `f5bb271` the committed gates are green: Windows `git_fidelity` **16/16**, Linux **18/18**.

## 1. P0 matrix at `f5bb271`

All scenarios are REFUTED; every refusal writes no receipt.

| ID | Attack | Observed (verbatim excerpt) | Verdict | Sev |
|---|---|---|---|---|
| A07 | delayed same-length rewrite; obligation reads it | **`REFUSED/CHECKOUT_MUTATED_DURING_RUN`**, no receipt; obligation log `content_hex=414c5048410a`; `entry "a.txt" changed while the obligations ran (6 vs 6)` | **REFUTED** | P1 |
| A08 | hook creates `.flow-scratch-injected.txt` + `.flow-scratch-dir/payload.txt` | **`REFUSED/UNSUPPORTED_CHECKOUT`**, no receipt: `holds 2 file(s) the tree does not declare (… ".flow-scratch-dir/payload.txt", ".flow-scratch-injected.txt")` | **REFUTED** | P3 |
| A09 | hook creates empty `.build-cache/` | **`REFUSED/UNSUPPORTED_CHECKOUT`**, no receipt: `holds 1 directory(ies) … (".build-cache")` | **REFUTED** | P3 |
| A16 | raw non-UTF-8 path (`git mktree` + shipped candidate) | **`REFUSED/UNSUPPORTED_CHECKOUT`**, no receipt: `holds a path that is not valid UTF-8: this platform cannot represent it faithfully` | **REFUTED** | P2 |
| A14 | positive control: faithful checkout, mode 100755 | `PASSED`, receipt written, `checkout_recheck: "OK"` | **REFUTED** (control) | P2 |
| A18 | `doctor` vs the delayed write | no unqualified “detected and refused” claim; limit stated (quoted in §5) | **REFUTED** | P2 |
| A19 | hook replaces a declared file with a directory | `REFUSED/UNSUPPORTED_CHECKOUT` (`missing 1 of 2 tree entries … ["a.txt"]`) | **REFUTED** | P1 |
| **A20** | **hook recreates `.git` after `materialise` unlinks it, before the check** | **`REFUSED/UNSUPPORTED_CHECKOUT`**, no receipt: `holds 1 file(s) the tree does not declare (for example [".git/HEAD"])` | **REFUTED** | P3 |
| A01/A02/A03/A05/A06/A10/A11 | local attrs/filters, hook add/rewrite, entry and byte budgets | typed refusals, no receipt; **pre-fix build still `PASSED` every one** | **REFUTED** | P0/P1 |
| A04/A12/A13/A15/A17 | in-tree WTE, symlink, gitlink, reserved/`..` via candidate, publish gate | typed refusals, no receipt; publish lands only the receipt tree | **REFUTED** | P1/P2 |

`results-p0-f5.json` is the 19-scenario run (all REFUTED); `results-p0-f5-a20.json` is the newly added
`.git`-race scenario (REFUTED). The full-suite fidelity gates at the pin: **16/16 Windows**, **18/18
Linux**.

## 2. `.git`: present at check time vs recreated after it (precise)

The walker now refuses a `.git` that exists when the checkout is checked, and still cannot see one
created later:

- **Present at check time — REFUSED (A20, deterministic).** `materialise` removes the worktree `.git`
  link and then writes the `.flow-scratch-owned` marker; the hook's background process waits for that
  marker (`while [ ! -f .flow-scratch-owned ]; do :; done`) and creates `.git/HEAD`. The run is
  `REFUSED/UNSUPPORTED_CHECKOUT` with `.git/HEAD` named as an undeclared file, no receipt. Three
  independent runs of the same probe refused 3/3 (`results-recheck-defeat-f5.json`, probe
  `e_dot_git_race`). At `7a034ff` the same fixture was `PASSED` because the walker skipped `.git`.
- **Recreated after the check — PASSED, documented (probe `e2_dot_git_after_check`).** An obligation that
  creates `.git/HEAD` while it runs: `PASSED`, receipt, `checkout_recheck: "OK"`, obligation log
  `git_dir_seen=True`. The re-check proves *declared* files only; extras created during the obligations
  are allowed by design (build outputs), and `doctor` states that a write after the check is not
  detected. This is the residual TOCTOU, not a walker gap: no filesystem walk can see a future write
  without a snapshot/sandbox.

## 3. Attempts to defeat the re-check (at `f5bb271`)

| Probe | Observed | Classification |
|---|---|---|
| (a) write-and-revert before the re-check | obligation reads the file, writes `ALPHA\n`, reads it back (`seen_hex=414c5048410a`), restores; run `PASSED`, receipt, `checkout_recheck: "OK"` | **documented limit** (`doctor`: “a write reverted before the re-check … is not detected”); confirmed |
| (b) extra dirs created, one deleted, one left | `PASSED`, receipt, `checkout_recheck: "OK"` | **documented by design**: the re-check re-proves declared files, not build outputs |
| (c1) hook re-creates a declared directory component (`nested`) | `PASSED`, receipt, recheck OK — no false positive | control passed |
| (c2) hook creates undeclared `nested/extra` | `REFUSED/UNSUPPORTED_CHECKOUT`: `holds 1 directory(ies) … ["nested/extra"]`, no receipt | control passed |
| (d) `chmod +x` a declared 100644 file during the obligations (Linux) | `REFUSED/CHECKOUT_MUTATED_DURING_RUN`: `declares mode 100644 but is executable`; no receipt; clean control `PASSED` + `checkout_recheck: "OK"` | control passed |
| (e) `.git` re-created before the check | `REFUSED/UNSUPPORTED_CHECKOUT` (`.git/HEAD`), no receipt, 3/3 | **closed** (was the `7a034ff` residual) |
| (e2) `.git` created after the check, by an obligation | `PASSED`, receipt, recheck OK, `git_dir_seen=True` | **documented limit** (§2) |

## 4. Campaign at `f5bb271` (committed harness, corrected W3-03)

The committed `experiments/mvp-qa/attacks.py` at `f5bb271` includes the corrected W3-03 assertion
(`changed during checkout` accepted; `d6d7ea9`), so this run needs no wrapper.

```powershell
python experiments\mvp-qa\attacks.py `
  --flow "$env:TEMP\flow-f5-target\debug\flow.exe" `
  --release-flow "$env:TEMP\flow-f5-target\release\flow.exe" `
  --out experiments\verify-p0\results-campaign-f5.json `
  --markdown experiments\verify-p0\verdict-tables-f5.md `
  --base-sha f5bb271ef7bfdb2d4adf429b31c4740d58c350ef
```

| | `f5bb271` raw (committed harness) | `7a034ff` raw (old harness) | `7a034ff` corrected |
|---|---|---|---|
| REPRODUCED | **6** | 7 (spurious W3-03 P0) | 6 |
| REFUTED | **115** | 114 | 115 |
| NOT_RUN | **1 (`PRF-09`)** | 1 | 1 |
| ERROR | **0** | 0 | 0 |
| Harness exit | **0 (“OK: no reproduced P0”)** | 1 | 0 |

The six are the same as waves 2–4: `FIL-20` P2, `PRC-10` P2, `PRF-01` P2, `PRF-02` P3, `PRF-08` P3,
`CMP-01` P3; `PRF-09` is the one NOT_RUN. Per-axis (total/reproduced): identity 20/0, scope 11/0,
git 12/0, durable 11/0, files 23/1, process 14/1, proof 9/3 + 1 NOT_RUN, compat 7/1, future 7/0,
inputs 5/0, fidelity 3/0. **No harness false positive at `f5bb271`:** the committed W3-03 assertion
accepts the current wording and is REFUTED, as the Lead expected.

For comparison, the `7a034ff` raw campaign (before the harness correction) reported 7/114/1 with the
spurious W3-03 P0; corrected, it was 6/115/1. That false positive is now impossible from the committed
harness.

## 5. Documentation check (A18)

Measured `doctor` strings at `f5bb271` (verbatim):

- guarantee: `"the verification checkout equals the verified tree exactly when it is checked: every
  regular file byte for byte, executable bit on Unix, no missing file, no extra file or directory
  (Flow's own scratch ownership marker excepted) — and the declared files are re-proved unchanged after
  the obligations ran (a change refuses the run)"`
- non-guarantee: `"hooks are not suppressed: `git worktree add` runs post-checkout and `git update-ref`
  runs reference-transaction. A change already present when the checkout is checked is refused, and a
  change to a declared file while the obligations run is caught by a re-check after the last obligation;
  a hook that writes outside the checkout, a write reverted before the re-check, or a write after it is
  not detected. A blocking reference-transaction aborts publish"`

Both now match the code and the measurements: the marker exception is the only skip (A08/A20); a `.git`
at check time is refused (A20); a write reverted before the re-check (a) or after it (e2) is not
detected. The user-guide Hooks text and `INVARIANTS` I24/I25/I26 describe the behaviour that exists.
**A18 is closed.**

## 6. Closed findings, documented limits, and the release verdict

**Closed at `f5bb271` (measured):** the fidelity P0 class (local attribute/filter conversions,
synchronous hook add/rewrite/delete, budget bypass), A07 (delayed write → `CHECKOUT_MUTATED_DURING_RUN`,
no receipt), A08 (`.flow-scratch*` file), A09 (empty extra directory), A16 (`not valid UTF-8` reason),
A18 (doctor/invariants truthful), the `.git` walker skip (A20), and the `d7e1015` “fix absent” finding.

**Documented limits (not closed, by design):** a write to a declared file reverted before the re-check
(a); any write after the re-check or made by an obligation (e2, extras for build outputs); writes
outside the checkout; an external `git` writer (no fencing); no OS sandbox; forgeable receipts.

**Accepted campaign residuals (unchanged from waves 2–4):** `FIL-20` (journal hardlink), `PRC-10`
(debug-only `FLOW_TEST_CRASH`; release ignores it), `PRF-01` (re-sealed forged receipt — documented),
`PRF-02`/`PRF-08` (deleted log files still displayed and publishable), `CMP-01` (help/usage is not
JSON) — P2/P3, none P0/P1.

**One-paragraph verdict (quote-ready):** At `f5bb271` (product code `d6d7ea9`) every P0/P1/P2 finding
this review filed against the checkout-fidelity gate is closed and re-measured on Windows and Linux:
the verification checkout now equals the verified tree when it is checked — byte for byte, Unix mode
included, no undeclared file or directory (a re-created `.git` included), with local attribute/filter
conversions and delayed writes to declared files refused with typed codes and no receipt, and a clean
receipt carrying `checkout_recheck: "OK"`. The remaining risks are documented, not defects: a write
reverted before the post-run re-check (and anything written after it, including a `.git` recreated
after the check or by an obligation) is not detected; extras created during obligations are allowed for
build outputs; writes outside the checkout, an external `git` writer, the absence of an OS sandbox, and
receipt forgery by a writer of `.flow/receipts` are outside the guarantee; the campaign still records
six accepted P2/P3 observations (journal hardlink, debug-only crash hook, forged receipt, deleted
evidence still displayed/publishable, non-JSON help). I would sign the release with those written down
as release notes, not as open fidelity defects.

## 7. Reproduce

```powershell
git archive f5bb271 -o %TEMP%\flow-f5.tar    # extract
# with CARGO_TARGET_DIR outside the repo:
cargo build --locked --offline ; cargo build --release --locked --offline

python experiments/verify-p0/p0_attacks.py `
  --post "%TEMP%\flow-f5-target\debug\flow.exe" `
  --pre  "%TEMP%\flow-prefix-target\debug\flow.exe" `
  --out  experiments/verify-p0/results-p0-f5.json          # 19 scenarios
python experiments/verify-p0/p0_attacks.py ... --only A20 `
  --out experiments/verify-p0/results-p0-f5-a20.json       # the `.git` race
python experiments/verify-p0/recheck_probes.py "%TEMP%\flow-f5-target\debug\flow.exe" `
  experiments/verify-p0/results-recheck-probes-f5.json
python experiments/verify-p0/recheck_defeat_probes.py "%TEMP%\flow-f5-target\debug\flow.exe" `
  experiments/verify-p0/results-recheck-defeat-f5.json
wsl -e bash -lc 'cd /mnt/d/App/Ravum-flow && PIN=f5bb271 bash experiments/verify-p0/linux_recheck_probe.sh'
```

Files written this round (new; no product source, tests or existing docs touched):
`experiments/verify-p0/{results-p0-f5.json, results-p0-f5-a20.json, results-recheck-probes-f5.json,
results-recheck-defeat-f5.json, results-campaign-f5.json, verdict-tables-f5.md}` and this document
(updated in place: it now pins `f5bb271`/`d6d7ea9` and records the `7a034ff` and `d7e1015` history).

Claim classification: **MEASURED** (binary observed), **CODE_READ** (source/history read), **PROPOSED**
(fix direction), **NOT_RUN** (campaign `PRF-09` only, as in every previous round).
