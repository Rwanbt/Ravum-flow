# Security policy — Ravum Flow

## Supported versions

`flow 0.2.x` is the only line under qualification. It is a single self-contained binary with
**no runtime dependency**; older builds are not patched. Each supported platform is stated in
[docs/CAPABILITIES.md](docs/CAPABILITIES.md) with the evidence that qualifies it — a platform
that is not listed there is not supported, whatever builds elsewhere.

| Platform | Tested | Release artefact |
|---|---|---|
| Windows x86_64 (MSVC) | yes, CI + local | yes |
| Linux x86_64 (GNU) | yes, CI + local | yes |
| macOS (arm64) | yes, CI | **not packaged yet** |

## Reporting a vulnerability

Use **GitHub private vulnerability reporting** on `Rwanbt/Ravum-flow` (Security → Report a
vulnerability). Do not open a public issue for a vulnerability, and do not attach real
credentials, private repository contents or personal paths: a canary value is enough to show a
leak, and this repository's fixtures use fictional canaries only.

Please include the version, the OS, the exact command, and what you observed versus what the
documentation claims. A report that a documented **limit** was exceeded is a bug report, not a
vulnerability: the limits are listed below and in `flow doctor`.

**Response.** This is a single-maintainer project with no staffed security team. Reports are
acknowledged on a best-effort basis; there is no paid bounty and no guaranteed SLA. A finding
is published in `CHANGELOG.md` with the fix and, when relevant, a `SECURITY.md` update.

## What a receipt is, and is not

A `PASSED` receipt is a statement about **one Git tree, one effective policy digest and one
execution**, on the `local-trusted` boundary. It is **not**:

- **a signature.** The receipt carries an integrity digest that detects corruption and casual
  edits. Anyone who can write `.flow/receipts` can produce a receipt that verifies. Signing
  needs a key in a separate trust domain and is not implemented;
- **a sandbox.** Obligations run with the rights of the invoking user and a scrubbed, allowlisted
  environment. Untrusted code execution is `UNSUPPORTED`; do not run a policy you do not trust;
- **a hook suppressor.** The Git plumbing Flow calls may still run `post-checkout` and
  `reference-transaction`. Their effects on the verification checkout are *detected and refused*,
  not prevented;
- **an external-Git fence.** Flow serialises its own writers with a local lock and publishes with
  a compare-and-swap. An external `git update-ref` or an editor writing to the working tree
  bypasses both;
- **a complete build-context proof.** Toolchain, flags, lockfiles and generated inputs are
  recorded, not enumerated exhaustively.

## Documented limits (not vulnerabilities)

| Limit | Value | Why |
|---|---|---|
| Regular files compared | 200 000 per tree | bounded, streamed comparison; above it the run **refuses** |
| Content per `cat-file` request | 32 MiB | bounded capture, streamed chunk by chunk |
| One file | 48 MiB | the Git capture is capped at 64 MiB |
| Patch admitted | 32 MiB | proposal admission |
| Obligation output kept | 1 MiB per stream | receipt/log size |
| Ref name | 1 024 characters | Git's own practical bound |
| Unsupported tree shapes | gitlinks/submodules, case collisions, non-UTF-8 paths, escaping symlinks | refused, never repaired |
| Retention | **none** | `.flow/` grows with verifications; there is no GC, and deleting receipts by hand removes evidence |

`flow doctor` prints these non-guarantees for the repository it is run in. A receipt that names a
limit is not a defect; a receipt that **hides** one would be.
