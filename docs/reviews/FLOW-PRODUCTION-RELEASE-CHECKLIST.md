# Flow production candidate — release checklist (G7)

Nothing here publishes a release. The artefacts below are **private candidate** outputs attached to
the draft PR; `GO_PRODUCTION` is a separate human decision (§9).

## 1. Version and build

- Version: **`0.2.0-rc.1`** (`crates/flow/Cargo.toml`, `Cargo.lock`).
- Toolchain: `rustc 1.95.0` (stable), target `x86_64-pc-windows-msvc` (built here) and
  `x86_64-unknown-linux-gnu` (built by CI).
- Build: `cargo build --locked --release --offline` (no dependencies at all: `[dependencies]` is empty).
- Rebuilt from the delivered code head `997f983` after the second review, because the first artefact
  predated its P1/P2 fixes.

## 2. Artefacts produced on this host

`dist/flow-0.2.0-rc.1-x86_64-pc-windows-msvc/` (git-ignored, uploaded as a CI artefact):

| File | Size | SHA-256 |
|---|---|---|
| `flow.exe` | 1 033 216 B | `bbde18c555821d571803bad1a7b6448b05230115a148468af4c7eca50a18264a` |
| `LICENSE` | 17 100 B | copied verbatim from the repository |
| `SHA256SUMS.txt` | — | the line above |

Verification: `Get-FileHash flow.exe -Algorithm SHA256` / `sha256sum -c SHA256SUMS.txt`.

**Superseded artefact.** The first RC of this wave was `1 013 248 B`,
`d6edbec03a29e3477be0ae3fffe9b9fda6ac5934699acf8cc40df464438d19bc`, built **before** the second
review's fixes. It is kept in the record as history and must not be shipped: the P1
(`--dry-run=true` published for real) is only fixed in the binary above.


## 3. Install / uninstall smoke (executed)

```
copy flow.exe into an empty directory
flow version                      -> "flow 0.2.0-rc.1", schemas listed
git init; flow init --adopt-policy -> INITIALIZED
flow run                          -> PASSED
flow publish                      -> PUBLISHED
delete the directory              -> gone; no registry, no service, no data outside the repo
```

No administrator privilege is required, no permanent data is created outside `.flow/` in the
repository being used, and nothing is installed system-wide.

## 4. Release-binary behaviour checks (executed)

| Check | Result |
|---|---|
| `FLOW_TEST_CRASH=after-intent flow publish` on the **release** binary | `PUBLISHED`, exit 0 — the debug-only hook is inert |
| `flow version` | `0.2.0-rc.1`, `flow-receipt/2`, and all nine schemas (`flow-config/1`, `flow-receipt/2`, `flow-journal/1`, `flow-proposal/1`, `flow-candidate/1`, `flow-adoption/1`, `flow-plan/1`, `flow-status/1`, `flow-proposal-list/1`) |
| `flow doctor` in a fresh repo | `REPORT`, `overall: OK`, exit 0 |
| `doctor` with a torn journal | `overall: FAILED`, exit 1 |

## 5. Reproducible build

**`REPRODUCIBLE` on one host and toolchain**, on Windows `x86_64-pc-windows-msvc` with `rustc 1.95.0`.
Recipe: two clean `git archive HEAD` checkouts in two different directories, built sequentially with

```
RUSTFLAGS="-C link-arg=/Brepro -C codegen-units=1 --remap-path-prefix=$PWD/= " \
  cargo build --locked --release --offline
```

| Where | Archive a | Archive b | Verdict |
|---|---|---|---|
| this host | `9c7a7c372e5b76941ee5a4fc5caa4525cb80afaeec299493ad1262689bae807e` | identical | **reproducible** |
| CI `test windows-latest`, run `42739319204` | `409d8f853a6cbc4d61dbe74c35692dcdfec94f1d79382c44a5289b1f344e4313` | identical | **reproducible** |

What each flag removes was established by elimination, not assumed:

| Flags (this host) | Result |
|---|---|
| none | `6a608e71…` vs `748869ca…` (same size, different bytes) — **not** reproducible |
| `--remap-path-prefix` only | `60c90251…` vs `2bbda724…` — still not |
| `--remap-path-prefix -C codegen-units=1 -C link-arg=/Brepro` | **identical** |

So the divergence was the PE/PDB identity and codegen ordering, not the source paths alone.

**Scope of the claim, precisely:** two builds of the same commit *in the same environment* are
bit-identical. **Cross-host identity is not claimed** — this host produces `9c7a7c37…` and the GitHub
runner produces `409d8f85…`, which is the expected difference between two toolchain installations, and
no campaign was run to remove it. Linux is not claimed either: the linker flags differ.

CI repeats the check on every Windows run (`reproducible build (Windows, two clean archives)`). (The
earlier attempt at `c624c23`, without these flags, gave `978fefb3…` vs `30e958d7…`, which is why the
claim was withheld then.)

## 6. Supply chain

| Item | Result |
|---|---|
| Runtime dependencies | **none** (`[dependencies]` empty, `Cargo.lock` has one package: `flow`) |
| Dev dependencies | none; test tools are external (`cargo-llvm-cov`, `cargo-mutants` — see the test matrix) |
| `cargo audit --file Cargo.lock` | 0 vulnerabilities (1430 advisories loaded, 1 crate scanned) |
| GitHub Actions | every `uses:` is pinned to a commit SHA (`checkout@11d5960…` v4.4.0, `dtolnay/rust-toolchain@b725eda…` with `toolchain: 1.95.0`, `setup-node@49933ea…` v4.4.0, `setup-python@a26af69…` v5.6.0, `upload-artifact@ea165f8…` v4.6.2, `msvc-dev-cmd@0b201ec…` v1.13.0), with the release in a comment |
| CI permissions | `contents: read` only |
| Secrets in artefacts | none; fixtures use fictional canaries only |
| Licence | MPL-2.0, `LICENSE` shipped with the artefact; no third-party code was copied |

## 7. GitHub protections — **not configured, and not configured by this branch**

Verified state at G0: `dev` and `main` report 404 on the protection endpoint and `rulesets` is `[]`,
so **no branch protection is active**. This branch did not change that (admin action, out of mandate).

Required before production, by repository administration:

1. Ruleset/branch protection on `main` and `dev`: require a pull request, 1–2 independent approvals,
   required status checks `flow-ci / test (ubuntu-latest)`, `flow-ci / test (windows-latest)`,
   `flow-ci / polyglot (ubuntu-latest)`, `flow-ci / polyglot (windows-latest)`, dismiss stale
   approvals, block force-push and deletion.
2. `CODEOWNERS` for `crates/flow/**`, `.github/**`, `docs/reviews/**`.
3. ~~Pin the third-party Actions to commit SHAs~~ — done (`e660cdb`), keep them pinned on upgrade.
4. Decide whether `flow-ci` is required on the stacked PR chain or only on `dev`.

## 8. Rollback

The product touches only `.flow/` in the target repository and, on publication, one branch ref.

- **Rollback a publication**: `git update-ref refs/heads/<branch> <old-oid> <new-oid>` — the journal
  records the `old`/`new` pair for every publication, and `flow reconcile` never moves a ref.
- **Rollback the tool**: delete the binary directory. Nothing else is installed.
- **Rollback local state**: `.flow/` can be copied aside before upgrading; it is not read by Git and
  never enters a verified tree.

## 9. Decisions that remain human

| Decision | Why it blocks production |
|---|---|
| `GO_PRODUCTION` itself | the mandate explicitly excludes self-attribution |
| Branch protection / required checks | admin action, out of mandate |
| `H4` (confirmatory performance) | not required to ship, required for any productivity claim |
| `H3` (untrusted execution) | out of scope; this candidate is local-trusted only |
| Action SHA pinning | **done** (`e660cdb`); re-pin on every action upgrade |
| Release publication | out of mandate |
