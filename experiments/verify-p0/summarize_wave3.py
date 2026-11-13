#!/usr/bin/env python3
"""Render the wave-3 campaign summary as Markdown (stdlib only).

Reads experiments/mvp-qa/results-wave3.json and prints the blocks that
docs/reviews/FLOW-PRODUCTION-ADVERSARIAL-WAVE3.md embeds, plus the wave-2
comparison. Kept in the repository so the numbers in the review can be
re-derived from the machine-readable results without trusting a transcript.
"""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
W3 = ROOT / "experiments" / "mvp-qa" / "results-wave3.json"
W2 = ROOT / "experiments" / "mvp-qa" / "results.json"
REPLAY = ROOT / "experiments" / "mvp-qa" / "results-wave3-119.json"
AXIS_TITLES = {
    "identity": "Identity / receipt scope",
    "scope": "Scope (portée)",
    "git": "Git refs and CAS",
    "durable": "Durable journal and recovery",
    "files": "User files and hostile paths",
    "process": "Process containment",
    "proof": "Proof quality",
    "compat": "Compatibility",
    "future": "0.1 multi-agent surface",
    "inputs": "Huge and odd inputs",
    "fidelity": "Checkout fidelity (wave 3, new)",
}


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def counts(records):
    out = {}
    for record in records:
        out[record["verdict"]] = out.get(record["verdict"], 0) + 1
    return out


def main():
    run = load(W3)
    scenarios = run["scenarios"]
    summary = run["summary"]
    print("<!-- SUMMARY -->")
    executed = sum(1 for r in scenarios if r["verdict"] != "NOT_RUN")
    print(f"- Declared: **{summary['total']}** · executed: **{executed}** · "
          f"REPRODUCED: **{summary['REPRODUCED']}** · REFUTED: **{summary['REFUTED']}** · "
          f"NOT_RUN: **{summary['NOT_RUN']}** · ERROR: **{summary['ERROR']}**")
    p0 = [r for r in scenarios if r["verdict"] == "REPRODUCED" and r["severity"] == "P0"]
    print(f"- Reproduced by severity: `{summary['reproduced_by_severity']}`; "
          f"P0 reproduced: **{len(p0)}**")
    print(f"- Binary: debug `{run['meta']['flow']['sha256'][:16]}…`, release "
          f"`{(run['meta']['flow'].get('release_sha256') or '')[:16]}…`; "
          f"base commit `{run['meta']['git']['base_sha'][:12]}`")
    print()
    print("<!-- AXES -->")
    print("| Axis | Declared | Executed | REPRODUCED | REFUTED | NOT_RUN | ERROR |")
    print("|---|---|---|---|---|---|---|")
    for axis, data in summary["by_axis"].items():
        executed_axis = data["total"] - data["NOT_RUN"]
        print(f"| {AXIS_TITLES.get(axis, axis)} | {data['total']} | {executed_axis} | "
              f"{data['REPRODUCED']} | {data['REFUTED']} | {data['NOT_RUN']} | "
              f"{data['ERROR']} |")
    print(f"| **Total** | **{summary['total']}** | **{executed}** | "
          f"**{summary['REPRODUCED']}** | **{summary['REFUTED']}** | "
          f"**{summary['NOT_RUN']}** | **{summary['ERROR']}** |")
    print()
    print("<!-- FINDINGS -->")
    order = {"P0": 0, "P1": 1, "P2": 2, "P3": 3}
    reproduced = sorted((r for r in scenarios if r["verdict"] == "REPRODUCED"),
                        key=lambda r: (order.get(r["severity"], 9), r["id"]))
    print("| ID | Severity | Title | Observed (truncated) |")
    print("|---|---|---|---|")
    for record in reproduced:
        observed = record["observed"].replace("|", "\\|").replace("\n", " ")[:180]
        print(f"| {record['id']} | {record['severity']} | {record['title']} | {observed} |")
    print()
    print("<!-- DELTA -->")
    w2 = load(W2)
    replay = load(REPLAY)
    print("| | Wave 2 (`f237d57`, literal 119) | Wave-3 replay (`dfb941d`, literal 119) | "
          "Wave-3 run (`5077c50`, 122) |")
    print("|---|---|---|---|")
    print(f"| REPRODUCED | {w2['summary']['REPRODUCED']} | {replay['summary']['REPRODUCED']} | "
          f"{summary['REPRODUCED']} |")
    print(f"| REFUTED | {w2['summary']['REFUTED']} | {replay['summary']['REFUTED']} | "
          f"{summary['REFUTED']} |")
    print(f"| NOT_RUN | {w2['summary']['NOT_RUN']} | {replay['summary']['NOT_RUN']} | "
          f"{summary['NOT_RUN']} |")
    print(f"| ERROR | {w2['summary']['ERROR']} | {replay['summary']['ERROR']} | "
          f"{summary['ERROR']} |")
    w2_ids = {r["id"]: r for r in w2["scenarios"]}
    w3_ids = {r["id"]: r for r in scenarios}
    changed = [i for i in sorted(w2_ids) if i in w3_ids
               and w2_ids[i]["verdict"] != w3_ids[i]["verdict"]]
    print(f"\nVerdict changed vs wave 2: {changed or 'none'}")
    print("Reproduced wave 2: "
          + ", ".join(f"{r['id']}({r['severity']})" for r in w2["scenarios"]
                      if r["verdict"] == "REPRODUCED"))
    print("Reproduced wave 3: "
          + ", ".join(f"{r['id']}({r['severity']})" for r in scenarios
                      if r["verdict"] == "REPRODUCED"))


if __name__ == "__main__":
    main()
