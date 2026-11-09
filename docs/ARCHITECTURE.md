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

## Candidate state machine (single canonical form — `X-05`)
`RECEIVED -> ADMITTED -> CANDIDATE_ASSEMBLED -> HOOKS_EXECUTED_ONCE -> CANDIDATE_FROZEN -> VERIFIED_FOR_SCOPE -> PUBLICATION_INTENT_DURABLE -> REF_PUBLISHED -> RECONCILED`.

Each state is entered only on **observable evidence**; an unobservable step is `UNKNOWN`, never an implicit success.

| State | Observable evidence required to enter | Still unknown at this state |
|---|---|---|
| `RECEIVED` | proposal id, author/agent, declared base Git OID | nothing assembled, nothing executed |
| `ADMITTED` | idempotency key accepted, writer lock held, obligations resolved | whether content applies to base |
| `CANDIDATE_ASSEMBLED` | candidate Git tree OID computed from base + proposal | whether hooks will change the tree |
| `HOOKS_EXECUTED_ONCE` | `hook_set_digest`, per-hook exit status, post-hook tree/index digests | whether a hook had external side effects |
| `CANDIDATE_FROZEN` | frozen tree OID — the only content identity used later | whether the tree is publishable |
| `VERIFIED_FOR_SCOPE` | receipt naming that tree OID + obligations + `BuildContextId` + `PolicyVersion` + execution boundary + real status | any undeclared build input ⇒ scope `UNKNOWN` |
| `PUBLICATION_INTENT_DURABLE` | fsynced intent record `{id, tree_oid, expected_old_ref_oid}` | whether the CAS will succeed |
| `REF_PUBLISHED` | ref re-read equals the expected new commit OID | whether the working tree reconciles |
| `RECONCILED` | working tree byte-identical (or explicit `NEEDS_RECONCILIATION`) | external Git bypass cannot be prevented |

Side states: `CONFLICT` (proposal does not apply), `STALE` (old OID diverged), `REFUSED` (policy denial, no mutation), `FAILED` (terminal, with error class), `UNKNOWN`, `UNKNOWN_OUTCOME` (CAS attempted, outcome unreadable — reconciliation required, never blind retry), `TIMEOUT`, `UNSUPPORTED`, `MANUAL_REQUIRED`, `ABORTED`, `NEEDS_RECONCILIATION`. `PASSED` is not `PROVED`.

**Deprecated aliases (do not reuse):** `APPROVED_HOOKS_APPLIED` → `HOOKS_EXECUTED_ONCE` (approval is not execution); `CHECKS_PENDING` → removed (a pending check is not a state that can be mistaken for green; verification transitions to `VERIFIED_FOR_SCOPE` or to a side state); `VALIDATED_FOR_SCOPE` → `VERIFIED_FOR_SCOPE` (the receipt says *verified for this scope*, never *validated* in general).

**Invariant:** a `VerificationReceipt` is emitted only after `CANDIDATE_FROZEN`, names its tree OID exactly, and any `UNKNOWN` in the manner of execution makes it non-publishable. `RECONCILED` is a working-tree state, not a content proof.

## Hooks/identity ordering: high-risk Ravum reference finding
Historic Ravum `git_writer.rs` at `5408d9b3…` (**`CODE_READ` only, not executed here**) runs `git write-tree` **before** the hooks (`:841-844`), calls `prepare-commit-msg` **before** `pre-commit` (`:1221-1228`), executes hooks with `current_dir` = the real repository root (`:1344`), and inherits the environment after removing only a short `GIT_*` list (`:85-94`, `:1353-1355`). Its own header claims "Git's order" while the code contradicts it. Consequences: a `pre-commit` that stages a file is **not** in the tree that gets committed; a hook can reach the real `.git`; an inherited variable can leak. Standard Git order is `pre-commit -> prepare-commit-msg -> commit-msg -> post-commit`, and the commit object is created only after `pre-commit` succeeds. Do not copy this implementation into Flow; scope it as `TREE-01`, `HOOK-02a/b/c/d`, `ENV-04a/b/c`, `HOOK-GITDIR-19`.

## Security/recovery
- Trusted vs untrusted processes have separate sandbox/permissions policies.
- `env_clear` or an allowlist is insufficient without OS process/FS/network isolation.
- Git ref CAS gives compare-on-commit, **not** atomic update of SQLite+filesystem or fencing epoch.
- For lease fencing, owner-local exclusive writer lock spans durable epoch check, ref CAS, outcome; external `git` bypasses this lock and must still be detected.
- An idempotent recovery journal distinguishes pre/post-ref-CAS, missing outcome, and dirty working tree; no `reset --hard` or silent human data loss.
- Binary bytes, modes, symlinks, case, renames, LFS, submodules, ignored build inputs are explicitly in scope or refused.

## What is actually implemented (MVP, `feat/flow-verified-foundation`)
`crates/flow` is a single dependency-free Rust binary implementing only the middle of the chain above:

- `run` computes the **index tree** with `git write-tree`, materialises that exact tree in a throwaway directory (Git worktree metadata removed before obligations run), executes only the obligations declared in `.flow/config.json` with a scrubbed environment, and writes a `VerificationReceipt` bound to the tree OID + a declared-input `BuildContextId` + `PolicyVersion` + execution boundary.
- `publish` refuses anything but a `PASSED` receipt for exactly that tree, appends a fsynced intent, re-reads the ref, then performs `git update-ref <ref> <new> <old>`; the outcome is appended afterwards and `reconcile` resolves a crash window by observation (`PUBLISHED` / `NOT_PUBLISHED` / `NEEDS_RECONCILIATION`).
- `doctor` reports what is **not** guaranteed (no sandbox, no hooks, no context completeness, no worktree checkout).

Not implemented, and therefore not promised: VFS/CRDT, scheduler, merge semantics, remote sync, hooks execution, working-tree reconciliation, multi-writer fencing. See the verification report for measured limits.

## Decision needed before architecture freeze
H1–H6 in [decision register](DECISION_REGISTER.md). Favor **candidate O3** for evaluation, not as accepted fact, unless O0/O1 wins empirical cost/correctness.
---

## MVP 0.1 — proposals, candidates and verified publication (implemented on `feat/flow-mvp-0.1`)

This section describes what the code on this branch **does**, not a ratified
architecture. The ADRs it implies remain `PROPOSED`.

### Objects and identities

| Object | Identity | Mutability | Where it lives |
|---|---|---|---|
| Proposal | `sha256("flow-proposal/1" + agent + task + base + idempotency_key)[0..32]` | immutable: an edit is a new proposal | `.flow/proposals/<id>.json` + `<id>.patch` |
| Candidate | `sha256("flow-candidate/1" + base + tree + ordered proposal ids)[0..32]` | immutable: same inputs, same id | `.flow/candidates/<id>.json` |
| Receipt | Git tree OID (the complete slot) | replaced only by a new complete run on that tree | `.flow/receipts/<tree>.json` |
| Journal | none — append-only sequence | append-only, fsynced | `.flow/journal.jsonl` |
| Adoption | policy digest | replaced only by an explicit `approve` | `.flow/adopted.json` |

Namespaces are not interchangeable: a **tree OID** identifies verified content, a
**commit OID** identifies a prepared publication, a **proposal id** identifies an
attributed change set, and a **context id** identifies tree + effective policy +
toolchain. Comparing across namespaces is a defect, not a shortcut.

### Transport decision (provisional)

A proposal is a **Git patch** (`git diff --binary`, byte-faithful for renames,
modes, symlinks and binary blobs as Git itself represents them), applied with
`git apply --cached --binary` to an index seeded by `git read-tree <base>` and
written with `git write-tree`. The alternative — referencing an agent's own
commit/tree — is not implemented; it would require the agent's objects to exist
in this repository's object store, which a remote agent does not have.

Consequences, stated honestly: the patch must apply to the exact base; the MVP
does no 3-way merge and no rename detection (`--no-renames`), so a rename
proposal is represented as a delete + an add and a rename/rename collision is
refused as a path collision.

### Pipeline

```
propose  ──> validation (identity, base is a commit, patch applies in a temp index,
             paths safe, tree representable) ──> immutable record
assemble ──> same base? same path operations compatible? patches apply in order
             in ONE temp index ──> candidate tree ──> immutable record
run      ──> materialise THAT tree in a scratch directory (`.git` link removed),
             verify fidelity, run the declared obligations, seal a receipt
publish  ──> receipt complete + policy/context still equal + candidate base == CAS old
             + ref is a real branch ──> intent ──> CAS ──> read-back ──> outcome
```

Every stage is separately observable through the CLI and through `.flow/`
artefacts; `flow explain --candidate <id>` joins proposal, candidate, receipt and
journal for one tree.

### Invariants (each has a test that fails if violated)

1. No Flow command writes the user's worktree or index for a proposal, candidate,
   verification or publication. (M6, `snapshot` byte comparison)
2. A receipt is valid only for the exact tree it names and the exact effective
   policy digest it embedded. (P0-A/P0-B gates)
3. A candidate that is verified is the candidate that is published: the receipt's
   tree must equal the candidate's tree, and the prepared commit must carry it.
4. An interrupted publication is resolved by observation, never by replay.
   (M5, journal gates)
5. A ref is moved only by CAS from an expected old OID, only for a real branch
   ref, and the result is read back. (`REF_IS_SYMREF`, `NEEDS_RECONCILIATION`)
6. A policy is executed only in a working copy that adopted its digest.
7. Fail-closed: anything not proven is refused with a stable machine code.

### Rejected shortcuts

- No second CAS/journal/receipt engine for candidates: `run --candidate` and
  `publish --candidate` call the same functions as the index path.
- No semantic merge, no AST merge, no automatic conflict resolution: overlap is
  diagnosed, and resolution is an explicit act by the operator or the agent.
- No daemon, no database, no network, no provider: the whole pipeline is local
  processes over Git objects and files.