# P0 baseline reproductions

Raw evidence that the three blocking defects exist **before** any correction.

| Field | Value |
| --- | --- |
| Repository | `Rwanbt/Ravum-flow` |
| Baseline commit (`crates/flow`, tests added on top) | `832f4995618a28c3e2bae25654e59651210a0faa` |
| Branch under correction | `fix/flow-foundation-proof-gates` |
| Host | Windows 11, Rust 1.95.0, git 2.51.1.windows.1 |
| Command | `cargo test --locked --test cli` |
| Result | **11 failed, 20 passed** |

`cli-tests-baseline.log` is the verbatim, unedited output of that command. It
contains only synthetic fixture data (temporary repositories under
`%TEMP%`, fake obligation ids, a fictional canary string) — no credential, no
real path outside the disposable fixtures, no Ravum content.

## What each failure proves

| Test | Baseline behaviour (observed) | Defect |
| --- | --- | --- |
| `p0a_selective_run_never_yields_a_publishable_receipt` | `flow run --obligation green` exits **0** and records `"status":"PASSED"` while the required `red` obligation never ran; the receipt occupies `<tree>.json` | **P0-A** partial verification recorded as a complete proof |
| `p0a_duplicate_or_unknown_obligation_ids_are_refused` | a repeated `--obligation` id and an unknown id are accepted silently | P0-A (selection is not validated) |
| `p0b_adding_a_required_obligation_makes_the_receipt_stale` | after the policy gains a required obligation, `publish` returns **PUBLISHED** for the pre-existing receipt | **P0-B** receipt reuse after policy change |
| `p0b_changing_a_timeout_makes_the_receipt_stale` | changing `timeout_s` does not invalidate the receipt | **P0-B** (timeout omitted from the context) |
| `p0b_changing_the_policy_version_makes_the_receipt_stale` | changing `policy_version` does not invalidate the receipt | **P0-B** (version read from a constant, not from the config) |
| `p0b_a_tampered_receipt_is_not_publishable` | flipping `"FAILED"` to `"PASSED"` in the receipt file is enough to publish | **P0-B** (no integrity binding) |
| `p0b_explicit_ref_must_name_a_branch` | `publish --ref HEAD` resolves and moves the branch | **P0-B** (ambiguous destination accepted) |
| `p0c_corrupt_journal_is_not_reported_as_reconciled` | a truncated journal yields `"status":"RECONCILED"`, exit **0**, `resolved: []` | **P0-C** corruption masked as success |
| `p0c_duplicate_intent_id_requires_a_human` | two intents sharing one id are both "resolved" | **P0-C** ambiguous recovery |
| `p0c_ref_moved_elsewhere_needs_reconciliation` | `NEEDS_RECONCILIATION` is reported inside a successful `RECONCILED` run, exit **0** | **P0-C** non-success rendered as success |
| `p0c_status_refuses_to_guess_on_a_corrupt_journal` | `flow status` reports `pending_intents: 0` on an unreadable journal | **P0-C** silent empty read |

The three tests whose names start with `p0a_full_run_`, `p0a_an_optional_` and
`two_concurrent_publishes_` pass on the baseline: they are *guards*, not
reproductions, and exist so the correction cannot regress behaviour that was
already right.

## Reproduce

```sh
git checkout 832f499
# copy crates/flow/tests/cli.rs from fix/flow-foundation-proof-gates
cargo test --locked --test cli
```

A red result here is the expected result: it is the reproduction, not a
regression. The corrected build must turn every row above green without
weakening the assertions.
