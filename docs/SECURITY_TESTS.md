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
