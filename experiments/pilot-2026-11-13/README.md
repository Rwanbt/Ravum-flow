# Real-project pilot — 2026-11-13

Two questions the readiness audit left open, answered by execution on a real repository rather than
by a fixture:

1. does a whole cycle (verify → publish) hold on a **real project**, and what does it cost?
2. does the collaboration path work when **two agents** propose on one immutable base, and does a
   real collision get refused?

The numbers live in [`docs/MEASUREMENTS.md`](../../docs/MEASUREMENTS.md) §4 and in
[`results.json`](results.json); this file says what was run and how to reproduce it.

## What was run

The script clones a **disposable copy** of this repository (2 705 tracked files at the head that was
checked out), never the working tree, and then:

| Step | Command | What it proves |
|---|---|---|
| init | `flow init` then a 2-obligation policy (`cargo metadata`, `cargo fmt --check`) and `flow approve` | a real policy is adopted and listed by digest |
| cycle | `git add -A` then `flow run`, `flow publish --expected-old <HEAD>` | the index tree is verified in a disposable checkout and published by CAS |
| heavy | the same tree with `cargo test --bin flow` as a required obligation | a real, slow obligation is inside the receipt, not beside it |
| two agents | two clones, two disjoint new files, `flow propose` ×2 from one base, `flow assemble`, `flow run --candidate`, `flow publish --candidate` | attributed proposals → one exact candidate tree → published |
| collision | two proposals creating **the same path**, `flow assemble` | refused by name (`PATH_OPERATION_CONFLICT`), exit 1, nothing merged |
| envelope | a fresh repository with **25 000 files**, one no-op obligation, `flow run` + `flow publish` | the old 20 000-file budget would have refused; the streamed comparison certifies and publishes |

## Why `publish` is the interesting step

Two behaviours the first attempt surfaced, and that the pilot now encodes:

- **A cycle with nothing staged is `ALREADY_PUBLISHED`, not a failure.** `flow run` verifies the
  *index* tree, and a ref already carrying that tree has nothing to publish. The pilot stages one
  change first, which is what the documented workflow asks for anyway.
- **A patch that edits an existing file is refused when the host rewrites line endings.**
  `PATCH_REFUSED` came from a Windows `core.autocrlf` mismatch between the patch context and the
  stored blob. The pilot uses *new files* for the two-agent case so the measurement is about Flow,
  not about a host setting; the refusal itself is a real behaviour worth knowing.

## Reproduce

```powershell
cargo build --locked --offline --release
pwsh -NoProfile -File experiments/pilot-2026-11-13/run-pilot.ps1            # full, ≈ 6 min
pwsh -NoProfile -File experiments/pilot-2026-11-13/run-pilot.ps1 -SkipHeavy # skips the cargo test run
```

The script works in `%TEMP%\flow-pilot-<id>` and prints the directory, so a failed run can be
inspected. It writes `results.json` next to itself.

## What this does not prove

- Not a benchmark: one run per step, one host (Windows 11, 16 CPUs), one repository.
- Not the S9 productivity claim: no comparison arm was run against plain Git, worktrees, `jj` or
  GitButler. `H4` still fixes those thresholds, and the advantage over those systems remains a
  structural argument, not a measurement.
- The 25 000-file scenario uses a no-op obligation, so its 185 s is the cost of the *fidelity
  proof*, not of a build.
