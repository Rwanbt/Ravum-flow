# Flow production candidate — release checklist (G7)

Nothing here publishes a release. The artefacts below are **private candidate** outputs attached to
the draft PR; `GO_PRODUCTION` is a separate human decision (§9).

## 1. Version and build

- Version: **`0.2.0-rc.1`** (`crates/flow/Cargo.toml`, `Cargo.lock`).
- Toolchain: `rustc 1.95.0` (stable), target `x86_64-pc-windows-msvc` (built here) and
  `x86_64-unknown-linux-gnu` (built by CI).
- Build: `cargo build --locked --release --offline` (no dependencies at all: `[dependencies]` is empty).

## 2. Artefacts produced on this host

`dist/flow-0.2.0-rc.1-x86_64-pc-windows-msvc/` (git-ignored, uploaded as a CI artefact):

| File | Size | SHA-256 |
|---|---|---|
| `flow.exe` | 1 013 248 B | `d6edbec03a29e3477be0ae3fffe9b9fda6ac5934699acf8cc40df464438d19bc` |
| `LICENSE` | 17 100 B | copied verbatim from the repository |
| `SHA256SUMS.txt` | — | the line above |

Verification: `Get-FileHash flow.exe -Algorithm SHA256` / `sha256sum -c SHA256SUMS.txt`.

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
| `flow version` | `0.2.0-rc.1`, `flow-receipt/2`, schema list |
| `flow doctor` in a fresh repo | `REPORT`, `overall: OK`, exit 0 |
| `doctor` with a torn journal | `overall: FAILED`, exit 1 |

## 5. Reproducible build

Two clean `git archive` checkouts, same toolchain and flags, built sequentially:
`978fefb3e864af1d6f14e6713acea631d895802a541f6b7912f3ba2c14751ed8` vs
`30e958d7b88131b8afc880ec9cbc9c65cc044a429aa7e019caad7e7f41fe78cc`.

**`REPRODUCIBILITY_NOT_PROVEN`.** The divergence is consistent with embedded absolute build paths /
PE metadata; no `--remap-path-prefix` campaign was run in this wave. No bit-for-bit claim is made.

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
