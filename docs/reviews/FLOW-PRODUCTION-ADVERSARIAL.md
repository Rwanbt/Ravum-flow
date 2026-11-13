# FLOW production-candidate — independent adversarial report (wave 2)

Reviewer: `security` teammate (roles A/G). No write access to `crates/flow/src`.
This report is the wave-2 result of the harness in
[`experiments/mvp-qa/attacks.py`](../../experiments/mvp-qa/attacks.py); every
number below is read from
[`experiments/mvp-qa/results.json`](../../experiments/mvp-qa/results.json).

**Headline: 119 declared, 116 executed, 3 NOT_RUN, 0 harness errors, 109
REFUTED, 7 DEFECT_REPRODUCED (3 × P2, 4 × P3), 0 reproduced P0.**

## 0. What changed since the 0.1 report

The 1.0 harness was written against `d884e29`. It is now adapted, not softened:

- Fixtures adopt their policy (`flow init` + `flow approve`) before `run`/
  `publish`, exactly as a human operator must. Scenarios that attack the
  adoption gate itself pass `adopt=False`.
- A scenario whose target is unreachable because a *different* gate fired is an
  **ERROR** in this harness, never a REFUTED verdict. The refusal code is
  compared against an explicit identity-gate allowlist; an adoption-gate code
  raises instead of producing a false negative.
- The seven `FUT-*` scenarios are enabled against the real 0.1 surface.
- New scenarios: `.flow` components as symlink/junction/hardlink (FIL-16..20),
  TOCTOU substitution (FIL-21), shipped-policy display (FIL-22/23), `explain
  --tree` odd shapes (PRF-07..09), deleted evidence display (PRF-08),
  release-build crash hook (PRC-14), and huge/odd inputs (INP-01..05).
- No attack was weakened: where a fix landed, the old assertion is retained and
  the refusal code that fired is recorded.

Verdict vocabulary mapping (the harness's own labels are in parentheses):
`DEFECT_REPRODUCED` (= REPRODUCED), `REFUTED` (= REFUTED), `NOT_RUN` (= NOT_RUN).
`ATTACK_EXECUTED` is not a separate value: a reproduced defect *is* an executed
attack. `UNSUPPORTED_OS` is recorded as `NOT_RUN` with the OS reason, because
the harness's schema has no separate slot.

## 1. Method and environment

| Item | Value |
|---|---|
| Worktree | `D:\App\Ravum-flow-wt\prod-security`, branch `work/prod-security` |
| Base SHA (branch point, wave 1) | `ea181d4` |
| Head SHA under test | `f237d57` (`feat/flow-production-candidate`, PR #54) |
| Binary (debug) | `target/debug/flow.exe`, sha256 `5d0b778ed43adee3…`, `flow 0.2.0-rc.1` |
| Binary (release, for PRC-14) | `target/release/flow.exe`, sha256 `54b1371b45ecb0c7…` |
| OS | Windows 11 10.0.26100 (harness platform `Windows-11-10.0.26100-SP0`) |
| Git | 2.51.1.windows.1 |
| Harness | version 2.0, Python stdlib only, 173.4 s wall |
| Fixtures | disposable repositories under the system temp directory; fictional canary `CANARY-FAKE-SECRET-001` only |
| Reproduce | `cargo build && cargo build --release` then `python attacks.py --flow target/debug/flow.exe --release-flow target/release/flow.exe --out experiments/mvp-qa/results.json --markdown experiments/mvp-qa/verdict-tables.md` |

One build of each profile, no concurrent heavy Cargo jobs. Per-scenario
evidence (command, expected refusal, observed status/code/exit, ref OID, file
SHA-256, journal bytes) is in `results.json`; the flat tables are in
[`verdict-tables.md`](../../experiments/mvp-qa/verdict-tables.md).

## 2. Verdicts per axis

| Axis | Declared | Executed | REFUTED | DEFECT_REPRODUCED | NOT_RUN |
|---|---|---|---|---|---|
| identity / receipt scope | 20 | 20 | 20 | 0 | 0 |
| scope | 11 | 11 | 11 | 0 | 0 |
| Git refs and CAS | 12 | 12 | 12 | 0 | 0 |
| durable journal | 11 | 10 | 10 | 0 | 1 |
| files and hostile paths | 23 | 22 | 21 | 1 | 1 |
| process containment | 14 | 14 | 13 | 1 | 0 |
| proof quality | 9 | 8 | 5 | 3 | 1 |
| compatibility | 7 | 7 | 6 | 1 | 0 |
| 0.1 multi-agent surface | 7 | 7 | 7 | 0 | 0 |
| huge / odd inputs | 5 | 5 | 4 | 1 | 0 |
| **total** | **119** | **116** | **109** | **7** | **3** |

`ERROR` = 0: no scenario failed to reach a verdict. `NOT_RUN` is never counted
as a pass.

## 3. Every DEFECT_REPRODUCED

### P2-1 · FIL-20 · a hard-linked `.flow/journal.jsonl` appends outside the workspace

Still present at `f237d57` on Windows.

```
# fixture: disposable repo, flow init, config with one required green obligation, flow approve,
#          one staged file so publish has work to do
cmd /c mklink /H <repo>\.flow\journal.jsonl <outside>\victim.jsonl
flow run
flow publish        # exit 0, status PUBLISHED
# observed: victim.jsonl grows by 514 bytes and now begins with
# {"schema":"flow-journal/1","event":"intent","id":"2026-11-09T23:45:11Z-…
```

The state-path guard treats a hard link as a regular file on Windows. At
`f237d57` `state.rs::guard_below_flow` contains a link-count check but it is
`#[cfg(unix)]`, so the production target platform has no check at all; the
`append_journal` path opens the file with `O_APPEND` and writes through the same
inode. Reachability, stated honestly: a Git tree cannot carry a hard link, so
this is **not** reachable by cloning (unlike the symlink class, which is now
closed); it needs a local writer inside `.flow` (the product treats the local
user as trusted) or an archive extractor that materialises a hard link into
`.flow`. The appended bytes are Flow journal records, not attacker-chosen
content. Severity P2: a user file outside the repository changes.
Minimal fix shape: apply the `nlink > 1` check on all platforms (Rust exposes
`std::os::windows::fs::MetadataExt::number_of_links`).

### P2-2 · PRC-10 · `FLOW_TEST_CRASH` is still honoured by the debug binary

Still present at `f237d57` in the **debug** build; `PRC-14` shows the release
build is inert.

```
flow run
FLOW_TEST_CRASH=after-intent flow publish   # exit 89
FLOW_TEST_CRASH=after-ref    flow publish   # exit 89
FLOW_TEST_CRASH=not-a-mode   flow publish   # exit 1 (unknown mode falls through)
```

Exit 89 is outside the documented 0/1/2 contract. Impact is denial of
publication plus a forced recovery path, not a false `PASSED`. The claimed fix
("compiled out of release") is confirmed by PRC-14 — but a developer or CI job
that runs the debug artifact still has a crash switch in the binary. Severity
P2 for the debug artifact, P3 if only the release artifact ships.

### P2-3 · PRF-01 · a re-sealed forged receipt publishes

Accepted, documented limitation, still present.

```
flow run                    # one required obligation is red -> FAILED
# edit .flow/receipts/<tree>.json: status PASSED, obligation status/exit PASSED/0,
# recompute the `integrity` digest exactly as flow does
flow publish                # exit 0, PUBLISHED
```

Integrity is not a signature: anyone who can write `.flow/receipts` can forge a
receipt. The casual edit (no re-seal) is refused `RECEIPT_INTEGRITY` (IDN-10).
No new information versus the 0.1 report; recorded because the harness must not
assume the documented boundary is a closed gate.

### P3-1 · PRF-02 / PRF-08 · evidence files can be deleted; `status`, `explain` and `publish` keep saying PASSED

Still present at `f237d57`. After a green run, delete every
`.flow/logs/<tree>/*.log`:

- `flow status` → `latest_receipt.status = "PASSED"`;
- `flow explain --tree <tree>` → receipt `status: PASSED`, `logs` still lists 2
  paths that do not exist;
- `flow publish` → `PUBLISHED`, `receipt_logs: 2`.

The receipt binds the tree and the policy, not the existence of the evidence it
names. Not a gate breach; `PASSED` plus `receipt_logs: N` reads as "the
evidence is here". The candidate head's "missing-log evidence" fix is **not
observable** in these four fields: I measured the same values as before. Either
the fix covers a different surface, or it is not effective; the Lead should
check `receipt_logs`/`logs` against the filesystem.

### P3-2 · INP-03 · case-colliding obligation ids are accepted and overwrite each other's evidence

Still present at `f237d57`.

```
# .flow/config.json: two required obligations with ids "PROBE" and "probe"
flow approve && flow run     # exit 0, PASSED
# receipt.logs  = [PROBE.stdout.log, PROBE.stderr.log, probe.stdout.log, probe.stderr.log]
# files on disk = [probe.stdout.log, probe.stderr.log]      # NTFS folds case
```

`validate_obligations` applies `paths::validate_relative` and an exact-match
duplicate check, not the host's case folding, although `doctor` warns about
case-insensitive worktrees in general. One obligation's evidence silently
overwrites the other's and the receipt lists paths that cannot coexist. No false
`PASSED` (statuses are kept per obligation). Severity P3.

### P3-3 · CMP-01 · `help` and a bare invocation print text, not JSON

Documented overclaim, unchanged: `flow help` (exit 0) and a bare `flow`
(exit 2) print usage text while the README says every command prints
machine-readable JSON. Severity P3, documentation.

## 4. The claimed candidate fixes, re-attacked

| Claimed fix | Scenario | Result |
|---|---|---|
| id-as-path (`explain --tree`, `proposals show`, `run --candidate`) | PRF-06, PRF-07 | **REFUTED the attack.** `explain --tree` now refuses absolute/traversal values with `INVALID_TREE_ID: … is not a 40- or 64-character hex id`; prefix/empty/dot/traversal shapes return a null receipt and cannot be made to read a planted file. |
| tampered-receipt display in `status`/`explain` | PRF-04 | **REFUTED the attack.** `status` shows `RECEIPT_INTEGRITY_INVALID`, `explain` does not present the tampered receipt as passed, and `publish` still refuses `RECEIPT_INTEGRITY`. |
| missing-log evidence | PRF-02, PRF-08 | **Still reproduces** (P3-1 above). |
| byte-exact checkout fidelity | FIL-08 | **REFUTED the attack.** A `.gitattributes` CRLF conversion is refused `UNSUPPORTED_CHECKOUT` before any obligation runs (blob LF, 11 bytes, obligation wanted CRLF). |
| `.flow` symlink/junction guards | FIL-09, 10, 11, 12, 13, 16, 17, 18, 19 | **All REFUTED.** Journal, receipts, logs (junction inside the repo and symlink outside), proposals, candidates, `adopted.json` and `lock` all refuse with the workspace-escape guard; outside targets are byte-identical and the ref does not move. The lock case holds by construction (`create_new` + `hard_link` never follows the destination) and refuses `LOCK_UNKNOWN`. |
| hardlink guards | FIL-20 | **Still reproduces** on Windows (P2-1). The check is `#[cfg(unix)]`. |
| `doctor` exit 1 on `overall=FAILED` | PRF-05 | **REFUTED the attack.** Torn journal: `overall=FAILED`, exit **1**. |
| policy provenance in `status` | FIL-22 | **REFUTED the attack.** `status.policy` now carries `adopted`, `adopted_at`, `adoption_source`, `digest`; a clone-shipped policy reports `adopted: false` while `run` refuses `POLICY_NOT_ADOPTED`. |
| `FLOW_TEST_CRASH` compiled out of release | PRC-14 | **REFUTED the attack.** Release build ignores both modes and publishes normally. The debug build still honours it (P2-2). |
| TOCTOU shape (not in the fix list) | FIL-21 | **REFUTED the attack.** `.flow/receipts` replaced by a symlink during a 5 s obligation: `run` refuses with the guard, nothing outside. |

## 5. Everything NOT_RUN, with the reason

| ID | Reason |
|---|---|
| DUR-09 | No injection point exists for a crash *before* the intent append: `FLOW_TEST_CRASH` supports only `after-intent` and `after-ref`, and killing the process between `commit-tree` and the journal append is not deterministic from outside. |
| FIL-06 | Git itself refuses to place a Windows-reserved name or a `..` path in the index (`update-index: Invalid path`; `read-tree` refuses the raw tree), so no verified index tree can contain one. Fixture impossible on this host; the same class is covered for obligation ids by INP-02/INP-04. |
| PRF-09 | A NUL byte cannot be delivered to any process on Windows: `CreateProcess`/`subprocess` refuses the argument (`ValueError: embedded null character`) before Flow starts. This shape is `UNSUPPORTED_OS`, not a product result. |

## 6. What I could not attack, and why

1. **The check-then-write window itself (FIL-21).** The deterministic
   substitution is caught. A high-frequency swap loop (995 rename/symlink swaps
   during a 3 s run) never landed between `guard_below_flow` and the receipt
   `rename`; I did not win the race, so I do not claim the race is closed. The
   product's own comment documents this as not defending a concurrent local
   attacker. A reliable hit would need in-process instrumentation or a filesystem
   that can be paused, neither of which is available here.
2. **Hard links as a supply chain.** A Git tree has no hard-link mode, so a
   cloned candidate cannot ship one. I did not build a tar/zip extraction
   variant: it would prove an archive-tool property, not a Flow property, and
   the time-box did not allow a deterministic fixture.
3. **Case-folding behaviour beyond this host.** INP-03 measures NTFS. The same
   policy on APFS (case-insensitive by default) or on a case-sensitive Linux
   checkout may behave differently; I did not run a Linux fixture.
4. **Receipt forgery without a key.** PRF-01 is an accepted limitation: no
   attack can distinguish "a receipt Flow wrote" from "a receipt the local user
   re-sealed". Only a signature would change the result, and the product says it
   has none.
5. **Native Windows junctions for every component.** Directory components use a
   junction (unprivileged) and file components use a symlink (requires Developer
   Mode or the symlink privilege); both mechanisms are reparse points and the
   same guard covers them. If a future host denies symlink creation, the file
   scenarios raise `NOT_RUN` with that reason rather than passing.
6. **Recovery under a real power loss.** Crash injection covers
   `after-intent`/`after-ref`, not a torn sector or a full disk; those need
   fault injection at the filesystem layer.

## 7. Genuine capability loss observed

None that blocks the candidate. Two deliberate contract changes are worth
recording because they change what an operator can do:

1. `explain --tree` now requires a 40/64-character hex OID. A bare prefix or a
   branch name can no longer be resolved; `explain` with no argument still
   resolves the HEAD tree. That is the fix for PATH-08, not a regression.
2. A policy that arrives with a clone executes nothing until this working copy
   runs `flow approve`/`init --adopt-policy`. That is the adoption gate doing its
   job; the cost is one explicit step per clone.

One *claim* gap, not a regression: there is no candidate-bound human approval.
`approve` binds the policy digest; `publish --candidate` is gated by a PASSED
receipt for the candidate tree plus the ref CAS and base check (FUT-05). An agent
that can run `run --candidate` and `publish --candidate` can publish without a
second human act after the policy was approved. The 0.1 baseline list declared
this as an expected property; the product does not claim it (verification report
§3.3), so it is reported as an observation, not scored as a defect.

## 8. Artefacts

- `experiments/mvp-qa/attacks.py` — harness 2.0 (119 scenarios).
- `experiments/mvp-qa/results.json` — raw evidence for this report.
- `experiments/mvp-qa/verdict-tables.md` — generated flat tables (per axis).
