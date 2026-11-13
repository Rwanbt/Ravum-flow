# Flow packaging — wave 3 (Windows + Linux)

**Status:** release-candidate artefacts produced, measured and documented. They are **not** a release:
nothing is signed, tagged or published, and `GO_PRODUCTION` remains a human decision.

**Built from:** commit `f5bb271ef7bfdb2d4adf429b31c4740d58c350ef`
(`feat/flow-production-candidate`, version `0.2.0-rc.1`, tree
`a870b34a894819e5f53da183b25171d812c5815e`). Every artefact came from a clean `git archive` of that
commit, never from the shared working tree. The product source is `d6d7ea9` (the one-line fix that
stops the fidelity walker from skipping a root `.git` entry) plus documentation; the previous
packaging head was `de86b3e`, so its artefacts are superseded by the ones below.

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
| `flow.exe` | 1 055 232 | `4fa86af5355ae7a2fa1b1bda617638c9df8051ca9ff587636c28eb3030966fb1` |
| `LICENSE` | 17 100 | `9303ea1cfbf049945652668089d753fb1a7e1f2e52c680615c3d2cea7515c118` |
| `SBOM.json` | 1 090 | `83a1a0ab477e3fd80798401cb472ee132bd7e3c3bad468afabb93953c65c8168` |
| `SHA256SUMS.txt` | 149 | `1a3cbc47c82b1b6b804ca73cb855452d21395fbf091b8239e70326139fc5512b` |
| `PROVENANCE.json` | 6 249 | `0127d572ef1d822f1bfa8a14eeefee82000df7463390d287595c3f29732ee633` |

### 1.2 Linux — `dist/flow-0.2.0-rc.1-x86_64-unknown-linux-gnu/`

| File | Bytes | SHA-256 |
|---|---:|---|
| `flow` | 1 188 368 | `ab8637b5fe7e2f957e6787b9aba7e4c84db8270b5e8a2492c7189351ac04b46d` |
| `LICENSE` | 16 727 | `66a3107d5ad6a058aab753eaac2047ccb2ed0e39465dd0fe5844da3e300d5172` |
| `SBOM.json` | 1 052 | `58cb27dd9249069e231089ea4a97693a546147c1541b5dd2dd0c1cdb24fafb56` |
| `SHA256SUMS.txt` | 145 | `4132e60f41812ea6067d37f05d265a65154244967bf449facc45c7dfb1cbe7ae` |
| `PROVENANCE.json` | 5 822 | `4ddd80b0fb73ab2785b36a273faa05b87fc5d7da1600b59e123b0f9d21bbcda4` |

`SHA256SUMS.txt` covers the binary and `LICENSE`; `PROVENANCE.json` covers those plus `SBOM.json`
and `SHA256SUMS.txt`. A file cannot list its own hash.

### 1.3 Source identity

| Field | Value |
|---|---|
| Commit | `f5bb271ef7bfdb2d4adf429b31c4740d58c350ef` |
| Tree OID | `a870b34a894819e5f53da183b25171d812c5815e` |
| Tree-manifest SHA-256 (571 entries) | `ebea6c9fd3316ed8d4db2023a771e5da8c5619a6edb73b418225cb962c3eb5ff` |
| `git archive` SHA-256, Windows host (git 2.51.1) | `6f59e14ba0f2cc43f681677d42d0c708178b557fea54adc4a5222923e89a701f` (23 889 920 B) |
| `git archive` SHA-256, WSL host (git 2.43.0) | `e0e1271c059cf380562ba43360405a1bd1488056df039155e8ab9e5fdcdd5f0d` (23 746 560 B) |

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

CI run `42803928091` (push) and `42803936581` (PR) are **green**, including both packaging steps.
The uploaded artefacts were downloaded and inspected:

| Target | CI binary SHA-256 | Bytes | Local binary SHA-256 | Bytes |
|---|---|---:|---|---:|
| Windows | `d8d5e45b4cf83470070c14bbd622a93f887e5142502543c18faf091c57bd0db2` | 1 055 744 | `4fa86af5…66fb1` | 1 055 232 |
| Linux | `26d90b6a5f87df996201a6f89d6d5a53f5a92a5d24b8f8f44221e1fa9aa0a84a` | 1 188 368 | `ab8637b5…b46d` | 1 188 368 |

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

Two clean archives of `f5bb271` in two directories, built sequentially, each with

```text
CARGO_TARGET_DIR=<private dir>
RUSTFLAGS=-C link-arg=/Brepro -C codegen-units=1 --remap-path-prefix=<archive dir>= 
cargo build --locked --release --offline
```

| Build | SHA-256 |
|---|---|
| archive a | `4fa86af5355ae7a2fa1b1bda617638c9df8051ca9ff587636c28eb3030966fb1` |
| archive b | `4fa86af5355ae7a2fa1b1bda617638c9df8051ca9ff587636c28eb3030966fb1` |
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
| archive a | `ab8637b5fe7e2f957e6787b9aba7e4c84db8270b5e8a2492c7189351ac04b46d` |
| archive b | `ab8637b5fe7e2f957e6787b9aba7e4c84db8270b5e8a2492c7189351ac04b46d` |
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

Command: `cargo test --locked --all-targets --offline` in a clean archive of `f5bb271`.

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
guardrail worked. `7a034ff` landed the implementation; `d6d7ea9` closed the reviewer's residual P3
(the walker no longer skips a root `.git` entry); `f5bb271` is the final code SHA used here.
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

Raw matrices: `wt-release-target/smoke-win-f5bb271.json`, `wt-release-target/smoke-linux-f5bb271.json`.

---

## 5. How to rebuild and verify

### 5.1 Windows (PowerShell, from the repository root)

```powershell
# build twice, run the full suite, compare, package, write SBOM + PROVENANCE
pwsh -NoProfile -File scripts/release/build-windows.ps1 `
  -Sha f5bb271ef7bfdb2d4adf429b31c4740d58c350ef `
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

**Record correction (explicit, as requested):** the commit message of `de86b3e` says the Windows
extraction was changed to `git archive --format=zip` + `Expand-Archive`. That is **not** what the
committed script contains: `de86b3e` and `f5bb271` both invoke the native
`C:\Windows\System32\tar.exe` on a `git archive -o` tar, and that is what CI measured and passed.
The zip description is a message/code discrepancy, not a second implementation; anyone reproducing
the artefact should use the tar path above.

### 5.2 Linux (WSL or CI, from the repository root)

```bash
# tests + two clean builds + reproducibility check + package + SBOM/PROVENANCE
bash scripts/release/build-linux.sh f5bb271ef7bfdb2d4adf429b31c4740d58c350ef

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

Both steps succeeded on `f5bb271` (runs `42803928091`, `42803936581`); the uploaded artefact hashes
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
| Windows two-build reproducibility | MEASURED | two clean archives, identical `4fa86af5…` |
| Linux two-build reproducibility | MEASURED | two clean builds, identical `ab8637b5…` |
| Test totals 232/233, 0 failed, 0 ignored | MEASURED | `cargo test --locked --all-targets --offline` on clean archives |
| The 55/54 and 16/18 suite differences | MEASURED + CODE_READ | test logs; platform-only tests |
| Smoke matrix rows 1–14 | MEASURED | `scripts/release/smoke-*.{ps1,sh}` outputs |
| Release binary ignores `FLOW_TEST_CRASH` | MEASURED + CODE_READ | crash-hook case; `crash_point()` is `cfg!(debug_assertions)` |
| Scratch is created under the OS temp dir and removed | MEASURED + CODE_READ | audit; `scratch.rs` |
| CLI exit/status contract | CODE_READ | `main.rs` `exit_code_for` |
| CI packaging steps pass and upload `dist/` | MEASURED | runs `42803928091`, `42803936581` |
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
- CI artefacts for `f5bb271`: downloaded under
  `wt-release-target/ci-artifacts-f5bb271/` for the §1.5 comparison.
