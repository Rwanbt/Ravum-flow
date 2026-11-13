# R0-a — disposable fixtures and instrumentation (`GO_R0_FIXTURES`)

Purpose: give every R0-b experiment the same synthetic, reproducible ground truth, and prove the harness itself before trusting its results.

**Nothing here reads, writes or executes against `Rwanbt/Ravum`, a real user repository, or a real credential.** Every fixture is generated under `experiments/r0a/work/`, inventoried in `work/created-paths.json`, and removable only through this harness.

## Commands

```bash
python experiments/r0a/run_r0a.py control   # gate B: prove the instrumentation
python experiments/r0a/run_r0a.py build     # build F0–F8, write out/manifest.json
python experiments/r0a/run_r0a.py verify    # read-only drift check of a previous build
python experiments/r0a/run_r0a.py clean     # remove only inventoried paths
python experiments/r0a/run_r0a.py all       # control then build (the normal entry point)
```

Requirements: Python 3.11+, `git` on `PATH`. No network, no package install.

## Fixtures

| ID | Repository | What it contains |
|---|---|---|
| `f0-core` | mono-language Rust crate | 2 commits, dirty tracked file, untracked file, ignored log and ignored cache |
| `f1-concurrency` | 3 agent branches | disjoint edits (`a.txt`, `b.txt`) and a same-hunk conflict (`c.txt`) |
| `f2-hooks` | 3 variants | hook order log, `pre-commit` refusal, `pre-commit` that mutates index + worktree |
| `f3-filetypes` | 3 variants | binary, `100755`, `120000` symlink entry, rename, delete, `.gitattributes` CRLF, case collision |
| `f4-e2e` | Python API + consumer | `main` green E2E, `breaking` branch red E2E against an unchanged consumer |
| `f5-crash` | journal publisher | driven crash points: before/after intent, after `update-ref`, before outcome |
| `f6-canary` | probe | fictitious canary in a fake `~/.ssh` + one localhost network attempt |
| `f7-context` | declared-input build | declared flags/lockfile, undeclared env, ignored cache, `context_id` per run |

Each fixture records commit OIDs, tree OID, a canonical **tree digest** (mode+OID+path of every blob) and a **worktree digest** (bytes of every file). The manifest is `out/manifest.json`; it also records OS, Python, Git and toolchain versions and per-fixture build time.

## Gate B (control checks)

`run_r0a.py control` must pass before any R0-b run:

1. **determinism** — two independent builds of the same content produce the same tree OID and digest (commits differ, trees equal).
2. **tree vs worktree** — mutating a tracked file changes the worktree digest and *not* the tree digest; committing it changes the tree digest (the TREE-01 control).
3. **cleanup bounded** — a file placed outside `work/` survives `cleanup()`.
4. **redaction before persist** — a canary written through the logger is absent from the stored bytes.
5. **instrumentation** — the stopwatch measures a known 50 ms sleep, and resource usage is either measured or explicitly `NOT_MEASURED`.

## Measured limitations (do not paper over)

- Windows cannot store the executable bit or a symlink without elevation; those fixtures build the Git **tree** entry through the index/plumbing. The worktree is therefore *not* a faithful materialisation of a POSIX checkout — that asymmetry is itself a `FILE-09` observation.
- A case-insensitive worktree cannot hold both `Case.txt` and `case.txt`; the tree can. This is a `PATH-08` observation, not a harness bug.
- `ru_maxrss` from `RUSAGE_CHILDREN` is unavailable on Windows and is a cumulative high-water mark on POSIX; it is never presented as a per-run peak.
- `subprocess` timeout kills the direct child only (no job object / process group on Windows). `RESOURCE-14` measures that limitation instead of assuming it away.
- Builds are not byte-for-byte identical across machines when a toolchain embeds timestamps; the *Git tree* is the identity that must match.

## Results

- `out/control.json` — gate B checks with their detail.
- `out/manifest.json` — fixture inventory, OIDs, digests, environment, timings.
