# P0 baseline reproductions

Raw evidence that the three blocking defects exist **before** any correction.

| Field | Value |
| --- | --- |
| Repository | `Rwanbt/Ravum-flow` |
| Baseline commit | `832f4995618a28c3e2bae25654e59651210a0faa` |
| Correction branch | `fix/flow-foundation-proof-gates` |
| Host | Windows 11, Rust 1.95.0, git 2.51.1.windows.1 |
| Command | `cargo test --locked --test cli` |
| Result | **16 failed, 20 passed** |

`cli-tests-baseline.log` is the verbatim, unedited output of that command, run
with the **final** test file (the one shipped on the correction branch), against
a throwaway worktree of the untouched baseline. It contains only synthetic
fixture data (temporary repositories under `%TEMP%`, fake obligation ids) — no
credential, no real path outside the disposable fixtures, no Ravum content.

## What each failure proves

| Test | Baseline behaviour (observed) | Defect |
| --- | --- | --- |
| `p0a_selective_run_never_yields_a_publishable_receipt` | `flow run --obligation green` exits **0**, records `"status":"PASSED"` while the required `red` obligation never ran, and occupies `<tree>.json` | **P0-A** partial verification recorded as a complete proof |
| `p0a_duplicate_or_unknown_obligation_ids_are_refused` | a repeated `--obligation` id and an unknown id are silently accepted | P0-A — selection is not validated |
| `p0b_adding_a_required_obligation_makes_the_receipt_stale` | after the policy gains a required obligation, `publish` returns **PUBLISHED** for the pre-existing receipt | **P0-B** receipt reuse after a policy change |
| `p0b_changing_a_timeout_makes_the_receipt_stale` | changing `timeout_s` does not invalidate the receipt | **P0-B** — timeout omitted from the context |
| `p0b_changing_the_policy_version_makes_the_receipt_stale` | changing `policy_version` does not invalidate the receipt | **P0-B** — version read from a constant, not the config |
| `p0b_a_tampered_receipt_is_not_publishable` | flipping `"FAILED"` to `"PASSED"` in the receipt file is enough to publish | **P0-B** — no integrity binding |
| `p0b_a_timeout_override_is_never_publishable` | `run --timeout 1` then `publish` publishes, although the receipt proves a 1 s deadline and the configured policy says 30 s | **P0-B** — the applied policy is not the proven one |
| `p0b_explicit_ref_must_name_a_branch` | `publish --ref HEAD` resolves and moves the branch behind it | **P0-B** — ambiguous destination accepted |
| `p0b_an_ambiguous_config_is_refused_not_guessed` | a config with a duplicated `required` key is accepted and the first value wins | **P0-B** — two readers can disagree about the policy |
| `p0c_corrupt_journal_is_not_reported_as_reconciled` | a truncated journal yields `"status":"RECONCILED"`, exit **0**, `resolved: []` | **P0-C** corruption masked as success |
| `p0c_duplicate_intent_id_requires_a_human` | two intents sharing one id are both "resolved" | **P0-C** ambiguous recovery |
| `p0c_ref_moved_elsewhere_needs_reconciliation` | `NEEDS_RECONCILIATION` is reported inside a successful `RECONCILED` run, exit **0** | **P0-C** non-success rendered as success |
| `p0c_status_refuses_to_guess_on_a_corrupt_journal` | `flow status` reports `pending_intents: 0` on an unreadable journal | **P0-C** silent empty read |
| `p0a_case_colliding_tree_is_refused_not_certified` | a tree holding `Case.txt` and `case.txt` is checked out as one file; `run` returns **PASSED** and `publish` **PUBLISHED** for the two-entry tree | **P0-1** materialisation fidelity never verified |
| `p0a_symlink_rewritten_as_a_file_is_refused` | with `core.symlinks=false`, a `120000` entry becomes a regular file holding the target path; an obligation asserting that content passes and `publish` publishes | **P0-1** same class |
| `p0c_one_outcome_cannot_close_two_intents` | two intents share one id (same tree, same second, two refs) and one outcome closes both: `reconcile` exits 0 `RECONCILED` while one ref was never published | **P0-2** duplicate-id masking |

## Guards (green on both sides by design)

These pass on the baseline too. They are not reproductions: they exist so the
correction cannot regress behaviour that was already right, and so a later
weakening is caught.

- `p0a_full_run_with_one_failing_required_obligation_is_never_publishable`
- `p0a_an_optional_failing_obligation_does_not_mask_a_green_required_set`
- `p0c_reconcile_is_idempotent` — the *behaviour* was already idempotent; what
  was broken was reporting corruption as success, covered above.
- `two_concurrent_publishes_publish_at_most_once` — `update-ref` CAS already
  prevented a lost update; the lock added under P1 makes the journal appends
  non-interleaved as well.

## Reproduce

```sh
# a throwaway worktree at the defective commit, with the final tests copied in
git worktree add /tmp/flow-baseline 832f499
cp crates/flow/tests/cli.rs /tmp/flow-baseline/crates/flow/tests/cli.rs
cd /tmp/flow-baseline && cargo test --locked --test cli
```

A red result here is the expected result: it is the reproduction, not a
regression. The corrected build must turn every row above green without
weakening the assertions.

`experiments/r0a` and `experiments/r0b` are unrelated to these gates: they record
environmental risks of Git and the OS, not the Flow correction.
