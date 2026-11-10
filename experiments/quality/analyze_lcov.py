#!/usr/bin/env python3
"""Summarise an lcov file for the Flow coverage gate.

Prints, per source file: line coverage, branch coverage, every uncovered branch
line (with the number of uncovered branch points) and every uncovered line, plus
the source context so a test can be written for it. Read-only: it never edits a
source file and never talks to Git.

Usage:
  python analyze_lcov.py <lcov.info> [--repo ROOT] [--file commands.rs]
                         [--context N] [--only-branches] [--product-only]
                         [--fail-under-lines PCT] [--fail-under-branches PCT]
                         [--fail-under-functions PCT] [--waivers FILE]

`--product-only` recomputes the totals over the module's *product* lines only:
every line at or after the file's `#[cfg(test)] mod tests` block is dropped. The
prescribed llvm-cov command instruments unit-test code that lives in `src/`, so
the plain percentage includes the tests themselves; this flag separates the two.

`--waivers` names one line per excluded line, with the proof that the line cannot
be reached:

  commands.rs:873  a 100644 entry cannot be checked out as anything but a file

A waiver that no longer matches an instrumented line, or whose line is in fact
covered, is reported as stale and fails the gate: an exclusion that silently
outlives its proof is exactly how a coverage gate rots. `--fail-under-*` exit 1
below the threshold (and on any stale waiver), so the same script can enforce the
gate in CI.
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
                current = {
                    "lines": {},
                    "branches": defaultdict(list),
                    "functions": {},
                }
                files[line[3:]] = current
            elif current is None:
                continue
            elif line.startswith("DA:"):
                number, count = line[3:].split(",")[:2]
                current["lines"][int(number)] = int(float(count))
            elif line.startswith("BRDA:"):
                number, _block, _branch, taken = line[5:].split(",")[:4]
                current["branches"][int(number)].append(taken)
            elif line.startswith("FN:"):
                number, _, name = line[3:].partition(",")
                current["functions"].setdefault(name, {"line": int(number), "hits": 0})
            elif line.startswith("FNDA:"):
                count, _, name = line[5:].partition(",")
                entry = current["functions"].setdefault(name, {"line": 0, "hits": 0})
                entry["hits"] = int(float(count))
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


def product_rows(files, repo, product_only):
    """Per-file coverage, optionally cut at the module's test block."""
    rows = []
    for name, record in files.items():
        cutoff = test_block_start(name, repo) if product_only else None
        if cutoff is not None:
            lines = {n: c for n, c in record["lines"].items() if n < cutoff}
            branches = {n: v for n, v in record["branches"].items() if n < cutoff}
            functions = {
                f: v for f, v in record["functions"].items() if v["line"] < cutoff
            }
        else:
            lines = record["lines"]
            branches = record["branches"]
            functions = record["functions"]
        rows.append(
            {
                "name": name,
                "cutoff": cutoff,
                "lines": lines,
                "branches": branches,
                "functions": functions,
                "line_hit": sum(1 for c in lines.values() if c > 0),
                "line_total": len(lines),
                "branch_total": sum(len(v) for v in branches.values()),
                "fn_hit": sum(1 for f in functions.values() if f["hits"] > 0),
                "fn_total": len(functions),
            }
        )
        missed = 0
        for takens in branches.values():
            missed += sum(1 for t in takens if t == "-" or int(float(t)) == 0)
        rows[-1]["branch_hit"] = rows[-1]["branch_total"] - missed
    return rows


def row_uncovered_branch_lines(row):
    return uncovered_branches({"branches": row["branches"]})


def load_waivers(path, files):
    """Parse `file:line reason` lines; refuse malformed or duplicate entries."""
    entries = []
    errors = []
    seen = set()
    with open(path, "r", encoding="utf-8") as handle:
        for number, raw in enumerate(handle, start=1):
            text = raw.strip()
            if not text or text.startswith("#"):
                continue
            match = re.match(r"^([A-Za-z0-9_.\\/-]+\.rs):(\d+)\s+(\S.*)$", text)
            if not match:
                errors.append(f"{path}:{number}: not `file.rs:line reason`")
                continue
            base, line, reason = match.group(1), int(match.group(2)), match.group(3)
            if len(reason.strip()) < 10:
                errors.append(f"{path}:{number}: the waiver needs a real reason")
                continue
            if (base, line) in seen:
                errors.append(f"{path}:{number}: duplicate waiver {base}:{line}")
                continue
            seen.add((base, line))
            name = next((n for n in files if n.endswith(base)), None)
            if name is None:
                errors.append(f"{path}:{number}: no measured source matches {base}")
                continue
            entries.append({"file": name, "line": line, "reason": reason.strip()})
    return entries, errors


def apply_waivers(rows, waivers):
    """Remove waived points from the denominator; report stale waivers."""
    by_name = {row["name"]: row for row in rows}
    applied = 0
    stale = []
    excluded_lines = 0
    excluded_branches = 0
    for waiver in waivers:
        row = by_name[waiver["file"]]
        line = waiver["line"]
        in_lines = line in row["lines"]
        in_branches = line in row["branches"]
        if not in_lines and not in_branches:
            stale.append(f"{waiver['file']}:{line}: no instrumented point on that line")
            continue
        line_missed = 1 if (in_lines and row["lines"][line] == 0) else 0
        branch_missed = 0
        if in_branches:
            branch_missed = sum(
                1 for t in row["branches"][line] if t == "-" or int(float(t)) == 0
            )
        if line_missed == 0 and branch_missed == 0:
            stale.append(f"{waiver['file']}:{line}: waived but covered")
            continue
        excluded_lines += line_missed
        excluded_branches += branch_missed
        applied += 1
    return applied, stale, excluded_lines, excluded_branches


def print_table(rows):
    for row in rows:
        name = row["name"].split("src")[-1].lstrip("\\/")
        marker = "" if row["cutoff"] is None else f"  (test block from line {row['cutoff']} excluded)"
        uncovered = row_uncovered_branch_lines(row)
        print(
            f"{name:<16}"
            f" lines {rate(row['line_hit'], row['line_total']):6.2f}%"
            f" ({row['line_hit']}/{row['line_total']})"
            f"  branches {rate(row['branch_hit'], row['branch_total']):6.2f}%"
            f" ({row['branch_hit']}/{row['branch_total']})"
            f"  functions {rate(row['fn_hit'], row['fn_total']):6.2f}%"
            f" ({row['fn_hit']}/{row['fn_total']})"
            f"  uncovered-branch-lines {len(uncovered)}"
            f"{marker}"
        )


def show_file(name, row, repo, context, only_branches):
    print(
        f"\n== {name}  lines {rate(row['line_hit'], row['line_total']):.2f}%"
        f" ({row['line_hit']}/{row['line_total']})"
        f"  branches {rate(row['branch_hit'], row['branch_total']):.2f}%"
        f" ({row['branch_hit']}/{row['branch_total']})"
    )
    source = None
    if repo:
        rel = re.sub(r"^.*?(crates[\\/]flow[\\/]src[\\/])", r"\1", name)
        candidate = os.path.join(repo, rel.replace("\\", os.sep).replace("/", os.sep))
        if os.path.isfile(candidate):
            with open(candidate, "r", encoding="utf-8", errors="replace") as handle:
                source = handle.read().splitlines()
    missing = sorted(n for n, c in row["lines"].items() if c == 0)
    if not only_branches and missing:
        print("-- uncovered lines: " + ",".join(str(n) for n in missing))
    branches = row_uncovered_branch_lines(row)
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


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("lcov")
    parser.add_argument("--repo", default=".")
    parser.add_argument("--file", action="append", default=[])
    parser.add_argument("--context", type=int, default=0)
    parser.add_argument("--only-branches", action="store_true")
    parser.add_argument("--product-only", action="store_true")
    parser.add_argument("--fail-under-lines", type=float, default=None)
    parser.add_argument("--fail-under-branches", type=float, default=None)
    parser.add_argument("--fail-under-functions", type=float, default=None)
    parser.add_argument("--waivers", default=None)
    args = parser.parse_args()

    files = parse_lcov(args.lcov)
    rows = product_rows(files, args.repo, args.product_only)
    gate = (
        args.fail_under_lines is not None
        or args.fail_under_branches is not None
        or args.fail_under_functions is not None
    )

    # Detail mode: the per-file report used to write tests.
    if not gate and not args.product_only:
        for row in rows:
            if args.file and not any(row["name"].endswith(f) for f in args.file):
                continue
            show_file(row["name"], row, args.repo, args.context, args.only_branches)
        if not args.file:
            hit = sum(r["line_hit"] for r in rows)
            total = sum(r["line_total"] for r in rows)
            bhit = sum(r["branch_hit"] for r in rows)
            btotal = sum(r["branch_total"] for r in rows)
            print(
                f"\nTOTAL lines {rate(hit, total):.2f}% ({hit}/{total})"
                f"  branches {rate(bhit, btotal):.2f}% ({bhit}/{btotal})"
            )
        return 0

    print_table(rows)
    line_hit = sum(r["line_hit"] for r in rows)
    line_total = sum(r["line_total"] for r in rows)
    branch_hit = sum(r["branch_hit"] for r in rows)
    branch_total = sum(r["branch_total"] for r in rows)
    fn_hit = sum(r["fn_hit"] for r in rows)
    fn_total = sum(r["fn_total"] for r in rows)

    excluded_lines = 0
    excluded_branches = 0
    stale = []
    applied = 0
    if args.waivers:
        waivers, errors = load_waivers(args.waivers, files)
        if errors:
            for error in errors:
                print(f"::error::waiver: {error}", file=sys.stderr)
            return 1
        applied, stale, excluded_lines, excluded_branches = apply_waivers(rows, waivers)
        line_total -= excluded_lines
        branch_total -= excluded_branches
        fn_total -= 0

    label = "PRODUCT-ONLY TOTAL" if args.product_only else "TOTAL"
    print(
        f"\n{label} lines {rate(line_hit, line_total):.2f}% ({line_hit}/{line_total})"
        f"  branches {rate(branch_hit, branch_total):.2f}% ({branch_hit}/{branch_total})"
        f"  functions {rate(fn_hit, fn_total):.2f}% ({fn_hit}/{fn_total})"
    )
    if args.waivers:
        print(
            f"waivers: {applied} applied, {len(stale)} stale"
            f" (excluded {excluded_lines} lines, {excluded_branches} branch points)"
        )
        for item in stale:
            print(f"::error::stale waiver: {item}", file=sys.stderr)

    failed = False
    checks = [
        ("lines", rate(line_hit, line_total), args.fail_under_lines),
        ("branches", rate(branch_hit, branch_total), args.fail_under_branches),
        ("functions", rate(fn_hit, fn_total), args.fail_under_functions),
    ]
    for label_name, measured, minimum in checks:
        if minimum is None:
            continue
        verdict = "ok" if measured >= minimum else "FAILED"
        print(f"gate {label_name}: {measured:.2f}% >= {minimum:.2f}%  {verdict}")
        if measured < minimum:
            failed = True
    if stale:
        failed = True
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
