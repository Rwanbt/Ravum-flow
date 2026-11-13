#!/usr/bin/env python3
"""List the mutants a wave-6 module run did not catch (missed or timed out).

Reads `mutants.out/outcomes.json`, which is the authoritative record: the log
file can lose its trailing summary line when the redirect is closed early.

Usage:
  python list_misses.py [--dir experiments/quality/mutation-w6] [module ...]
"""

import argparse
import json
import os
import sys

ORDER = ["state", "gitcli", "main", "commands"]
KINDS = {"MissedMutant": "missed", "Timeout": "timeout", "Unviable": "unviable"}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--dir", default="experiments/quality/mutation-w6")
    parser.add_argument("modules", nargs="*", default=[])
    args = parser.parse_args()

    for module in args.modules or ORDER:
        path = os.path.join(args.dir, module, "mutants.out", "outcomes.json")
        if not os.path.isfile(path):
            print(f"{module}: NOT_RUN")
            continue
        with open(path, "r", encoding="utf-8") as handle:
            outcomes = json.load(handle).get("outcomes", [])
        rows = [
            (outcome["scenario"]["Mutant"]["name"], outcome.get("summary"))
            for outcome in outcomes
            if isinstance(outcome.get("scenario"), dict) and "Mutant" in outcome["scenario"]
        ]
        interesting = [(name, kind) for name, kind in rows if kind in KINDS and KINDS[kind] != "unviable"]
        unviable = sum(1 for _, kind in rows if kind == "Unviable")
        print(f"{module}: {len(rows)} mutants, {unviable} unviable, {len(interesting)} not caught")
        for name, kind in interesting:
            print(f"   {KINDS.get(kind, kind):<8} {name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
