#!/usr/bin/env python3
"""Corrected W3-03 assertion for the 7a034ff campaign run (wave-4 follow-up).

The campaign's `w3_hook_same_length_rewrite` asserts the literal substring
"different bytes" in the refusal reason. `7a034ff` made the message stage-aware
("changed during checkout"), so the run is *correctly refused* but the scenario
falls through to REPRODUCED -- a harness string-coupling defect, not a product
regression. This wrapper re-runs only W3-03 with an assertion on the typed
refusal (code + no receipt), without editing the campaign in
`experiments/mvp-qa/` (out of this round's write scope).

Usage:
  python experiments/verify-p0/w3_03_corrected.py <debug> <release> <out.json>
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "mvp-qa"))
import attacks as A  # noqa: E402


def w3_hook_same_length_rewrite_corrected(c):
    c.prepare([A.ob_green()], staged=True)
    c.write_file("README.md", b"hello\n")
    c.stage("README.md")
    A._install_tracked_hook(c, "printf 'HELLO\\n' > README.md")
    tree = c.git_out("write-tree")
    run = c.flow("run")
    blob = c.git_out("cat-file", "-p", f"{tree}:README.md")
    evidence = {"run": run.brief(), "tree": tree, "blob": blob,
                "receipt": c.receipt_for_tree(tree).exists()}
    if (run.exit == 1 and run.code == "UNSUPPORTED_CHECKOUT"
            and not c.receipt_for_tree(tree).exists()):
        return A.X("a same-length rewrite by post-checkout is refused by the "
                   "byte comparison (no receipt), not certified",
                   f"refused ({run.code}): {run.reason[:260]}", **evidence)
    return A.R("a same-length hook rewrite must be refused by the byte comparison",
               f"run={run.brief()} receipt={evidence['receipt']}", **evidence)


def main(argv):
    if len(argv) < 4:
        print(__doc__)
        return 2
    for index, scenario in enumerate(A.SCENARIOS):
        if scenario.id == "W3-03":
            A.SCENARIOS[index] = A.Scn(
                "W3-03", "fidelity", "post-checkout rewrites a file at equal "
                "byte length", "P0", w3_hook_same_length_rewrite_corrected)
    return A.main([
        "--flow", argv[1],
        "--release-flow", argv[2],
        "--filter", "W3-03",
        "--out", argv[3],
        "--base-sha", "7a034ffb855ccd020f1abdd46034c7bd2c33d416",
    ])


if __name__ == "__main__":
    sys.exit(main(sys.argv))
