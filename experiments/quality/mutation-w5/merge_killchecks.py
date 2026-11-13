#!/usr/bin/env python3
"""Merge the wave-5 campaign with its targeted killchecks.

A targeted killcheck re-tests the exact mutation sites the campaign reported as
survivors or timeouts, after the assertions that close them. This script does not
re-run anything: it reads each `mutants.out/outcomes.json`, matches mutants by
their full name (file:line:column + replacement), and reports the campaign state
and the post-killcheck state side by side. A campaign number is never overwritten;
it is qualified by what a later, narrower run proved.

Usage:
  python merge_killchecks.py [--dir experiments/quality/mutation-w5]
"""

import argparse
import glob
import json
import os
import sys

MODULES = ["proposal", "scratch"]
KIND = {
    "Success": "caught",
    "CaughtMutant": "caught",
    "MissedMutant": "missed",
    "Timeout": "timeout",
    "Unviable": "unviable",
    "Failure": "failure",
}


def load(path):
    """name -> summary for every mutant outcome in one run."""
    out = {}
    if not os.path.isfile(path):
        return out
    with open(path, "r", encoding="utf-8") as handle:
        outcomes = json.load(handle).get("outcomes", [])
    for outcome in outcomes:
        scenario = outcome.get("scenario")
        if not isinstance(scenario, dict) or "Mutant" not in scenario:
            continue
        name = scenario["Mutant"]["name"]
        out[name] = KIND.get(outcome.get("summary"), outcome.get("summary"))
    return out


def killchecks(directory):
    merged = {}
    for path in sorted(glob.glob(os.path.join(directory, "killcheck-*", "mutants.out", "outcomes.json"))):
        for name, kind in load(path).items():
            # A later, narrower run supersedes the campaign for that exact mutant.
            merged[name] = kind
    return merged


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--dir", default="experiments/quality/mutation-w5")
    args = parser.parse_args()

    checks = killchecks(args.dir)
    total = {"campaign": {}, "final": {}}
    rows = []
    for module in MODULES:
        campaign = load(os.path.join(args.dir, module, "mutants.out", "outcomes.json"))
        if not campaign:
            rows.append((module, None, None))
            continue
        final = {name: checks.get(name, kind) for name, kind in campaign.items()}
        rows.append((module, campaign, final))
        for label, data in (("campaign", campaign), ("final", final)):
            counts = total[label].setdefault(module, {"caught": 0, "missed": 0, "timeout": 0, "unviable": 0, "failure": 0})
            for kind in data.values():
                counts[kind] += 1

    print(f"{'module':<10} {'state':<9} {'mutants':>7} {'caught':>7} {'missed':>7} {'timeout':>7} {'unviable':>8} {'strict':>8}")
    for module, campaign, final in rows:
        if campaign is None:
            print(f"{module:<10} {'-':<9} {'NOT_RUN':>7}")
            continue
        for label, data in (("campaign", campaign), ("final", final)):
            c = total[label][module]
            applicable = c["caught"] + c["missed"] + c["timeout"]
            strict = 100.0 * c["caught"] / applicable if applicable else 100.0
            print(
                f"{module:<10} {label:<9} {len(data):>7} {c['caught']:>7} {c['missed']:>7}"
                f" {c['timeout']:>7} {c['unviable']:>8} {strict:>7.1f}%"
            )

    print()
    for module, campaign, final in rows:
        if campaign is None:
            continue
        resolved = sum(
            1
            for name, kind in campaign.items()
            if kind in ("missed", "timeout") and final[name] == "caught"
        )
        print(f"{module}: {resolved} survivor(s)/timeout(s) resolved by a targeted killcheck")
        for name, kind in sorted(final.items()):
            if kind in ("missed", "timeout"):
                print(f"  still {kind}: {name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
