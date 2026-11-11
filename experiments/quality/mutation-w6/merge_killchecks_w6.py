#!/usr/bin/env python3
"""Merge the wave-6 campaign with its killchecks (rounds 1-3).

A killcheck re-tests exactly the sites the campaign reported as missed or timed
out, on a tree that contains the assertions that close them, with a wider suite
list. This script never re-runs anything: it matches mutants by full name across
every `killcheck*` output directory and reports the campaign number next to the
final one. A campaign number is not overwritten; it is qualified by what the
narrower run proved.

The campaign files for `state` and `gitcli` are the second (resumed) pass: they
hold every mutant that was not already caught or unviable, which is exactly the
set this script needs (the resolved sites are part of it).

Usage:
  python merge_killchecks_w6.py [--dir experiments/quality/mutation-w6]
"""

import argparse
import glob
import json
import os
import sys

MODULES = ["state", "gitcli", "main", "commands"]
KIND = {
    "Success": "caught",
    "CaughtMutant": "caught",
    "MissedMutant": "missed",
    "Timeout": "timeout",
    "Unviable": "unviable",
    "Failure": "failure",
}


def load(path):
    out = {}
    if not os.path.isfile(path):
        return out
    with open(path, "r", encoding="utf-8") as handle:
        outcomes = json.load(handle).get("outcomes", [])
    for outcome in outcomes:
        scenario = outcome.get("scenario")
        if not isinstance(scenario, dict) or "Mutant" not in scenario:
            continue
        out[scenario["Mutant"]["name"]] = KIND.get(outcome.get("summary"), outcome.get("summary"))
    return out


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--dir", default="experiments/quality/mutation-w6")
    args = parser.parse_args()

    checks = {}
    for pattern in ("killcheck-*", "round*"):
        for directory in sorted(glob.glob(os.path.join(args.dir, pattern))):
            checks.update(load(os.path.join(directory, "mutants.out", "outcomes.json")))

    total_reported = total_resolved = 0
    for module in MODULES:
        campaign = load(os.path.join(args.dir, module, "mutants.out", "outcomes.json"))
        if not campaign:
            print(f"{module}: NOT_RUN")
            continue
        reported = sorted(
            name for name, kind in campaign.items() if kind in ("missed", "timeout")
        )
        resolved = [name for name in reported if checks.get(name) == "caught"]
        residual = [name for name in reported if checks.get(name) != "caught"]
        total_reported += len(reported)
        total_resolved += len(resolved)
        print(f"\n{module}: {len(reported)} reported, {len(resolved)} caught by a killcheck, "
              f"{len(residual)} still not caught")
        for name in residual:
            print(f"  still {checks.get(name, 'not re-tested')}: {name}")
    print(f"\nreported sites: {total_reported}, resolved by a killcheck: {total_resolved}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
