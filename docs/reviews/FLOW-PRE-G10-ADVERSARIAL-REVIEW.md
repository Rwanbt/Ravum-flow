# Internal adversarial pre-review — 2026-11-15

**This is not `G10`.** `G10` requires a reviewer outside the team; this document records an internal
pre-review run by two separate agents that did not write the branch, to find defects before an external
reviewer spends time on it. It cannot close `G10`, and it does not promote the verdict.

Scope: branch `feat/flow-production-hardening-fable`, tree at head `7446e32` plus the fixes listed
below. Reviewers read the current tree and the diff `d8642a1..HEAD`, ran tests, and did not edit files.

## Verdicts

| Review | Verdict | Blocking findings at the time |
|---|---|---|
| Transactional contracts (FBL-001..013, publish, gc, lock) | `PASS WITH FINDINGS` | 2 (P1 probe approval, P2 publication of missing/swapped logs) |
| Claims and evidence (docs vs code, CI run ids, FBL matrix) | `PASS WITH FINDINGS` | 2 (README denial of `gc`, FBL-009 proof overstated) |

## Defects found and fixed on this branch

| ID | Sev | Finding | Fix and proof |
|---|---|---|---|
| P1 | P1 | `context_probes` were not part of `effective_policy`, so a probe added to a cloned `.flow/config.json` after `flow approve` executed during `run` without approval (a canary file was created). | probes are part of the approved digest and of `flow approve`'s output; `tests/cli.rs::an_unapproved_context_probe_is_never_executed` refuses with `POLICY_NOT_APPROVED` and asserts the canary is absent. |
| P2 | P2 | `publish` verified the receipt seal but not the log bytes it names; `explain` reported `LOG_INTEGRITY_INVALID` while `publish` still moved the ref. | `verify_receipt_logs` is shared by `explain` and `publish`; publication refuses `MISSING_LOGS` / `LOG_INTEGRITY_INVALID` before the intent. `tests/cli.rs::publish_refuses_swapped_or_missing_evidence`. |
| P3 | P2 | `gc --keep N` (no age policy) deleted every candidate and partial receipt: "no age policy" meant "expired". A just-assembled candidate became `UNKNOWN_CANDIDATE`. | candidates and partial receipts are collected only by an explicit age policy; `gc_keep_only_keeps_fresh_candidates_and_partial_receipts` and the extended gc suite. |
| P4 | P2 | `CAPABILITIES.md` claimed bounded retention while `.flow/blobs` and `.flow/proposals` are never collected, and `README.md` denied `gc` entirely. | both documents corrected; the retention claim is now scoped to receipts/logs/candidates. |
| P5 | P3 | 144 named gates was stale (the workflow requires 152), and the "differences are not skips" sentence contradicted the APFS gate that prints `SKIP`. | `MEASUREMENTS.md` corrected, including the caveat that `ignored=0` does not prove every counted test executed its body. |
| P6 | P3 | FBL-004 was stated `CLOSED` although a policy that declares no probe cannot see a toolchain switch behind an unchanged shim binary; FBL-009's proof named a test that does not drive `CANDIDATE_IMMUTABLE`. | FBL-004 is `PARTIAL` with its scope named; FBL-009 is `CLOSED (write path)` and the defensive branch is no longer counted as proof. |

## Residual findings (documented, not fixed here)

| ID | Sev | Finding | Why it stands |
|---|---|---|---|
| R1 | P3 | `gc` TOCTOU: it reads `runs/`, then scans, then deletes; a `run` that starts after the read creates its marker and logs while `gc` deletes the tree's log directory. | The window is milliseconds (materialise + fidelity check precede log writes), and a receipt naming a deleted log is now refused at publication. A full fix needs a lock `run` observes for its whole lifetime, which conflicts with `run` being long-lived; it is a design decision, not a patch. |
| R2 | P3 | `add_worktree` retry is bounded and residue-aware only through Git's own error: if a failed `worktree add` leaves a populated directory, the retry can fail differently and the run refuses. | Both tests use a shim that leaves no residue; a realistic residue test needs a real half-written worktree, which the Git race does not reproduce on demand. Documented as `NOT_VERIFIED`. |
| R3 | P3 | Host identity is the host-name hash (`flow-host/1`): two machines with the same host name are indistinguishable, so a shared `.flow/` can attribute another machine's dead PID to "this host". | Matches FBL-008's `PARTIAL`; a boot id or machine id is the fix, and multi-host `.flow/` is unsupported. The lock module comment now says "provably alive **on this host**". |

## Checked and found sound

- **FBL-001**: `put_blob` → `create_once(metadata)` hard link; the loser re-reads and compares
  `content_digest`; `patch_blob` must be `.flow/blobs/<2>/<digest>`; `load`/`assemble` re-hash and
  require `tree_oid` + `changes` to re-derive. Crashes leave orphan blobs or temp files, never mixed
  attribution. G1: 8 real processes × 15 rounds, 1 `ADMITTED` / 7 `IDEMPOTENCY_CONFLICT`.
- **FBL-002**: run-scoped log directories, receipt digests, swap detected by `explain` and now by
  `publish`.
- **FBL-003**: per-generation `lock.steal.<nonce>` via `create_new`, liveness re-checked before the
  removal, nonce re-read before unlink, foreign/host-less never stolen, claimed recovery waits
  `STEAL_GRACE` then `LOCK_UNKNOWN`.
- **Publish**: seal, tree, scope, `missing_required`, recomputed policy digest/context, per-obligation
  `PASSED`, candidate binding, intent → re-read → CAS → post-CAS observation; a lying `update-ref`
  becomes `NEEDS_RECONCILIATION` (shim tests).
- **FBL-006** strict decoder; lock-free index read.

## Independence statement

Both reviewers confirmed they did not author the branch and did not edit, create, commit or push any
file. Fixtures were created under the system temp directory and removed. Their reviews are internal
preparation; the `G10` slot remains open (see [the brief](FLOW-G10-INDEPENDENT-REVIEW-BRIEF.md)).
