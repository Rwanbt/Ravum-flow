    Finished `release` profile [optimized] target(s) in 5.61s
# Crash points on the release binary

Binary: `/tmp/flow-crash-target/release/flow`

| case | killed | status after | reconcile | publication afterwards | ref moved | verdict |
|---|---|---|---|---|---|---|
| rename-1 | exit 137 | REPORT | RECONCILED | PUBLISHED | no | PASS |
| rename-2 | exit 137 | REPORT | RECONCILED | REFUSED(STALE_REF) | yes | PASS |
| fsync-1 | exit 137 | REPORT | RECONCILED | PUBLISHED | no | PASS |

Failures: 0
