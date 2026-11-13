#!/usr/bin/env python3
"""Summarise the wave-6 per-module mutation runs.

Reads each `mutants.out/outcomes.json`, counts what happened, and separates the
survivors that live in a module's `#[cfg(test)]` block (mutating a test's own
assertion) from survivors in product code. A mutant that does not compile is
"unviable" and is excluded from every rate; a timeout is reported separately.

A survivor in a `#[cfg(unix)]`/`#[cfg(not(windows))]` block is listed as missed
on this Windows host because the mutated line is not compiled at all; wave 6
classifies those by hand instead of counting them as killed or as a gap.

Usage:
  python summarize.py [--dir experiments/quality/mutation-w6] [--repo .]
                      [--misses]
"""

import argparse
import json
import os
import re
import sys

ORDER = ["state", "gitcli", "main", "commands"]

KINDS = {
    "Success": "caught",
    "CaughtMutant": "caught",
    "MissedMutant": "missed",
    "Timeout": "timeout",
    "Unviable": "unviable",
    "Failure": "failure",
}


def test_block_start(name, repo):
    rel = re.sub(r"^.*?(crates[\\/]flow[\\/]src[\\/])", r"\1", name)
    candidate = os.path.join(repo, rel.replace("\\", os.sep).replace("/", os.sep))
    if not os.path.isfile(candidate):
        return None
    with open(candidate, "r", encoding="utf-8", errors="replace") as handle:
        lines = handle.read().splitlines()
    start = None
    for index, line in enumerate(lines):
        if line.strip() == "#[cfg(test)]":
            window = lines[index + 1 : index + 4]
            if any(w.strip().startswith("mod tests") for w in window):
                start = index + 1
    return start


def load(module_dir):
    path = os.path.join(module_dir, "mutants.out", "outcomes.json")
    if not os.path.isfile(path):
        return None
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle).get("outcomes", [])


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--dir", default="experiments/quality/mutation-w6")
    parser.add_argument("--repo", default=".")
    parser.add_argument("--misses", action="store_true")
    args = parser.parse_args()

    rows = []
    totals = dict(caught=0, missed=0, timeout=0, unviable=0, failure=0)
    product = dict(caught=0, missed=0, timeout=0)
    test_code_misses = 0
    for module in ORDER:
        module_dir = os.path.join(args.dir, module)
        outcomes = load(module_dir)
        if outcomes is None:
            rows.append((module, None))
            continue
        counts = dict(caught=0, missed=0, timeout=0, unviable=0, failure=0)
        cutoff = test_block_start(f"crates/flow/src/{module}.rs", args.repo)
        misses = []
        for outcome in outcomes:
            scenario = outcome.get("scenario")
            if not isinstance(scenario, dict) or "Mutant" not in scenario:
                continue  # baseline
            kind = KINDS.get(outcome.get("summary"), outcome.get("summary"))
            counts[kind] = counts.get(kind, 0) + 1
            mutant = scenario["Mutant"]
            in_test = cutoff is not None and int(mutant.get("line") or 0) >= cutoff
            if kind == "missed":
                misses.append((mutant.get("name"), in_test))
                if in_test:
                    test_code_misses += 1
            if kind in ("caught", "missed", "timeout") and not in_test:
                product[kind] += 1
        for key in totals:
            totals[key] += counts.get(key, 0)
        rows.append((module, (counts, misses)))

    print(f"{'module':<9} {'caught':>7} {'missed':>7} {'timeout':>8} {'unviable':>9} {'kill%':>7}")
    for module, data in rows:
        if data is None:
            print(f"{module:<9} {'-':>7} {'-':>7} {'-':>8} {'-':>9} {'NOT_RUN':>7}")
            continue
        counts, _ = data
        tested = counts["caught"] + counts["missed"] + counts["timeout"]
        rate = 100.0 * counts["caught"] / tested if tested else 0.0
        print(
            f"{module:<9} {counts['caught']:>7} {counts['missed']:>7}"
            f" {counts['timeout']:>8} {counts['unviable']:>9} {rate:>6.1f}%"
        )
    tested = totals["caught"] + totals["missed"] + totals["timeout"]
    rate = 100.0 * totals["caught"] / tested if tested else 0.0
    print(
        f"{'ALL':<9} {totals['caught']:>7} {totals['missed']:>7}"
        f" {totals['timeout']:>8} {totals['unviable']:>9} {rate:>6.1f}%"
    )
    ptested = product["caught"] + product["missed"] + product["timeout"]
    prate = 100.0 * product["caught"] / ptested if ptested else 0.0
    print(
        f"\nproduct code only: {product['caught']} caught / {ptested} tested"
        f"  ({prate:.1f} %)  missed {product['missed']}, timeout {product['timeout']}"
    )
    print(f"survivors inside a module's own `#[cfg(test)]` block: {test_code_misses}")

    if args.misses:
        print("\nmissed mutants (product code):")
        for module, data in rows:
            if data is None:
                continue
            _, misses = data
            for name, in_test in misses:
                if not in_test:
                    print(f"  {name}")


if __name__ == "__main__":
    sys.exit(main())
