# FLOW P0 checkout-fidelity — independent re-check

**Date:** 2026-11-10 (Europe/Paris) · **Reviewer:** `verify-p0` (independent; did not write the product)
**Branch:** `feat/flow-production-candidate`
· **Pinned SHA (this round):** `7a034ffb855ccd020f1abdd46034c7bd2c33d416` ("fix: land the checkout re-check
implementation that `bae7390` lost")
· **Previously pinned SHA:** `d7e1015ccc1e0f270bcf8b63ea8afd48d652de08` — the round that reported the
`bae7390` implementation absent (tests and docs without `crates/flow/src`; gates red). Those results are
kept in `results-p0-recheck.json`, `results-recheck-probes.json`, `results-campaign-recheck.json` and
summarised in §0.
· **Harness:** `experiments/verify-p0/p0_attacks.py` (19 scenarios), `recheck_probes.py`,
`recheck_defeat_probes.py`, `linux_recheck_probe.sh`
· **Results:** `experiments/verify-p0/{results-p0-7a.json, results-recheck-probes-7a.json,
results-recheck-defeat-7a.json, results-campaign-7a.json}`

| Item | Value |
|---|---|
| Pinned source | `git archive 7a034ff` → `cargo build --locked --offline` (debug + release) |
| Debug binary | sha256 `520a6bee56fe133a1dde95e810c9a7f61a43eaa66114307728d618b431fe6eff` |
| Release binary | sha256 `d86b9fc8de89a96e8c5d333c1728e5df2ab2d66766867d336eadc582ad94d30` |
| Pre-fix baseline (regressions) | `0de8c21` debug sha256 `2556febb27ac6f75c94aa5a641dce1cf14eed26c7ff2958d9d618ef8f4cc52ce` |
| Host | Windows 11 `10.0.26100`, Git `2.51.1.windows.1`, Python 3.13.7, rustc 1.95.0; WSL Ubuntu (rustc 1.95.0, git 2.43.0) |
| Head at report time | `14fd28cf1cc9d06a31bdcff1400b50aa732285ff` — since `7a034ff` only docs/release-script/artefact commits; `git diff 7a034ff..HEAD -- crates/` is empty, so the measured binary is still the product code |
| `git status` relevant to the build | no `crates/flow/src/**` modification in the checkout; binary is the committed `7a034ff` content |

## 0. Pin history (why this round exists)

| Round | Pin | Finding |
|---|---|---|
| 1 | `d7e1015` | `bae7390` claimed the re-check fix but changed **only** `git_fidelity.rs` + two docs; `CHECKOUT_MUTATED_DURING_RUN`/`verify_checkout_unchanged`/`checkout_recheck` did not exist. A07/A08/A09 reproduced, A16 gave the old lossy reason, `doctor` unchanged; its own new gates failed (Windows 13 passed / 3 failed of 16; Linux 14/4 of 18). |
| 2 | `7a034ff` | `git diff --stat d7e1015..7a034ff` = `crates/flow/src/commands.rs` +262 lines (plus CI and `scripts/release/*`). `git grep` finds `ls_tree_entries`, `verify_checkout_unchanged`, `CHECKOUT_MUTATED_DURING_RUN`, `checkout_recheck`. **This document reports this round.** |

At `7a034ff` the committed gates are green: Windows `git_fidelity` **16/16**, Linux **18/18** (the two
Unix-only gates included), and the re-check implementation is present in the source.

## 1. Verdict table — 19 P0 scenarios on `7a034ff`

All 19 are REFUTED. Exit/status and receipt presence are the CLI contract; every refusal writes no
receipt.

| ID | Attack | Observed (verbatim excerpt) | Verdict | Sev |
|---|---|---|---|---|
| A07 | delayed same-length rewrite after the one-shot check; obligation reads it | **`REFUSED/CHECKOUT_MUTATED_DURING_RUN`**, no receipt; obligation log `content_hex=414c5048410a` (it *did* read `ALPHA\n`), refusal: `entry "a.txt" changed while the obligations ran (6 on disk vs 6 in the tree …)` | **REFUTED** | P1 |
| A08 | hook creates `.flow-scratch-injected.txt` + `.flow-scratch-dir/payload.txt` | **`REFUSED/UNSUPPORTED_CHECKOUT`**, no receipt: `the checkout holds 2 file(s) the tree does not declare (for example [".flow-scratch-dir/payload.txt", ".flow-scratch-injected.txt"])` | **REFUTED** | P3 |
| A09 | hook creates empty `.build-cache/` | **`REFUSED/UNSUPPORTED_CHECKOUT`**, no receipt: `the checkout holds 1 directory(ies) the tree does not declare (for example [".build-cache"])` | **REFUTED** | P3 |
| A16 | raw non-UTF-8 path (`bad\xff-name.txt` via `git mktree` + a shipped candidate record) | **`REFUSED/UNSUPPORTED_CHECKOUT`**, no receipt: `tree … holds a path that is not valid UTF-8: this platform cannot represent it faithfully, so the tree cannot be verified` | **REFUTED** | P2 |
| A14 | positive control: faithful checkout, mode 100755 | `PASSED`, receipt written, `checkout_recheck: "OK"` | **REFUTED** (control) | P2 |
| A18 | `doctor` vs the delayed write | `doctor` no longer makes the unqualified “detected and refused” claim (quoted in §4); the limit is stated | **REFUTED** | P2 |
| A01 | local `.git/info/attributes` + same-length smudge filter | `REFUSED/UNSUPPORTED_CHECKOUT` “changed during checkout” (6 vs 6); **pre-fix `PASSED`** | **REFUTED** | P0 |
| A02 | `core.attributesFile` outside the repo + filter | refused (6 vs 6), no receipt; pre-fix `PASSED` | **REFUTED** | P0 |
| A03 | local `eol=crlf` | refused (7 vs 6), no receipt; pre-fix `PASSED` | **REFUTED** | P0 |
| A05 | synchronous hook adds a file | refused “1 file(s) … does not declare”, no receipt; pre-fix `PASSED` | **REFUTED** | P1 |
| A06 | synchronous hook same-length rewrite | refused (6 vs 6), no receipt; pre-fix `PASSED` | **REFUTED** | P0 |
| A10 | 20 002 regular files (entry budget) | refused with the 20 000-entry budget message, no receipt; pre-fix `PASSED` | **REFUTED** | P1 |
| A11 | 65 MiB file (byte budget) | refused `GIT_OUTPUT_TOO_LARGE` (64 MiB cap), no receipt; pre-fix `PASSED` | **REFUTED** | P1 |
| A17 | publish after PASSED / after a refused run | lands exactly the receipt's tree; refused-run repo keeps its ref, `NO_RECEIPT` | **REFUTED** | P0 |
| A04/A12/A13/A15/A19 | in-tree WTE, symlink, gitlink, reserved/`..` via candidate, directory swap | typed refusals, no receipt | **REFUTED** | P1/P2 |

No regression appeared: the pre-fix build still `PASSED` A01/A02/A03/A05/A06/A10/A11 on the same
fixtures, and the post-fix build refreshed every one into a typed refusal.

## 2. Attempts to defeat the re-check

| Probe | Setup | Observed | Classification |
|---|---|---|---|
| (a) write-and-revert before the re-check | one obligation reads `a.txt`, writes `ALPHA\n`, reads it back (`seen_hex=414c5048410a`), then restores `alpha\n` | `PASSED`, receipt written, `checkout_recheck: "OK"` — the transient rewrite is **undetected** | **documented limit**: the re-check observes the file after the obligations; `doctor` states “a write reverted before the re-check … is not detected”. Confirmed, not a defect. |
| (b) extra directory created then deleted / left behind | obligation 1 creates `.tmp-cache/` and `.leftover/`; obligation 2 deletes `.tmp-cache/` only | `PASSED`, receipt, `checkout_recheck: "OK"`; the leftover is ignored | **documented by design**: the re-check re-proves *declared* files, not extra entries, because obligations legitimately write build outputs. Confirmed. |
| (c1) hook re-creates a declared directory component | tree has `nested/file.txt`; hook `mkdir -p nested` | `PASSED`, receipt, `checkout_recheck: "OK"` — no false positive | **control passed** |
| (c2) hook creates an undeclared child directory | same tree; hook `mkdir -p nested/extra` | `REFUSED/UNSUPPORTED_CHECKOUT`: `the checkout holds 1 directory(ies) the tree does not declare (for example ["nested/extra"])`, no receipt | **control passed** |
| (d) `chmod +x` a declared 100644 file during the obligations (Linux) | obligation `os.chmod("a.txt", 0o755)`; pinned Linux build of `7a034ff` | `REFUSED/CHECKOUT_MUTATED_DURING_RUN`: `entry "a.txt" declares mode 100644 but is executable`; no receipt. Clean control on Linux: `PASSED`, `checkout_recheck: "OK"`. Linux `git_fidelity` 18/18 | **control passed** (mode proof works) |
| (e) extra attempt: hook races the `.git` unlink and recreates a `.git` directory | hook waits `[ ! -f .git ]`, then `mkdir .git` + `HEAD`; obligation asserts `.git` is a directory | `PASSED`, receipt, obligation log `git_dir_seen=True` | **residual finding P3**, see §5 |

## 3. 122-scenario campaign on `7a034ff`

Command (output under `experiments/verify-p0/`, never overwriting the wave-2/wave-3 data):

```powershell
python experiments\mvp-qa\attacks.py `
  --flow "$env:TEMP\flow-7a-target\debug\flow.exe" `
  --release-flow "$env:TEMP\flow-7a-target\release\flow.exe" `
  --out experiments\verify-p0\results-campaign-7a.json `
  --markdown experiments\verify-p0\verdict-tables-7a.md `
  --base-sha 7a034ffb855ccd020f1abdd46034c7bd2c33d416
```

| | Raw run (`results-campaign-7a.json`) | Corrected W3-03 assertion (`results-w3-03-corrected.json`) |
|---|---|---|
| REPRODUCED | 7 — one is **W3-03, a harness false positive** | **6, same set as wave 3** |
| REFUTED | 114 | **115** |
| NOT_RUN | 1 (`PRF-09`) | **1** |
| ERROR | 0 | **0** |

The raw harness exited **1** with “1 reproduced P0: W3-03”, and the product behaviour in that
scenario is **correct**: the run was `REFUSED/UNSUPPORTED_CHECKOUT` with no receipt. The campaign's
`w3_hook_same_length_rewrite` asserts the literal substring `"different bytes"` in the refusal reason;
`7a034ff` made the message stage-aware (`entry "README.md" changed during checkout …`), so the
assertion misses and the scenario falls through to REPRODUCED. This is a **campaign string-coupling
defect, not a product regression** — exactly the kind of false verdict an independent re-run must not
forward. I did not edit `experiments/mvp-qa/attacks.py` (out of this round's write scope); instead
`experiments/verify-p0/w3_03_corrected.py` re-runs that one scenario with an assertion on the typed
refusal (`exit 1`, `code`, no receipt) and returns **REFUTED** (output above). The campaign owner
should replace the substring check with a code/status check.

With that correction the campaign is unchanged from wave 3: the same six are reproduced
(`FIL-20` P2, `PRC-10` P2, `PRF-01` P2, `PRF-02` P3, `PRF-08` P3, `CMP-01` P3), `PRF-09` is the one
NOT_RUN, no P0. Per-axis (raw): identity 20/0, scope 11/0, git 12/0, durable 11/0, files 23/1,
process 14/1, proof 9/3 + 1 NOT_RUN, compat 7/1, future 7/0, inputs 5/0, fidelity 3/1 (the false
positive; corrected 3/0).

The campaign contains no delayed-write scenario (W3-02/03 are synchronous), so its greenness is not
evidence about the re-check; §1/§2 are.

## 4. Documentation check (A18 re-check)

Measured `doctor` strings at `7a034ff` (verbatim):

- guarantee: `"the verification checkout equals the verified tree exactly when it is checked: every
  regular file byte for byte, executable bit on Unix, no missing file, no extra file or directory — and
  the declared files are re-proved unchanged after the obligations ran (a change refuses the run)"`
- non-guarantee: `"hooks are not suppressed: `git worktree add` runs post-checkout and `git update-ref`
  runs reference-transaction. A change already present when the checkout is checked is refused, and a
  change to a declared file while the obligations run is caught by a re-check after the last obligation;
  a hook that writes outside the checkout, a write reverted before the re-check, or a write after it is
  not detected. A blocking reference-transaction aborts publish"`

The non-guarantee matches the measured behaviour (a: reverted write undetected; A07: delayed write
caught by the re-check; outside writes not Flow's business). The guarantee now qualifies *when* the
checkout is proved (“exactly when it is checked”) and says declared files are re-proved after the
obligations — true. One literal caveat remains: “no extra file or directory” is false for a root
`.git` recreated by a hook after `materialise` unlinks it (probe (e)).

`docs/MVP-USER-GUIDE.md` (Hooks) and `docs/reviews/FLOW-PRODUCTION-INVARIANTS.md` (I25, I26, I24) now
describe behaviour that exists: the user guide's “After the last obligation, Flow re-proves every
declared file … (`CHECKOUT_MUTATED_DURING_RUN`) … a write reverted before that re-check or one that
happens after it [is not detected]” matches A07/(a); I26's `verify_checkout_unchanged` and
`checkout_recheck: OK` match code and receipts; I25's file+directory set matches (c2); I24's per-command
boolean sets match (probe `flags`: `run --dry-run`, `status --force`, `doctor --adopt-policy` → exit 2,
no receipt, no execution, ref unchanged).

**A18 is closed** for the statements that were wrong at `d7e1015`, with the `.git` caveat above.

## 5. Closed findings vs documented limits

**Closed at `7a034ff` (measured):**
- **A07 (P1)** — a delayed write to a declared file is now refused with `CHECKOUT_MUTATED_DURING_RUN`
  and no receipt is written. The obligation may still *see* the changed bytes (nothing is prevented),
  but no proof is issued, so the false-`PASSED` defect is closed. Reported as detection, not prevention.
- **A08 (P3)** — only the exact `.flow-scratch-owned` marker is Flow's; a `.flow-scratch*` file is
  refused.
- **A09 (P3)** — the directory set is compared; an empty undeclared directory is refused, while a
  declared component re-created by a hook is not a false positive.
- **A16 (P2)** — a non-UTF-8 tree path is refused with its own reason, not as a lossy case collision.
- **A18 (P2)** — `doctor` states detection-not-prevention and the write-and-revert limit; docs match code.
- **d7e1015 "fix absent"** — `7a034ff` lands the implementation; committed gates green (16/16 Windows,
  18/18 Linux).

**Residual finding (new, P3): root `.git` is still skipped by the entry walker.** `collect_entries`
skips a root entry named `.git` (the worktree link is removed by `materialise`), so a post-checkout
hook that waits for the unlink and recreates `.git/` is certified `PASSED`; probe (e) shows the
obligation seeing `git_dir_seen=True`. This is narrower than A08 (a race against `detach_git_link`, and
`scratch.rs` already documents that removing `.git` is “hygiene … not confinement”), but it
contradicts the `doctor` guarantee's literal “no extra file or directory”. Smallest fix: do not skip
`.git` in the walk (it should not exist at that point), or qualify the guarantee. Classified **minor
defect / documentation overstatement**, not a security regression.

**Documented limits (confirmed):**
- a write reverted before the re-check, or performed after it, is undetected (probe (a); `doctor`);
- extra entries created *during* the obligations are allowed (probe (b), by design for build outputs);
- writes outside the checkout are not Flow's business; no OS sandbox (`doctor`, `INVARIANTS` §6);
- an external `git` writer is not fenced (`INVARIANTS` §6);
- receipts can be forged by anyone who can write `.flow/receipts` (`doctor`).

## 6. Reproduce

```powershell
git archive 7a034ff -o %TEMP%\flow-7a.tar    # extract
# with CARGO_TARGET_DIR outside the repo:
cargo build --locked --offline ; cargo build --release --locked --offline

python experiments/verify-p0/p0_attacks.py `
  --post "%TEMP%\flow-7a-target\debug\flow.exe" `
  --pre  "%TEMP%\flow-prefix-target\debug\flow.exe" `
  --out  experiments/verify-p0/results-p0-7a.json

python experiments/verify-p0/recheck_probes.py `
  "%TEMP%\flow-7a-target\debug\flow.exe" `
  experiments/verify-p0/results-recheck-probes-7a.json

python experiments/verify-p0/recheck_defeat_probes.py `
  "%TEMP%\flow-7a-target\debug\flow.exe" `
  experiments/verify-p0/results-recheck-defeat-7a.json

wsl -e bash -lc 'cd /mnt/d/App/Ravum-flow && bash experiments/verify-p0/linux_recheck_probe.sh'
```

Files written this round (all new; no product source, tests or existing docs touched):
`experiments/verify-p0/{results-p0-7a.json, results-recheck-probes-7a.json,
results-recheck-defeat-7a.json, results-campaign-7a.json, results-w3-03-corrected.json,
verdict-tables-7a.md, p0-7a.log, campaign-7a.log, w3_03_corrected.py, linux_recheck_probe.sh}` and
this document (updated in place: it now pins `7a034ff` and records the previously pinned `d7e1015`).

Claim classification: **MEASURED** (binary observed), **CODE_READ** (source read), **PROPOSED** (fix
direction), **NOT_RUN** (none this round in the P0 matrix; campaign `PRF-09` remains NOT_RUN as before).
