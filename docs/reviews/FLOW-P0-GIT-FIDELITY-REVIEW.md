# FLOW P0 checkout-fidelity fix — independent adversarial review

**Date:** 2026-11-10 (Europe/Paris) · **Reviewer:** `verify-p0` (independent; did not write the code)
**Branch:** `feat/flow-production-candidate` · **Lead-named commit:** `dfb941db8b22d3340ad873950113fcdf03166f81`
· **Head at measurement:** `5077c508b00c9e6b2062ec4a68e166c265140547`
· **Head at report time:** `ccdbec96e66533deeb322017400567013a6dff0b` — since `5077c50` the branch only added
tests (`fc80af3` local-smudge red-first gate, `ccdbec9` error-path tests); **no `crates/flow/src/**`
change**, so the measured binary is still the code under review
· **Harness:** [`experiments/verify-p0/p0_attacks.py`](../../experiments/verify-p0/p0_attacks.py) (Python stdlib only, disposable fixtures)
· **Machine-readable results:** [`experiments/verify-p0/results-p0.json`](../../experiments/verify-p0/results-p0.json)

> **Branch moved during the review.** The Lead named `dfb941d` ("fix: prove the verification checkout
> equals the verified tree"). While this review was running, the branch advanced to `2f4814e` and then
> `5077c50` ("fix: verify the executable bit of the verification checkout on Unix"). The canonical
> measurements below use binaries built from `git archive 5077c50` (immutable content, not the working
> tree). The `dfb941d..5077c50` delta on the fidelity path is `#[cfg(unix)]` only (executable bit) plus
> `gitcli::redact` changes; the Windows behaviour measured here is the same code. The earlier `dfb941d`
> binaries (`b023024e…` debug, `38a01662…` release) produced the identical 19-scenario verdict set.

## 1. Claim under test

> "the obligations saw exactly the bytes the verified tree stores, and nothing else" — product model
> summarized in the task; `flow run` materialises the verified tree with `git worktree add`, checks it,
> runs obligations there, and writes a receipt binding the tree OID + effective policy digest.

The fix under review (`crates/flow/src/commands.rs`, function `verify_checkout_fidelity` /
`verify_checkout_content`):

1. removes the "no conversion looks possible" shortcut, so **every** regular file is compared with its
   tree blob, unconditionally (entry budget 20 000, retained-bytes budget 64 MiB, both **refuse**, never
   skip);
2. rejects files present in the checkout that the tree does not declare;
3. discloses local attribute sources (`.git/info/attributes`, `core.attributesFile`) in `doctor`;
4. adds `crates/flow/tests/git_fidelity.rs` (6 end-to-end tests + a positive control).

## 2. Method and binaries

| Role | Provenance | SHA-256 |
|---|---|---|
| post-fix (canonical) | `git archive 5077c50` → `cargo build --locked --offline`, debug | `69622f4936e2478fda0dcb2a927b0e4aef605bda96d03a1ea48abfe20d6f1fc4` |
| post-fix (release) | same archive, `--release` (used for the campaign's `PRC-14`) | `3cb65065d04a86d698de1b9b435ecbe6baa41d3c422c4c4d42659ce25e49163f` |
| **pre-fix baseline** | `git archive 0de8c21` (parent of the fix) → same flags, debug | `2556febb27ac6f75c94aa5a641dce1cf14eed26c7ff2958d9d618ef8f4cc52ce` |
| fixture smudge filter (same-length upper-caser) | `experiments/verify-p0/smudge.rs` | `d452b152…` |
| fixture clean pass-through | `experiments/verify-p0/passthru.rs` | `8e5a307b…` |

Every scenario runs the real binary on a disposable repository; the pre-fix build is run on the *same*
fixture after the post-fix run, so "pre = PASSED, post = REFUSED" is a measurement of the patch, not a
claim. 19 scenarios were executed on Windows 11 (Git 2.51.1.windows.1, Rust 1.95.0); none is `NOT_RUN`.

Reproduce: `python experiments/verify-p0/p0_attacks.py --post <5077c50-debug> --pre <0de8c21-debug> --out experiments/verify-p0/results-p0.json`
(single scenario: `--only A07`).

## 3. Verdict

**15 / 19 attacks REFUTED. 4 findings reproduced** (`A07` P1, `A18` P2 documentation, `A08`/`A09` P3).
The P0 class named in the patch is closed on Windows and Linux (measured). The residual findings are
outside the byte comparison: a **post-check TOCTOU**, two **name/directory blind spots** in the
extra-entry check, and a **doctor sentence that overstates detection**. No way was found to make
`flow publish` land a tree the receipt does not name (short of the documented receipt forgery).

| ID | Attack | Expected | Observed | Verdict | Sev |
|---|---|---|---|---|---|
| A01 | `.git/info/attributes` → `filter=evil`, same-length smudge, `required=true` | refuse, no receipt | post `REFUSED/UNSUPPORTED_CHECKOUT` (6 vs 6 bytes); **pre-fix `PASSED`** | **REFUTED** | P0 |
| A02 | `core.attributesFile` outside the repo → same filter | refuse | post refused (6 vs 6); pre `PASSED` | **REFUTED** | P0 |
| A03 | `.git/info/attributes` → `text eol=crlf` | refuse | post refused (7 vs 6); pre `PASSED` | **REFUTED** | P0 |
| A04 | committed `.gitattributes` `working-tree-encoding=UTF-16` | refuse | refused (14 vs 6 bytes) | **REFUTED** | P1 |
| A05 | tracked `post-checkout` adds `injected.txt` | refuse | refused: "1 file(s) the tree does not declare"; pre `PASSED` | **REFUTED** | P1 |
| A06 | tracked `post-checkout` rewrites `a.txt` at equal length | refuse | refused (6 vs 6); pre `PASSED` | **REFUTED** | P0 |
| A07 | `post-checkout` spawns a **delayed** same-length rewrite after the check | must not certify unseen bytes | **PASSED**, receipt written, obligation log `content_hex=414c5048410a` | **DEFECT_REPRODUCED** | P1 |
| A08 | hook creates `.flow-scratch-injected.txt` + `.flow-scratch-dir/payload.txt` | refuse (nothing else) | **PASSED**; obligation saw both | **DEFECT_REPRODUCED** | P3 |
| A09 | hook creates an empty `.build-cache/` directory | refuse (nothing else) | **PASSED**; obligation saw the directory | **DEFECT_REPRODUCED** | P3 |
| A10 | 20 001 injected paths → 20 002 regular files | refuse above 20 000 | refused with the entry-budget message (36 s); pre `PASSED` (52 s) | **REFUTED** | P1 |
| A11 | one 65 MiB file | refuse (fail closed) | refused `GIT_OUTPUT_TOO_LARGE` (64 MiB cap); pre `PASSED` | **REFUTED** | P1 |
| A12 | symlink tree entry (`120000`) on `core.symlinks=false` | refuse | refused (checked out as regular file) | **REFUTED** | P1 |
| A13 | gitlink tree entry (`160000`) | refuse | refused (entry missing from checkout) | **REFUTED** | P1 |
| A14 | positive control: `100755` file, faithful checkout | `PASSED` + receipt | `PASSED` + receipt | **REFUTED** | P2 |
| A15 | `NUL`, `CON`, `..`, `.git`, `x.`, `a\b.txt` via `mktree` + shipped candidate record | refuse | all six `SCRATCH_REFUSED` (git refuses the invalid path), no receipt | **REFUTED** | P1 |
| A16 | two invalid-UTF-8 tree paths | refuse or compare both | refused by the **case-fold gate** ("2 paths that differ only by case" over the lossy `bad�.txt`), no receipt | **REFUTED** | P2 |
| A17 | publish after a PASSED run / after a refused run | lands only the receipt tree; `NO_RECEIPT` otherwise | tree matched exactly; refused-run repo kept its ref | **REFUTED** | P0 |
| A18 | `doctor` disclosure vs measured A07 | must not overstate detection | says a post-checkout change is "detected and refused"; A07 is certified | **DEFECT_REPRODUCED** | P2 |
| A19 | hook replaces a declared file with a directory | refuse | refused (presence gate, `UNSUPPORTED_CHECKOUT`) | **REFUTED** | P1 |

## 4. Findings

### F1 — P1: post-check TOCTOU; the fidelity check is one-shot and unobservable afterwards (A07)

**Setup.** Tracked `post-checkout` (`core.hooksPath=.hooks`):

```sh
#!/bin/sh
( sleep 6; printf 'ALPHA\n' > a.txt ) >/dev/null 2>&1 &
```

The tree blob of `a.txt` is `alpha\n` (6 bytes). The policy runs:

```sh
python -c 'import time,pathlib; time.sleep(9); d=pathlib.Path("a.txt").read_bytes();
print("content_hex="+d.hex()); raise SystemExit(0 if d==b"ALPHA\n" else 9)'
```

**Observed.** `flow run` → exit 0, `"status": "PASSED"`, receipt written; the obligation's own log is
`content_hex=414c5048410a` (`ALPHA\n`). `verify_checkout_fidelity` ran ≈ 0.2 s after `git worktree add`
returned, before the background writer fired; nothing re-reads the checkout afterwards. Reproduced on
Linux too (§7). This falsifies the product sentence for a *delayed* writer.

**Is it detectable by the user?** No, not from the proof: the receipt binds `tree_oid`, `commit_oid`,
`policy_digest`, obligation statuses/log previews; it records no checkout digest at obligation time;
`worktree_diff` is a diff of the *source repository's* worktree against the tree, not of the scratch;
the scratch directory is removed when the run ends (`Scratch::drop`). The only observables are external
(monitoring the scratch path, an OS audit log, a lock). A user cannot tell A07's receipt from a faithful
one.

**Smallest fix direction (PROPOSED).**
1. *Honesty first:* qualify the `doctor` sentence and the module docs (this is F2), and add the TOCTOU
   limit to `non_guarantees`.
2. *Detection, not prevention:* re-run the same byte comparison after the obligations finish and record
   the result (e.g. `checkout_recheck: "OK" | "CHANGED"`, refusing `PASSED` on `CHANGED`). A writer can
   still time around both checks, but a delayed mutation during a real run becomes visible.
3. *Prevention* requires OS isolation or a read-only snapshot for the obligation phase; the crate
   documents that it has no sandbox, so that is a design change, not a patch.

### F2 — P2: `doctor` overstates the hook guarantee (A18)

`non_guarantees` says:

> "A post-checkout that changes the disposable checkout is detected and refused (bytes and file set),
> not prevented; a blocking reference-transaction aborts publish"

A07 measures a post-checkout change that is **certified**. The unqualified "is detected and refused"
is false for any change applied after the one-shot check. Suggested wording: "a change **already
present when the check runs** is detected and refused …; a process that writes **after** the check is
not detected (the receipt is a statement about the tree, not about the bytes at obligation time)."

### F3 — P3: `.flow-scratch*` and `.git` root names are skipped by the extra-entry check (A08)

`collect_files` skips root entries named `.git` or starting with `.flow-scratch` (the ownership marker
is `.flow-scratch-owned`). A hook can therefore add `.flow-scratch-injected.txt` and an arbitrary tree
under `.flow-scratch-dir/`; A08's obligation saw both and the run was `PASSED`. A `.git` **directory**
re-created after `detach_git_link` is skipped the same way (CODE_READ: `collect_files`, and the scratch
module already states that removing `.git` is not confinement).

Fix direction: skip only the exact marker name, at the root, and validate its content/identity (the
marker is written by `materialise` and its bytes are known); treat every other entry, including
directories, as part of the checkout. Reject any `.git` that reappears.

### F4 — P3: empty extra directories are invisible (A09)

`collect_files` enumerates files; an empty `.build-cache/` created by the hook is not in the expected
set and not in the present set, so the "nothing else" check passes. A09's obligation saw the directory.
Fix direction: derive the declared directory set from the tree paths and compare it with the walk (or
document that "file set" is what is checked; the doctor sentence says "bytes and file set", which is
accurate, while the code comment says "*exactly* the tree", which is not).

### F5 — P2: diagnostics for lossy path collisions are misleading (A16, CODE_READ/MEASURED)

Git collapsed both invalid-UTF-8 names to `bad�.txt`; Flow refused with "2 paths that differ only by
case". The refusal is correct (fail closed) but the reason is wrong: the collision is a lossy
UTF-8 rendering, not case folding. A distinct error would help operators.

## 5. What the fix measurably closes

On the same fixtures, the pre-fix build (`0de8c21`) **certified** and the post-fix build refused:

| Scenario | pre-fix | post-fix |
|---|---|---|
| A01 local filter, same length | `PASSED` | `REFUSED/UNSUPPORTED_CHECKOUT` |
| A02 `core.attributesFile` + filter | `PASSED` | refused |
| A03 local `eol=crlf` | `PASSED` | refused |
| A05 hook adds a file | `PASSED` | refused |
| A06 hook same-length rewrite | `PASSED` | refused |
| A10 20 002 regular files | `PASSED` (comparison skipped) | refused (budget) |
| A11 65 MiB tree | `PASSED` **without reading a byte** | refused (`GIT_OUTPUT_TOO_LARGE`, 64 MiB cap) |

This is the P0 class from the patch: a host-local attribute source or a synchronous hook mutation is no
longer certified. The new regression tests
([`crates/flow/tests/git_fidelity.rs`](../../crates/flow/tests/git_fidelity.rs)) pass on Windows at
`5077c50` (10/10, including the included `json` module tests) and on Linux (11/11, including the Unix
executable-bit test added at `5077c50`). At report time (`ccdbec9`) the file has 12 Windows tests, all
passing, including `a_local_smudge_filter_is_refused_not_certified` (committed as `fc80af3`) — the same
bypass as A01, now pinned as a red-first gate. Their module doc ("whatever the local Git configuration
and hooks do") is broader than the tests: they only exercise hooks that finish before the check.

## 6. `publish`: no bypass found

A17 measured that after a `PASSED` run `publish` lands a commit whose tree equals the receipt's tree
(`refs/heads/main^{tree} == tree`), and that after a **refused** run (`UNSUPPORTED_CHECKOUT`) publish
returns `NO_RECEIPT` and the ref does not move. CODE_READ of `cmd_publish`: the tree is recomputed from
the index (or the candidate record), a receipt must exist for *that* tree, the receipt integrity is
verified, the policy/context digests are recomputed from current state, and every required obligation
must be `PASSED`. A forged receipt is the documented non-guarantee ("anyone who can write
`.flow/receipts` can forge one"), not a gap in this fix.

## 7. Linux data point (WSL Ubuntu, rustc 1.95.0, git 2.43.0)

Command: `wsl -e bash -lc 'cd /mnt/d/App/Ravum-flow && bash experiments/verify-p0/linux_probe.sh'`
(builds the pinned head binary with `CARGO_TARGET_DIR=/tmp/flow-target-verify`).

| Probe | Result |
|---|---|
| committed `git_fidelity` tests | **11 passed / 0 failed** (includes `a_hook_that_changes_the_executable_bit_is_refused_not_certified`) |
| `.git/info/attributes` + `filter=evil` (`tr` same-length) | `REFUSED/UNSUPPORTED_CHECKOUT` (6 vs 6 bytes) — MEASURED |
| A07 TOCTOU variant | run exit 0, a `PASSED` receipt, obligation log `content_hex=414c5048410a` — MEASURED |

So the byte-comparison gate and the TOCTOU gap are platform-independent; the executable-bit gate is
Unix-only by design (Windows does not represent the bit; A14 shows a `100755` entry is still certified
on Windows because the mode has no execution meaning there).

## 8. Documentation accuracy (CODE_READ)

| Text | Assessment |
|---|---|
| `doctor` guarantee: "the verification checkout equals the verified tree exactly: every regular file byte for byte, no missing file, no extra file" | **OVERSTATED** literally: `.flow-scratch*`/`.git` root entries and empty directories are skipped (A08/A09). Accurate for ordinary files. |
| `doctor` non-guarantee: post-checkout change "is detected and refused" | **OVERSTATED** (A07/A18). |
| `verify_checkout_content` doc: "Every regular file is therefore compared, byte for byte" | Accurate for the `ls-tree`-declared regular files; the *extra* side is a scan with skips. |
| `verify_checkout_fidelity` comment: "The checkout must be *exactly* the tree" | Same overstatement as above. |
| `git_fidelity.rs` module doc: "exactly the verified tree, whatever the local Git configuration and hooks do" | Overbroad: covers synchronous hook effects only. |
| `scratch.rs` module doc: removing `.git` "only defeats accidental or casual repository discovery", no confinement | Accurate and honest. |

## 9. Limits of this review / not tested

- **No OS sandbox**: obligations are same-user processes and can read/write host paths; out of scope
  for the fidelity fix and documented (CODE_READ). A07 is a consequence of this design, not a new one.
- **Receipt forgery** remains possible for anyone who can write `.flow/receipts` (documented).
- **Adversarial filters that emit byte-identical output** are undetectable by design (the code comment
  says so); they are harmless for the content claim.
- **Hardlinks with identical bytes** at check time are indistinguishable but harmless; combined with a
  delayed writer they reduce to F1 (CODE_READ + A07).
- **Unreadable file between checkout and read**: `fs::read` failure propagates (`commands.rs`,
  `verify_checkout_content`) → `Err` → `UNSUPPORTED_CHECKOUT`, i.e. fail closed (CODE_READ; not raced).
- A10/A11 exercised the budgets at their edges; trees larger than 20 000 files or 64 MiB are refused by
  design and were not stressed further.
- No attempt was made to break `publish`'s CAS/journal (covered by the campaign, §B).

## 10. Files

Created by this review (no `crates/flow/src/**` edit, no commit):

- `experiments/verify-p0/p0_attacks.py` — 19-scenario harness (this document's data).
- `experiments/verify-p0/smudge.rs` / `smudge.exe`, `passthru.rs` / `passthru.exe` — fixture filters.
- `experiments/verify-p0/linux_probe.sh` — WSL data point.
- `experiments/verify-p0/results-p0.json`, `p0-run.log` — results and console log.
- this document.

Claim classification used above: **MEASURED** (binary behaviour observed), **CODE_READ** (source read,
not executed), **PROPOSED** (fix direction), **NOT_RUN** (not executable on this host; none in §3).
