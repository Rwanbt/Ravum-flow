# Wave 8 — Windows stability campaign: eight full-suite runs, one resource failure

Status: **measurement only.** No product change came out of it. The tree under test was the wave-8
code head `3b1e33e` (the doctor assertion in `f5a19de` came afterwards and is not part of these runs).

## Method

Eight consecutive `cargo test --locked --offline --all-targets` runs on the Windows host, started from
the same working tree, with the thread count alternating to cover both modes: run 2 with
`RUST_TEST_THREADS=1`, the other seven at the default (parallel) count that CI uses. Each run's exit
code, per-run passed/failed/ignored counts and wall time are in `windows-stability.csv`; the failing
run's full output is kept as `failure-run-7.txt`.

## Results

| run | threads | exit | passed | failed | ignored | seconds |
|---|---|---|---|---|---|---|
| 1 | default | 0 | 468 | 0 | 0 | 116.9 |
| 2 | 1 | 0 | 468 | 0 | 0 | 589.8 |
| 3 | default | 0 | 468 | 0 | 0 | 129.8 |
| 4 | default | 0 | 468 | 0 | 0 | 149.4 |
| 5 | default | 0 | 468 | 0 | 0 | 136.4 |
| 6 | default | 0 | 468 | 0 | 0 | 144.8 |
| 7 | default | **101** | **451** | **4** | 0 | 134.3 |
| 8 | default | 0 | 468 | 0 | 0 | 123.3 |

Seven of eight runs are green with the full 468 executed and nothing ignored.

## The failing run is a host-resource failure, not a product defect (MEASURED + classified)

Run 7 failed four tests, all in `proposals.rs` (`binary_and_mode_changes_survive_the_cycle`,
`m1_two_disjoint_agents_publish_exactly_the_verified_tree`,
`m7_a_replacement_agent_resumes_from_the_immutable_artefacts`,
`m3_stale_base_is_refused_and_a_rebased_proposal_succeeds`). The failure text each test printed is the
product's own wrapper around a **failed process spawn**, not an assertion about Flow's behaviour:

```
{"status":"REFUSED","code":"PATCH_REFUSED","reason":"… error launching git:
 Ressources système insuffisantes pour terminer le service demandé."}
{"status":"ERROR","error":"cannot write the proposal tree: error launching git:
 Le fichier de pagination est insuffisant pour terminer cette opération."}
```

Those are the Windows system errors `ERROR_NO_SYSTEM_RESOURCES` and `ERROR_COMMITMENT_LIMIT`: the OS
refused to start another process because memory/paging was exhausted. The run was executed
**concurrently with the WSL2 per-module mutation campaign** (two `cargo` jobs and repeated workspace
copies), which is exactly the concurrency `AGENTS.md` warns against on a 16 GB host.

The suite's own transparency is the evidence that this is not a masked product failure: `flow`
reported the launch failure as `ERROR`/`REFUSED` with the OS reason instead of reporting a green run;
the four tests then failed on that status, as they should.

## Re-run evidence

- Run 8, four minutes later on the same tree and with the same command, is **468 passed / 0 failed /
  0 ignored**.
- `cargo test --locked --offline --test proposals` alone afterwards: **23 passed / 0 failed**.

## Limit of this evidence

Eight runs on a host that was also running the Linux campaign is a **stability sample, not a
flake-freedom proof**. It did surface one real thing: under memory pressure the product's failure
mode is honest (it reports the spawn failure), and the suite turns that into a red run. A clean
serialised campaign (no concurrent Cargo job) remains the way to claim a rate, and is not claimed
here.
