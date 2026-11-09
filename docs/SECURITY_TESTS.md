# Security and correctness matrix — PROPOSED, tests not executed
All tests use **disposable synthetic repositories** and **fictitious canaries** only; no private source, real credential or live Ravum workspace. No execution until `GO_R0_FIXTURES`.

| Test | Injection / threat | Expected gate |
|---|---|---|
| TREE-01 | dirty/untracked dependency makes working directory pass while published tree fails | validate exact frozen Git tree or fail closed |
| HOOK-02a | approved pre-commit changes isolated index | final frozen tree includes authorized changes and is reverified |
| HOOK-02b | approved hook writes a dirty human file | user working directory untouched; refuse/reconcile safely |
| HOOK-02c | hook records invocation order | Git-compatible pre-commit, prepare-commit-msg, commit-msg |
| TREE-03 | ignored file changes build outcome | BuildContextId includes it or result is scoped/UNKNOWN |
| ENV-04 | `FLOW_TEST_SECRET_CANARY` in host env and fake SSH directory | candidate worker cannot read/access, no leak to logs/network |
| LEGACY-06 | Ravum V1 AUTO_SAFE with `SKIPPED` verification | only TEXTUALLY_APPLICABLE, never publishable receipt |
| FENCE-07 | expired writer resumes after new epoch | writer lock+epoch reject; external Git limitation documented |
| FILE-09 | binary/modes/renames/deletions/symlink/Windows case/LFS | bytes and modes accurate or unsupported/fail closed |
| GIT-10 | HEAD/ref changes between prepare and publish | compare-and-swap refuses stale candidate |
| FS-11 | human edits selected/neighboring path before reconcile | NEEDS_RECONCILIATION, no `reset --hard` |
| CRASH-12 | kill at each journal/ref boundary | idempotent recovery, no double publish |
| E2E-13 | API change breaks unchanged consumer; E2E red | conservative invalidation or full gate, never green |
| CACHE-14 | toolchain/flags/lockfile/env differ from cached receipt | cache miss/STALE |
| TRUST-15 | malicious hook/test attempts fake SSH read or network egress | OS sandbox works or blocked as UNSUPPORTED |
| PATH-16 | traversal/symlink/junction/Unicode/case attack | no write outside allowed workspace |

## Evidence standard
Record fixture SHA, OS, execution sandbox, command and limits, setup/probe logs redigés, expected vs observed, failure reproduction status `REPRODUCED | DISPROVED | NOT_REPRODUCED | BLOCKED`. A successful unit test is not a formal security proof. High-risk findings are handed to **Ravum session** for its own prioritization, not fixed by Flow unilaterally.