# Architecture candidates — all PROPOSED
**No components implemented in Ravum Flow as of repository bootstrap.** Historical Ravum prototypes are not automatically transferable. See [V6 §3–5](reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md).

## Options and burden of proof
| Option | Mechanism | Principal caveat |
|---|---|---|
| O0 | Existing Git + private copies/worktrees as needed + serialized integration | May already solve the problem at lower maintenance cost |
| O1 | Existing jj/GitButler/agent tools | Compare exact behavior, offline support, license and security, not marketing |
| O2 | Thin managed patch protocol + frozen Git tree + receipts + Git publisher | Only clients using managed protocol receive write coordination guarantees |
| O3 | O2 with disposable isolated execution views for compiler, hooks and unmanaged tools | Disk/RAM and lifecycle complexity to benchmark |
| O4 | VFS/CRDT/remote/distributed mechanisms | Deferred, new proof/OS/enterprise gates needed |

**A single physical directory cannot provide distinct file content simultaneously to uncontrolled external processes without a projection/isolation layer.** A watcher can detect but cannot prevent previous overwrites.

## Candidate data flow
```text
agents / humans / IDE (any language or provider)
     -> attributed proposals (base version, paths, binary bytes, modes)
     -> local control plane (admission, concurrency, journal)
     -> Git candidate tree prepared from expected base + selected changes
     -> approved Git hooks in disposable isolated work directory
     -> final Git tree frozen after all approved content-changing hooks
     -> language-native isolated verifier (tree + BuildContextId + PolicyVersion)
     -> VerificationReceipt tied to CandidateGitTreeOid
     -> durable intent + writer lock + epoch check + git update-ref old -> new
     -> non-destructive working-tree reconciliation / recovery
     -> optional forge CI at exact merge-result tree/ref
```
Do not assume the hook protocol is solved; replay of non-idempotent hooks and non-staged changes require an explicit policy.

## Authority domains
- Git refs/trees: published history (local). Remote forge is a separate eventual projection.
- Durable journal: intentions and recovery outcomes; not a competing source of code truth.
- Candidate Git tree: exact content being qualified.
- User working directory: potentially dirty/untracked state, never a verification receipt.
- BuildContextId: toolchain, dependencies, flags, platform, relevant env, filesystem inputs, sandbox policy; unknown inputs limit receipt reuse.
- Ravum runtime: uniquely owns `ActiveVersionId`, code activation, rollback, DSP safety and language proofs.

## Candidate state machine
`RECEIVED -> ADMITTED -> CANDIDATE_ASSEMBLED -> APPROVED_HOOKS_APPLIED -> CANDIDATE_FROZEN -> CHECKS_PENDING -> VALIDATED_FOR_SCOPE -> PUBLICATION_INTENT_DURABLE -> REF_PUBLISHED -> RECONCILED`.

Side states: `CONFLICT`, `FAILED`, `UNKNOWN`, `STALE`, `TIMEOUT`, `UNSUPPORTED`, `MANUAL_REQUIRED`, `ABORTED`, `NEEDS_RECONCILIATION`. `PASSED` is not `PROVED`.

## Hooks/identity ordering: high-risk Ravum reference finding
Historic Ravum `git_writer.rs` at `5408d9b3…` writes tree **before** hooks, calls `prepare-commit-msg` before `pre-commit`, runs hooks in the actual repository root, and passes inherited environment. These are **CODE_READ** observations, not executed demonstrations. Do not copy implementation into Flow; scope future tests HOOK-02a/b/c, TREE-01, ENV-04. Standard commit hook order: `pre-commit -> prepare-commit-msg -> commit-msg -> post-commit` (Git docs).

## Security/recovery
- Trusted vs untrusted processes have separate sandbox/permissions policies.
- `env_clear` or an allowlist is insufficient without OS process/FS/network isolation.
- Git ref CAS gives compare-on-commit, **not** atomic update of SQLite+filesystem or fencing epoch.
- For lease fencing, owner-local exclusive writer lock spans durable epoch check, ref CAS, outcome; external `git` bypasses this lock and must still be detected.
- An idempotent recovery journal distinguishes pre/post-ref-CAS, missing outcome, and dirty working tree; no `reset --hard` or silent human data loss.
- Binary bytes, modes, symlinks, case, renames, LFS, submodules, ignored build inputs are explicitly in scope or refused.

## Decision needed before architecture freeze
H1–H6 in [decision register](DECISION_REGISTER.md). Favor **candidate O3** for evaluation, not as accepted fact, unless O0/O1 wins empirical cost/correctness.