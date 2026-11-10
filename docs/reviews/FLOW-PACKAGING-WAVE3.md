# Flow packaging — wave 3 (Windows + Linux)

**Status:** release-candidate artefacts produced, measured and documented. They are **not** a release:
nothing is signed, tagged or published, and `GO_PRODUCTION` remains a human decision.

**Built from:** commit `de86b3e3895bd9e566b5e77af06a755ae0c981f1`
(`feat/flow-production-candidate`, version `0.2.0-rc.1`, tree
`1ab82782c4c0c8d1ffd908714456e88713d88aca`). Every artefact came from a clean `git archive` of that
commit, never from the shared working tree. Product code is identical to `7a034ff`; `de86b3e` only
changes the Windows packaging script.

**What this closes:** the audit gap “no confirmed deliverable binary artefact” and the missing
provenance/SBOM, for two targets, with commands that reproduce them and a CI run that now produces
and uploads the same artefacts.

Claim tags follow `AGENTS.md`: **MEASURED** (executed here, output recorded), **CODE_READ**
(read in the source), **NOT_RUN** (not attempted).

---

## 1. Artefacts (local, MEASURED)

### 1.1 Windows — `dist/flow-0.2.0-rc.1-x86_64-pc-windows-msvc/`

| File | Bytes | SHA-256 |
|---|---:|---|
| `flow.exe` | 1 055 232 | `c5312c266ba6114e576efa41dbc599dbe2e3f2212ab9c29290f8545dedf1878a` |
| `LICENSE` | 17 100 | `9303ea1cfbf049945652668089d753fb1a7e1f2e52c680615c3d2cea7515c118` |
| `SBOM.json` | 1 090 | `f61f461e63281d78067e0092121efa04e86b6ff678d4e8724724d54552491609` |
| `SHA256SUMS.txt` | 149 | `e88f08152ec152a64829ec96defa2e74f1f051a6f69ce051ec2aa137e57fa066` |
| `PROVENANCE.json` | 6 249 | `1398ffc68943464923852b29da986bd6aa64072724e17934d96fe8436d531c59` |

### 1.2 Linux — `dist/flow-0.2.0-rc.1-x86_64-unknown-linux-gnu/`

| File | Bytes | SHA-256 |
|---|---:|---|
| `flow` | 1 188 368 | `859071eacd5932a95c9dbfacfeac352aa66598d47020c9ae54733ba0674eee67` |
| `LICENSE` | 16 727 | `66a3107d5ad6a058aab753eaac2047ccb2ed0e39465dd0fe5844da3e300d5172` |
| `SBOM.json` | 1 052 | `f31a2e46b9ce9ff3b5e610ba0f847bb3180e80c1ed0add767d78e1c47efd7931` |
| `SHA256SUMS.txt` | 145 | `ba9be6ca2855fccd91ea7d7dde19b31adac3563e0d4e058cd08f2213ed8591fb` |
| `PROVENANCE.json` | 5 822 | `f4b81eaff3741b6f10bf2798408bc4552316af3cf3cf67a3fbddc2e462ec547b` |

`SHA256SUMS.txt` covers the binary and `LICENSE`; `PROVENANCE.json` covers those plus `SBOM.json`
and `SHA256SUMS.txt`. A file cannot list its own hash.

### 1.3 Source identity

| Field | Value |
|---|---|
| Commit | `de86b3e3895bd9e566b5e77af06a755ae0c981f1` |
| Tree OID | `1ab82782c4c0c8d1ffd908714456e88713d88aca` |
| Tree-manifest SHA-256 (506 entries) | `dd0ce22371ec15e8347fe30fc833a4a36d60075df1d8f70f72c715181ff5b066` |
| `git archive` SHA-256, Windows host (git 2.51.1) | `e691b573080490497b9368fcec218cbb465c6fc3047c6333930c54a29b4949a8` (9 912 320 B) |
| `git archive` SHA-256, WSL host (git 2.43.0) | `62d0b507c8d7ccc35507728b7b3e55da350ed4ba4cb6f7e76dacf0df780f47b6` (9 830 400 B) |

**MEASURED:** the two `git archive` hashes differ because the tar serialisation differs between the
Git installations; the *canonical* source identity (commit + tree OID + tree-manifest) is identical
on both. A consumer compares the tree OID, not a host-specific tar hash. The same two archive hashes
were produced by the CI jobs for this commit, which is the useful cross-check.

### 1.4 LICENSE size difference — line endings only (MEASURED)

The Windows `LICENSE` is 17 100 B and the Linux one 16 727 B. This is **not** a different licence:

- committed blob (`<commit>:LICENSE`) = 16 727 B, 373 lines, LF;
- shipped Windows file = 17 100 B, 373 CRLF pairs, 0 bare LF;
- shipped Linux file = 16 727 B, 0 CRLF, 373 LF;
- normalising CRLF → LF gives the **same** SHA-256 on both:
  `66a3107d5ad6a058aab753eaac2047ccb2ed0e39465dd0fe5844da3e300d5172`.

Cause: this host's Git has `core.autocrlf=true`, and the `git archive` extraction materialises text
files in working-tree form. `PROVENANCE.json` records the fact in `source.host_git_core_autocrlf`,
`source.archive_extraction_note` and a `license` block (`shipped_bytes`, `committed_blob_bytes`,
`committed_blob_oid`, `shipped_line_endings`, `normalized_text_sha256`). No consumer should read the
size difference as a licence difference.

### 1.5 CI-produced artefacts (same commit, different hosts)

CI run `42802527240` (push) and `42802536498` (PR) are **green**, including both packaging steps.
The uploaded artefacts were downloaded and inspected:

| Target | CI binary SHA-256 | Bytes | Local binary SHA-256 | Bytes |
|---|---|---:|---|---:|
| Windows | `cc790a2e8088a6b0ad9c3f692c1783ff64d2a68cfaafeb846c22054e81ad1eea` | 1 055 744 | `c5312c26…f1878a` | 1 055 232 |
| Linux | `0276bf2d18eb5d26a4e9d9c15139fe374e87fc67261e10f75037fde58845435c` | 1 188 368 | `859071ea…ee67` | 1 188 368 |

The CI artefacts carry the same commit, tree OID and tree-manifest hash, the same archive hashes,
and the same measured test counts (232 Windows / 233 Linux). The **binaries differ** because the
host toolchain installations differ (CI Windows Server 2025 Datacenter vs local Windows 11 Pro; CI
Ubuntu 26.04.1 with kernel 7.0.0-1019-azure vs local WSL Ubuntu 24.04.3), which is exactly the
cross-host difference this report does **not** claim to remove. The CI artefacts are valid within the
same per-host/per-toolchain scope; they are not byte-identical to the local ones. Linux happens to
have the same size but different bytes — size is not identity.

### 1.6 Superseded artefacts

- `dist/superseded/flow-0.2.0-rc.1-x86_64-pc-windows-msvc-997f983/` — kept, labelled, **not
  shipable**: built from `997f983`, `flow.exe` 1 033 216 B,
  `bbde18c555821d571803bad1a7b6448b05230115a148468af4c7eca50a18264a`, before the P0
  checkout-fidelity fix; no PROVENANCE/SBOM. See its `SUPERSEDED.md`.
- The `d7e1015` candidate produced **no artefact**: its suite was red (see §3.4) and the packaging
  script refused to package it.
- The `2f4814e` rehearsal artefacts were overwritten by the final build; their numbers are superseded
  and are not part of this deliverable.

---

## 2. Reproducibility (MEASURED, per host/toolchain)

### 2.1 Windows — **REPRODUCIBLE on this host**

Two clean archives of `de86b3e` in two directories, built sequentially, each with

```text
CARGO_TARGET_DIR=<private dir>
RUSTFLAGS=-C link-arg=/Brepro -C codegen-units=1 --remap-path-prefix=<archive dir>= 
cargo build --locked --release --offline
```

| Build | SHA-256 |
|---|---|
| archive a | `c5312c266ba6114e576efa41dbc599dbe2e3f2212ab9c29290f8545dedf1878a` |
| archive b | `c5312c266ba6114e576efa41dbc599dbe2e3f2212ab9c29290f8545dedf1878a` |
| verdict | **identical** |

The flag set is the one in `.github/workflows/flow-ci.yml`; the CI check itself also passed on this
commit. The extracted tree is CRLF for text files on this host (see §1.4), but both builds see the
same bytes, so the comparison is valid.

### 2.2 Linux — **REPRODUCIBLE on this host**

Two clean archives, same scheme, with the GNU-appropriate flags:

```text
RUSTFLAGS=-C codegen-units=1 --remap-path-prefix=<archive dir>= 
cargo build --locked --release --offline
```

| Build | SHA-256 |
|---|---|
| archive a | `859071eacd5932a95c9dbfacfeac352aa66598d47020c9ae54733ba0674eee67` |
| archive b | `859071eacd5932a95c9dbfacfeac352aa66598d47020c9ae54733ba0674eee67` |
| verdict | **identical** |

`-C link-arg=/Brepro` is MSVC-only and has no GNU equivalent; Linux does not claim to be built with
the same linker flags as Windows.

### 2.3 Exact scope

- Proved: same commit, same host, same toolchain (`rustc 1.95.0`), two clean archives → identical
  bytes, on **both** platforms.
- Not proved: identity across hosts, rustc installations, linkers, or between Windows and Linux.
  The CI/local comparison in §1.5 shows the expected difference, not a failure.
- The tree OID + tree-manifest is the portable identity; a binary hash travels only with the
  host/toolchain scope stated here.

---

## 3. Test evidence (on the packaged commit)

Command: `cargo test --locked --all-targets --offline` in a clean archive of `de86b3e`.

| Suite | Windows | Linux |
|---|---:|---:|
| `unittests src/main.rs` | 37 | 37 |
| `tests/bounds.rs` | 24 | 24 |
| `tests/cli.rs` | 55 | 54 |
| `tests/concurrency.rs` | 9 | 9 |
| `tests/crash_run.rs` | 6 | 6 |
| `tests/error_paths.rs` | 32 | 32 |
| `tests/git_fidelity.rs` | 16 | 18 |
| `tests/isolation.rs` | 27 | 27 |
| `tests/proposals.rs` | 18 | 18 |
| `tests/secrets.rs` | 8 | 8 |
| **total** | **232 passed / 0 failed / 0 ignored** | **233 passed / 0 failed / 0 ignored** |

**Difference and why:** `tests/cli.rs` is 54 on Linux because `a_cmd_shim_on_path_is_bare_program_name`
is Windows-only (PATH/PATHEXT shim resolution); `tests/git_fidelity.rs` is 16 on Windows because the
two Unix-only fidelity gates (`executable bit`, non-UTF-8 tree path) are not compiled there. The two
differences cancel: 232/233, both 0 failed and 0 ignored. This matches the integration run on the same
tree.

### 3.4 Release-gate event: the `d7e1015` red tree (recorded, not hidden)

The first “final” SHA offered for packaging was `d7e1015`. A clean archive of it **failed its own
suite**: Windows 3 failures / 232 executed, Linux 4 failures / 233 executed.

- `a_hook_file_named_like_a_scratch_marker_is_refused_not_certified` — run returned `PASSED`;
  `collect_files` skipped every root entry whose name starts with `.flow-scratch`,
  so a hook-created `.flow-scratch-injected.txt` was invisible.
- `a_hook_that_creates_an_empty_directory_is_refused_not_certified` — `collect_files` pushed only
  files, so an empty directory was invisible.
- `a_delayed_write_during_the_obligations_is_refused` — no post-run re-check existed;
  `CHECKOUT_MUTATED_DURING_RUN` appeared only in the test file, not in `crates/flow/src`.
- (Linux) `a_tree_path_that_is_not_valid_utf8_is_refused_with_its_own_reason` — the expected
  diagnostic was not implemented.

Cause: `bae7390` added the tests and docs but not the `crates/flow/src` implementation (it was
reverted during the red-first proof). The packaging script **refused to package** the red tree — the
guardrail worked. `7a034ff` landed the implementation; `de86b3e` is the final code SHA used here.
The event is recorded because silently rewriting the evidence would be dishonest.

---

## 4. Clean-room smoke matrix (both platforms, MEASURED)

Both binaries were copied to a fresh install directory and invoked **by absolute path** with a
scrubbed environment (`env -i` on Linux; reduced `PATH`/`HOME`/`TMP` and no `FLOW_*` on Windows).
Every repository was created from scratch; no repository and no `.flow/` was reused.

Common setup: `git init -q -b main`, local fictional identity, seed commit with `a.txt`; policy
obligation `git --version` (required, 30 s).

| # | Case | Command | Exit | Key JSON | Windows | Linux |
|---|---|---|---:|---|---|---|
| 1 | version | `flow version` | 0 | `status=VERSION`, `tool="flow 0.2.0-rc.1"`, `schema="flow-receipt/2"` | PASS | PASS |
| 2 | init | `flow init` | 0 | `status=INITIALIZED` | PASS | PASS |
| 3 | adopt policy | `flow init --adopt-policy` | 0 | `status=INITIALIZED`, `adopted=true` | PASS | PASS |
| 4 | run (green) | `flow run` | 0 | `status=PASSED`, `obligations[0].status=PASSED` | PASS | PASS |
| 5 | publish | `flow publish` | 0 | `status=PUBLISHED`, `ref=refs/heads/main` | PASS | PASS |
| 6 | missing config | `flow run` (no `.flow/`) | 1 | `status=ERROR`; Windows `error="not initialized: cannot read …\.flow\config.json: … (os error 3)"`; Linux `"…: No such file or directory (os error 2)"` | PASS | PASS |
| 7 | corrupt config | `flow run` (`.flow/config.json` = `{not json`) | 1 | `status=ERROR`, `error="expected key at 1"` | PASS | PASS |
| 8 | unapproved policy | `flow run` (config present, never adopted) | 1 | `status=REFUSED`, `code=POLICY_NOT_ADOPTED`, reason “this working copy has no adoption record…” | PASS | PASS |
| 9 | missing obligation program | `flow run` (`program="flow-definitely-missing-command-xyz"`) | 1 | `status=UNSUPPORTED`, `obligations[0].status=UNSUPPORTED`, stderr `program not found` — **never PASSED** | PASS | PASS |
| 10 | read-only directory | `flow init` in a write-denied repository root | 1 | Windows `status=ERROR`, `error="cannot create receipts dir: Accès refusé. (os error 5)"`; Linux `error="cannot create receipts dir: Permission denied (os error 13)"` | PASS | PASS |
| 11 | release binary ignores the test hook | `FLOW_TEST_CRASH=after-intent flow publish` after a green run | 0 | `status=PUBLISHED`; not exit 89, no crash | PASS | PASS |
| 12 | installed copy is self-contained | all commands through `<install>/flow[.exe]` | — | install directory contains only `flow` / `flow.exe` | PASS | PASS |
| 13 | no scratch left | after all runs | — | no `flow-scratch*` remains in the audited temp dir | PASS | PASS |
| 14 | nothing outside expected dirs | audit | — | no writes to `%APPDATA%\flow`, `%LOCALAPPDATA%\flow`, `%USERPROFILE%\.flow`, `~/.flow`, `~/.config/flow`, `/usr/local/bin/flow`, `/usr/bin/flow` | PASS | PASS |

Windows OS errors are localised (French system): `Accès refusé` = access denied; the Linux run is
the English equivalent. Both are quoted verbatim. **Result: 14/14 checks on each platform, 0 smoke
failures.**

### 4.1 What the smoke run created

Only inside the throwaway fixture root:

- the install directory: exactly the copied binary;
- each test repository: its `.git/` plus, where the command succeeded,
  `.flow/{config.json,adopted.json,journal.jsonl,.gitignore}`,
  `.flow/receipts/<tree>.json` and `.flow/logs/<tree>/{<id>.stdout.log,<id>.stderr.log}`;
- the OS temp directory: `flow-scratch-<pid>-<n>` during `run`, removed when the run ends.

Audited and empty after the run: install-directory additions, leftover scratch directories, and all
outside candidates in row 14. 223 files were created under the disposable repository root on both
platforms (Git objects + the Flow state above; not 223 Flow files).

Raw matrices: `wt-release-target/smoke-win-de86b3e.json`, `wt-release-target/smoke-linux-de86b3e.json`.

---

## 5. How to rebuild and verify

### 5.1 Windows (PowerShell, from the repository root)

```powershell
# build twice, run the full suite, compare, package, write SBOM + PROVENANCE
pwsh -NoProfile -File scripts/release/build-windows.ps1 `
  -Sha de86b3e3895bd9e566b5e77af06a755ae0c981f1 `
  -WorkRoot D:\App\Ravum-flow\wt-release-target

# a CI step that already ran the suite uses the existing log and skips a second run:
pwsh -NoProfile -File scripts/release/build-windows.ps1 `
  -Sha "$env:GITHUB_SHA" -TestLog test-output.txt

# verify
cd dist\flow-0.2.0-rc.1-x86_64-pc-windows-msvc
sha256sum -c SHA256SUMS.txt          # Git Bash
Get-FileHash flow.exe -Algorithm SHA256

# clean-room smoke (not part of CI; CI has its own install smoke)
pwsh -File ..\..\scripts\release\smoke-windows.ps1 -Binary .\flow.exe -Root D:\tmp\flow-smoke
```

The script uses `System32\tar.exe` to extract `git archive -o` tar files: when it is launched from
Git Bash (as CI does), a bare `tar` is MSYS tar, which mangles a `C:\...` destination. If the caller
prefers a different extraction tool, the archive bytes and the source tree are unchanged.

### 5.2 Linux (WSL or CI, from the repository root)

```bash
# tests + two clean builds + reproducibility check + package + SBOM/PROVENANCE
bash scripts/release/build-linux.sh de86b3e3895bd9e566b5e77af06a755ae0c981f1

# CI: use the already-produced gated log, no duplicate suite
RUN_TESTS=0 TEST_LOG=test-output.txt bash scripts/release/build-linux.sh "$GITHUB_SHA"

# verify
cd dist/flow-0.2.0-rc.1-x86_64-unknown-linux-gnu && sha256sum -c SHA256SUMS.txt

# clean-room smoke
bash scripts/release/smoke-linux.sh dist/flow-0.2.0-rc.1-x86_64-unknown-linux-gnu/flow
```

The script refuses to package if the two builds differ or if the supplied/executed suite has a
failure (override only for diagnosis with `ALLOW_DIFF=1` / `ALLOW_TEST_FAILURE=1`; the artefact then
records the failure). `python3` is a build-time JSON writer only; the product has no dependencies.

### 5.3 Manual source extraction (the pipeline)

```bash
git -C /path/to/Ravum-flow archive <sha> | tar -x -C <clean-dir>
# Windows equivalent used by the script, immune to MSYS path conversion:
git -C <repo> archive -o <archive.tar> <sha>
C:\Windows\System32\tar.exe -xf <archive.tar> -C <clean-dir>
```

`git archive -o` produces the same tar stream as the pipe. Both extract the same commit; the archive
hash is recorded in PROVENANCE.

### 5.4 CI wiring (now applied in `.github/workflows/flow-ci.yml`)

The two steps run after `build release` and use the gated test log; `dist/` is uploaded per OS. The
committed invocation is:

```yaml
      - name: package the release artefact (Windows)
        if: runner.os == 'Windows'
        shell: bash
        run: pwsh -NoProfile -File scripts/release/build-windows.ps1 -Sha "$GITHUB_SHA" -TestLog test-output.txt

      - name: package the release artefact (Linux)
        if: runner.os == 'Linux'
        shell: bash
        run: RUN_TESTS=0 TEST_LOG=test-output.txt bash scripts/release/build-linux.sh "$GITHUB_SHA"
```

Both steps succeeded on `de86b3e` (runs `42802527240`, `42802536498`); the uploaded artefact hashes
are in §1.5. The first attempt on `7a034ff` failed on Windows in the extraction, which is why the
script now avoids MSYS tar.

### 5.5 Verify a document

```bash
python3 -m json.tool PROVENANCE.json >/dev/null   # valid JSON
python3 -m json.tool SBOM.json >/dev/null
# then compare `artifacts[].sha256` with `sha256sum -c`.
```

---

## 6. Distribution gaps (honest list)

1. **No signature.** Both artefacts are `unsigned: true`; there is no key, no minisign/GPG file and
   no Sigstore/SLSA attestation. SHA-256 checksums detect corruption, not substitution.
2. **No macOS / ARM builds.** Only `x86_64-pc-windows-msvc` and `x86_64-unknown-linux-gnu` were
   built and measured. No portability claim beyond these targets.
3. **SBOM is minimal and not officially validated.** It is CycloneDX 1.5-shaped: one application
   component, no dependencies (true: `Cargo.lock` has one package), MPL-2.0 read from `LICENSE`,
   hash of the binary. It was not checked with the official CycloneDX validator (unavailable
   offline), and there is no SPDX alternative.
4. **No release publication.** CI uploads `dist/` as a workflow artefact, but there is no tag, GitHub
   Release or signed manifest; that remains a human decision.
5. **Cross-host reproducibility not proved.** CI and this host produce different binaries from the
   same commit/tree (see §1.5) because the toolchain installations differ. Both are valid only within
   their own host/toolchain scope.
6. **`git archive` hashes are not portable across Git versions** (2.51.1 vs 2.43.0 measured here) and
   are affected by `core.autocrlf`. PROVENANCE records the tree OID/manifest so a verifier can use
   the stable identity.
7. **No OS package format.** The deliverable is a directory with a bare binary, not `.zip`/`.tar.gz`,
   MSI/DEB/RPM, or a container. There is no install/uninstall script beyond copying the binary.
8. **Test counts are commit-specific.** `PROVENANCE.json` records the counts measured for its own
   commit and must not be inherited by a different commit.
9. **CI and local artefacts are not the same bytes.** Anyone verifying a download should use the
   matching `SHA256SUMS.txt`/`PROVENANCE.json`; the local hashes in §1.1–1.2 and the CI hashes in
   §1.5 are different by design.

---

## 7. Claim classification

| Claim | Class | Evidence |
|---|---|---|
| Artefact bytes and SHA-256 values | MEASURED | `Get-FileHash` / `sha256sum -c`, §1 |
| Windows two-build reproducibility | MEASURED | two clean archives, identical `c5312c26…` |
| Linux two-build reproducibility | MEASURED | two clean builds, identical `859071ea…` |
| Test totals 232/233, 0 failed, 0 ignored | MEASURED | `cargo test --locked --all-targets --offline` on clean archives |
| The 55/54 and 16/18 suite differences | MEASURED + CODE_READ | test logs; platform-only tests |
| Smoke matrix rows 1–14 | MEASURED | `scripts/release/smoke-*.{ps1,sh}` outputs |
| Release binary ignores `FLOW_TEST_CRASH` | MEASURED + CODE_READ | crash-hook case; `crash_point()` is `cfg!(debug_assertions)` |
| Scratch is created under the OS temp dir and removed | MEASURED + CODE_READ | audit; `scratch.rs` |
| CLI exit/status contract | CODE_READ | `main.rs` `exit_code_for` |
| CI packaging steps pass and upload `dist/` | MEASURED | runs `42802527240`, `42802536498` |
| CI and local binaries differ by host/toolchain | MEASURED | §1.5 hashes and host fields |
| Cross-host reproducibility | NOT_RUN | not attempted |
| Signing / attestation | NOT_RUN | no key exists |
| macOS / ARM builds | NOT_RUN | not attempted |
| Official CycloneDX validation | NOT_RUN | validator unavailable offline |
| Publication / tag / release | NOT_RUN | out of mandate |

---

## 8. Pointers

- Build scripts: `scripts/release/build-windows.ps1`, `scripts/release/build-linux.sh` (committed;
  convenience tooling, not part of the product).
- Smoke scripts: `scripts/release/smoke-windows.ps1`, `scripts/release/smoke-linux.sh` (doc/QA only;
  CI keeps its own install smoke).
- Release checklist (unchanged): `docs/reviews/FLOW-PRODUCTION-RELEASE-CHECKLIST.md`.
- Superseded artefact: `dist/superseded/flow-0.2.0-rc.1-x86_64-pc-windows-msvc-997f983/SUPERSEDED.md`.
- CI artefacts for `de86b3e`: downloaded under
  `wt-release-target/ci-artifacts-de86b3e/` for the §1.5 comparison.
