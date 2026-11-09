# Security and correctness matrix — PROPOSED, tests not executed
All tests use **disposable synthetic repositories** and **fictitious canaries** only: no private source, real credential or live Ravum workspace. Execution status of every row is `NOT_RUN` until a run publishes its evidence.

## Canonical test IDs
**Canonical IDs are the V6 §9 identifiers**, as consolidated in [DEEPSEEK-03 §8](reviews/DEEPSEEK-03-THREAT-MODEL.md). This file is an **index**, not a second source: it may add an expected gate, never a competing number. Old identifiers are kept only in the alias column.

| Canonical ID | Alias (legacy) | Injection / threat | Expected gate |
|---|---|---|---|
| `TREE-01` | `TREE-01` | dirty/untracked dependency makes the working directory pass while the frozen tree fails | verify from the exact frozen Git tree OID or fail closed |
| `HOOK-02a` | `HOOK-02a` | approved `pre-commit` mutates the isolated index | final frozen tree contains authorized changes and is reverified, or loud refusal |
| `HOOK-02b` | `HOOK-02b` | approved hook writes a dirty human file | user working directory untouched; refuse/reconcile, never silent overwrite |
| `HOOK-02c` | `HOOK-02c` | hook records invocation order | Git-compatible order `pre-commit → prepare-commit-msg → commit-msg → post-commit` |
| `HOOK-02d` | — | hook fails, leaves unstaged content, or `post-commit` is red after CAS | failure is traceable; never a silent bypass |
| `CONTEXT-03` | `TREE-03` | ignored input changes the build outcome | input declared in `BuildContextId`, or scope `UNKNOWN` and no publishable receipt |
| `ENV-04a` | `ENV-04` | `FLOW_FAKE_SECRET_CANARY` present in host env | untrusted process never receives it |
| `ENV-04b` | — | fake `~/.ssh`, `.env`, `%APPDATA%` on disk | unreadable by the untrusted worker |
| `ENV-04c` | — | canary in stdout/stderr/journal/artifacts | redacted **before persistence** |
| `SANDBOX-05` | `TRUST-15` | fake SSH read + outbound network from untrusted code | refused when the advertised mode is claimed; otherwise code is not executed |
| `LEGACY-06` | `LEGACY-06` | Ravum V1 `AUTO_SAFE` with `SKIPPED` verification | `TEXTUALLY_APPLICABLE` only; never a publishable receipt |
| `FENCE-07a` | `FENCE-07` | expired writer resumes after a newer epoch | writer lock + epoch reject; external Git documented as outside the promise |
| `FENCE-07b` | — | external `git` publishes between our read and CAS | CAS refuses stale candidate; unmanaged bypass documented |
| `FENCE-07c` | — | crash between CAS and outcome append | state `UNKNOWN_OUTCOME`; reconciliation non-destructive |
| `FENCE-07d` | — | API returns `Err` after the ref was actually published | outcome never reported as clean failure |
| `PATH-08` | `PATH-16` | traversal, **parent** symlink/junction, reserved Windows names, case, Unicode | no write outside the allowed workspace; otherwise `UNSUPPORTED` |
| `FILE-09` | `FILE-09` | binary, `100755`, `120000`, rename, delete, `.gitattributes`, CRLF | bytes and modes exact, or explicit `UNSUPPORTED` |
| `REF-10` | `GIT-10` | HEAD/ref changes between prepare and publish | compare-and-swap refuses stale candidate |
| `CRASH-11` | `CRASH-12` | kill at each journal / ref boundary | idempotent replay, no double publish |
| `E2E-12` | `E2E-13` | API break hits an unchanged consumer; required E2E is red | conservative invalidation or full gate; never green |
| `CACHE-13` | `CACHE-14` | toolchain/flags/lockfile/env/OS differ from a cached receipt | cache miss or `STALE`; no promotion |
| `RESOURCE-14` | `RESOURCE-14` | 16 GB ceiling, cancellation, surviving descendants | no zombie, no simulated success, quota never exceeded |
| `FORGE-15` | — | forge merge queue / squash changes the final tree | requalify at the exact final tree |
| `RECOVERY-16` | `FS-11`, `RECOVERY-16` | truncated journal, full disk, GC'd objects | honest non-destructive recovery |
| `UNMANAGED-17` | — | writes from tools outside the managed protocol | detection only; no prevention promise |
| `OS-18` | — | Windows case/AV, Linux permissions/FUSE | announced modes only; no destructive fallback |
| `HOOK-GITDIR-19` | `X-HOOK-GITDIR` | a hook in a linked worktree reaches the shared Git directory (refs, config) | documented `UNSUPPORTED` without real OS isolation; worktree is **not** a sandbox |

## Evidence standard
Record fixture SHA, OS, execution sandbox, command and limits, redacted setup/probe logs, expected vs observed, and failure status `REPRODUCED | DISPROVED | NOT_REPRODUCED | BLOCKED | NOT_RUN`. A passing unit test is not a security proof. High-risk findings about Ravum go to a separate handoff for the Ravum session; Flow does not patch Ravum unilaterally.

## MVP 0.1 (2026-11-09) — new negative tests

| Boundary | Test | Assertion |
|---|---|---|
| Lock: live owner is never displaced | `a_live_old_lock_is_never_stolen` | ancient timestamp + live pid ⇒ `LOCKED`, lock preserved, ref unmoved |
| Lock: unattributable file is refused | `an_unparseable_lock_is_refused_and_preserved` | empty/foreign record ⇒ `LOCK_UNKNOWN` + `manual_required`, file preserved |
| Lock: dead owner recovery | `a_dead_owners_lock_is_recovered` | provably dead pid ⇒ lock recovered, publication proceeds |
| Lock: at-most-once under contention | `eight_concurrent_publishes_publish_at_most_once` | exactly one `PUBLISHED`, one intent/outcome, one added commit |
| Git: bounded output | `obligation_stdout_is_bounded_through_the_cli`, `read_capped_at_*` | 10 MiB obligation stdout truncated + marked; 65 MiB Git stdout ⇒ `GIT_OUTPUT_TOO_LARGE` |
| Git: hijacked environment | `hijacked_git_environment_does_not_redirect_the_cli` | `GIT_DIR`/`GIT_WORK_TREE`/`GIT_INDEX_FILE`/`GIT_CONFIG_*` cannot redirect the CLI |
| Hooks are dispatched by plumbing | `nonblocking_hooks_are_dispatched_by_run_and_publish`, `blocking_post_checkout_...`, `blocking_reference_transaction_...` | measured, not assumed: a blocking hook aborts `run`/`publish` and the ref does not move |
| Missing toolchain | `missing_toolchain_is_unsupported_and_never_publishable` | `UNSUPPORTED`, never `PASSED` |
| Ref: symbolic ref | `p1_a_symbolic_ref_is_never_a_publication_target` | `REF_IS_SYMREF`, target branch unmoved |
| Proposal: idempotency and immutability | `admission_is_idempotent_and_immutable` | same key+bytes ⇒ same id; same key+different bytes ⇒ `IDEMPOTENCY_CONFLICT` |
| Proposal: forbidden paths | `proposals_outside_the_trusted_surface_are_refused` | `.flow/`, `.git/`, unknown base refused, nothing written |
| Assembly: ambiguity | `m2_three_agents_collision_...`, `assembly_reports_order_...` | collision diagnosed with the proposals named, no ref/worktree change |
| Publication: stale base | `m3_stale_base_...` | `BASE_MISMATCH`, no forced update, explicit rebase then success |
| Publication: no receipt | `publishing_a_candidate_requires_a_verified_receipt` | `NO_RECEIPT`, ref unmoved |
| Dry run | `publish_dry_run_plans_without_mutating_anything` | refs, journal and files unchanged |
| User files preserved | `m6_user_files_and_index_are_preserved_through_the_whole_cycle` | index listing and every file hash unchanged through the whole cycle |

**Still `NOT_RUN`:** adversarial campaign consolidation (`docs/reviews/MVP-QA-ADVERSARIAL-REPORT.md`),
R0 corpus on Linux, OS sandbox, hook execution as a verification source, context completeness.