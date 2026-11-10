#!/usr/bin/env python3
"""Summarise an lcov file for the Flow coverage gate.

Prints, per source file: line coverage, branch coverage, every uncovered branch
line (with the number of uncovered branch points) and every uncovered line, plus
the source context so a test can be written for it. Read-only: it never edits a
source file and never talks to Git.

Usage:
  python analyze_lcov.py <lcov.info> [--repo ROOT] [--file commands.rs]
                         [--context N] [--only-branches] [--product-only]

`--product-only` recomputes the totals over the module's *product* lines only:
every line at or after the file's `#[cfg(test)] mod tests` block is dropped. The
prescribed llvm-cov command instruments unit-test code that lives in `src/`, so
the plain percentage includes the tests themselves; this flag separates the two.
"""

import argparse
import os
import re
import sys
from collections import OrderedDict, defaultdict


def parse_lcov(path):
    files = OrderedDict()
    current = None
    with open(path, "r", encoding="utf-8", errors="replace") as handle:
        for raw in handle:
            line = raw.rstrip("\n")
            if line.startswith("SF:"):
                current = {"lines": {}, "branches": defaultdict(list)}
                files[line[3:]] = current
            elif current is None:
                continue
            elif line.startswith("DA:"):
                number, count = line[3:].split(",")[:2]
                current["lines"][int(number)] = int(float(count))
            elif line.startswith("BRDA:"):
                number, _block, _branch, taken = line[5:].split(",")[:4]
                current["branches"][int(number)].append(taken)
            elif line.startswith("end_of_record"):
                current = None
    return files


def rate(hit, total):
    return 100.0 * hit / total if total else 100.0


def uncovered_branches(record):
    out = {}
    for number, takens in record["branches"].items():
        missed = sum(1 for t in takens if t == "-" or int(float(t)) == 0)
        if missed:
            out[number] = (missed, len(takens))
    return out


def show_file(name, record, repo, context, only_branches):
    lines = record["lines"]
    line_hit = sum(1 for c in lines.values() if c > 0)
    branch_total = sum(len(v) for v in record["branches"].values())
    branch_hit = branch_total - sum(m for m, _ in uncovered_branches(record).values())
    print(
        f"\n== {name}  lines {rate(line_hit, len(lines)):.2f}% ({line_hit}/{len(lines)})"
        f"  branches {rate(branch_hit, branch_total):.2f}% ({branch_hit}/{branch_total})"
    )
    source = None
    if repo:
        rel = re.sub(r"^.*?(crates[\\/]flow[\\/]src[\\/])", r"\1", name)
        candidate = os.path.join(repo, rel.replace("\\", os.sep).replace("/", os.sep))
        if os.path.isfile(candidate):
            with open(candidate, "r", encoding="utf-8", errors="replace") as handle:
                source = handle.read().splitlines()
    missing = sorted(n for n, c in lines.items() if c == 0)
    if not only_branches and missing:
        print("-- uncovered lines: " + ",".join(str(n) for n in missing))
    branches = uncovered_branches(record)
    if not branches:
        return
    print(f"-- uncovered branch lines ({len(branches)}):")
    for number in sorted(branches):
        missed, total = branches[number]
        print(f"   {number}: {missed}/{total} branch points untaken")
        if source and 1 <= number <= len(source):
            lo = max(1, number - context)
            hi = min(len(source), number + context)
            for n in range(lo, hi + 1):
                mark = ">>" if n == number else "  "
                print(f"      {mark}{n:5d} {source[n - 1]}")


def test_block_start(name, repo):
    """1-based line of the `#[cfg(test)]` that introduces `mod tests`, or None."""
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


def product_only(files, repo):
    """Per-file and global coverage over product lines only."""
    rows = []
    total = [0, 0, 0, 0]  # line hit, line total, branch hit, branch total
    for name, record in files.items():
        cutoff = test_block_start(name, repo)
        lines = record["lines"]
        if cutoff is not None:
            lines = {n: c for n, c in lines.items() if n < cutoff}
            branches = {
                n: v for n, v in record["branches"].items() if n < cutoff
            }
        else:
            branches = record["branches"]
        line_hit = sum(1 for c in lines.values() if c > 0)
        branch_total = sum(len(v) for v in branches.values())
        missed = 0
        for takens in branches.values():
            missed += sum(1 for t in takens if t == "-" or int(float(t)) == 0)
        branch_hit = branch_total - missed
        rows.append((name, line_hit, len(lines), branch_hit, branch_total, cutoff))
        total[0] += line_hit
        total[1] += len(lines)
        total[2] += branch_hit
        total[3] += branch_total
    for name, line_hit, line_total, branch_hit, branch_total, cutoff in rows:
        marker = "" if cutoff is None else f"  (test block from line {cutoff} excluded)"
        print(
            f"{name.split('src')[-1].lstrip(chr(92) + '/'):<16}"
            f" lines {rate(line_hit, line_total):6.2f}% ({line_hit}/{line_total})"
            f"  branches {rate(branch_hit, branch_total):6.2f}% ({branch_hit}/{branch_total})"
            f"{marker}"
        )
    print(
        f"\nPRODUCT-ONLY TOTAL lines {rate(total[0], total[1]):.2f}%"
        f" ({total[0]}/{total[1]})  branches {rate(total[2], total[3]):.2f}%"
        f" ({total[2]}/{total[3]})"
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("lcov")
    parser.add_argument("--repo", default=".")
    parser.add_argument("--file", action="append", default=[])
    parser.add_argument("--context", type=int, default=0)
    parser.add_argument("--only-branches", action="store_true")
    parser.add_argument("--product-only", action="store_true")
    args = parser.parse_args()

    files = parse_lcov(args.lcov)
    if args.product_only:
        product_only(files, args.repo)
        return 0
    total_lines = total_hit = total_branches = total_branch_hit = 0
    for name, record in files.items():
        lines = record["lines"]
        line_hit = sum(1 for c in lines.values() if c > 0)
        branch_total = sum(len(v) for v in record["branches"].values())
        missed = sum(m for m, _ in uncovered_branches(record).values())
        total_lines += len(lines)
        total_hit += line_hit
        total_branches += branch_total
        total_branch_hit += branch_total - missed
        if args.file and not any(name.endswith(f) for f in args.file):
            continue
        show_file(name, record, args.repo, args.context, args.only_branches)
    if not args.file:
        print(
            f"\nTOTAL lines {rate(total_hit, total_lines):.2f}% ({total_hit}/{total_lines})"
            f"  branches {rate(total_branch_hit, total_branches):.2f}%"
            f" ({total_branch_hit}/{total_branches})"
        )


if __name__ == "__main__":
    sys.exit(main())
