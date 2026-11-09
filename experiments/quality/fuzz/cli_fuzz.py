#!/usr/bin/env python3
"""Seeded, deterministic mutation fuzzer for the `flow` CLI option parser.

Target: the argument grammar implemented in `crates/flow/src/main.rs`
(`value_flags`, `BOOL_FLAGS`, `validate_flags`, `collect_values`, `flag_value`,
`has_flag`) and the command dispatch behind it. The binary is driven as a black
box on a disposable fixture repository, exactly as a user would invoke it.

Invariants checked per invocation
---------------------------------
* the process never panics (`panicked at` must not appear on stderr);
* the exit code is one of the three documented codes: 0, 1, or 2;
* stdout is empty or a single valid JSON document (the CLI contract);
* no invocation exceeds the per-run timeout (a hang is a finding).

What it does **not** cover: the semantics of commands with a valid, meaningful
argument set (that is the integration suite's job), the Git object model, or
memory safety (safe Rust). `cargo-fuzz` is not used: it needs libFuzzer and a
nightly target directory, and this harness must reproduce a seed exactly.

Reproduce
---------
    python experiments/quality/fuzz/cli_fuzz.py --seconds 60
    python experiments/quality/fuzz/cli_fuzz.py --seed 20261110 --iters 500
    python experiments/quality/fuzz/cli_fuzz.py --replay crashes/finding-0001.json
"""

from __future__ import annotations

import argparse
import json
import os
import random
import shutil
import stat
import subprocess
import sys
import tempfile
import time

HERE = os.path.dirname(os.path.abspath(__file__))
WORKSPACE = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
DEFAULT_OUT = HERE  # experiments/quality/fuzz
DEFAULT_FLOW = os.path.join(WORKSPACE, "target", "debug", "flow.exe")

COMMANDS = [
    "init",
    "approve",
    "doctor",
    "status",
    "run",
    "publish",
    "propose",
    "proposals",
    "assemble",
    "reconcile",
    "explain",
    "disable",
    "enable",
    "version",
    "help",
    "",
]

FLAGS = [
    "--obligation",
    "--obligations",
    "--timeout",
    "--candidate",
    "--ref",
    "--expected-old",
    "--message",
    "--agent",
    "--task",
    "--base",
    "--patch",
    "--key",
    "--proposal",
    "--tree",
    "--dry-run",
    "--json",
    "--force",
    "--adopt-policy",
    "--help",
    "--unknown",
    "--",
    "--=",
    "---",
]

VALUES = [
    "",
    "x",
    "1",
    "0",
    "-1",
    "HEAD",
    "main",
    "refs/heads/main",
    "0000000000000000000000000000000000000000",
    "deadbeef",
    ".",
    "..",
    "../x",
    "a/b",
    "a\\b",
    "CON",
    "file with spaces",
    "--dry-run",
    "--",
    "green",
    "list",
    "show",
    "--timeout=1",
    "--candidate=",
    "=x",
    "é中🦀",
    "x" * 300,
    "\x1f",
]

# A NUL byte is deliberately absent from VALUES: no OS can carry it in argv, so
# generating it would only measure the harness, not the CLI (it fails at
# CreateProcess/execve before the program starts).

SUBCMDS = ["list", "show", "bogus", ""]

# Seeds are whole argument vectors, not single tokens: mutation of a realistic
# vector reaches states that random token soup does not.
SEED_VECTORS = [
    ["version"],
    ["help"],
    ["status"],
    ["doctor"],
    ["run"],
    ["run", "--obligation", "green"],
    ["run", "--timeout", "5"],
    ["run", "--candidate", "0123456789abcdef0123456789abcdef"],
    ["publish", "--dry-run"],
    ["publish", "--ref", "main", "--expected-old", "HEAD"],
    ["publish", "--message", "m", "--dry-run"],
    ["propose", "--agent", "a", "--task", "t", "--base", "HEAD", "--patch", "p.patch"],
    ["proposals", "list"],
    ["proposals", "show", "0123456789abcdef0123456789abcdef"],
    ["assemble"],
    ["assemble", "--proposal", "0123456789abcdef0123456789abcdef"],
    ["explain"],
    ["explain", "--tree", "HEAD"],
    ["init", "--force"],
    ["disable"],
    ["enable"],
    ["reconcile"],
    ["bogus"],
    [],
]


def clear_readonly(path: str) -> None:
    for root, dirs, files in os.walk(path):
        for name in dirs + files:
            full = os.path.join(root, name)
            try:
                mode = os.lstat(full).st_mode
                if not stat.S_ISLNK(mode):
                    os.chmod(full, stat.S_IWRITE | stat.S_IREAD | stat.S_IEXEC)
            except OSError:
                pass


def remove_tree(path: str) -> None:
    if not os.path.exists(path):
        return
    clear_readonly(path)
    shutil.rmtree(path, ignore_errors=True)


def git(repo: str, *args: str) -> str:
    out = subprocess.run(
        ["git", "-C", repo, *args], capture_output=True, timeout=120, check=False
    )
    if out.returncode != 0:
        raise RuntimeError(
            f"git {args} failed: {out.stderr.decode('utf-8', 'replace')[:400]}"
        )
    return out.stdout.decode("utf-8", "replace").strip()


def make_fixture(flow: str, root: str) -> str:
    os.makedirs(root, exist_ok=True)
    git(root, "init", "-q", "-b", "main")
    git(root, "config", "user.name", "Flow Fuzz")
    git(root, "config", "user.email", "flow@example.invalid")
    git(root, "config", "commit.gpgsign", "false")
    with open(os.path.join(root, "alpha.txt"), "w", encoding="utf-8") as handle:
        handle.write("one\ntwo\nthree\n")
    git(root, "add", "-A")
    git(root, "commit", "-q", "-m", "base")
    subprocess.run([flow, "init"], cwd=root, capture_output=True, timeout=120)
    config = {
        "schema": "flow-config/1",
        "policy_version": "FLOW-POLICY-1",
        "execution_boundary": "local-trusted",
        "obligations": [
            {
                "id": "green",
                "program": "git",
                "args": ["--version"],
                "required": True,
                "timeout_s": 30,
            }
        ],
        "disabled": False,
    }
    os.makedirs(os.path.join(root, ".flow"), exist_ok=True)
    with open(os.path.join(root, ".flow", "config.json"), "w", encoding="utf-8") as handle:
        json.dump(config, handle)
    subprocess.run([flow, "approve"], cwd=root, capture_output=True, timeout=120)
    return root


def mutate(rng: random.Random, corpus: list[list[str]]) -> list[str]:
    vector = list(rng.choice(corpus))
    rounds = 1 + rng.randrange(5)
    for _ in range(rounds):
        op = rng.randrange(8)
        if op == 0 or not vector:
            at = rng.randrange(len(vector) + 1)
            vector.insert(at, rng.choice(FLAGS + VALUES + COMMANDS + SUBCMDS))
        elif op == 1:
            del vector[rng.randrange(len(vector))]
        elif op == 2:
            vector[rng.randrange(len(vector))] = rng.choice(FLAGS + VALUES + COMMANDS)
        elif op == 3:
            at = rng.randrange(len(vector))
            vector[at] = vector[at] + "=" + rng.choice(VALUES)
        elif op == 4:
            del vector[rng.randrange(len(vector)) :]
        elif op == 5:
            at = rng.randrange(len(vector) + 1)
            vector[at:at] = [rng.choice(FLAGS), rng.choice(VALUES)]
        elif op == 6:
            other = list(rng.choice(corpus))
            if other:
                start = rng.randrange(len(other))
                chunk = other[start : start + 1 + rng.randrange(len(other) - start)]
                at = rng.randrange(len(vector) + 1)
                vector[at:at] = chunk
        else:
            if len(vector) >= 2:
                a = rng.randrange(len(vector))
                b = rng.randrange(len(vector))
                vector[a], vector[b] = vector[b], vector[a]
    return vector


# `help` / `-h` / `--help` and an empty argv print the usage text instead of
# JSON. That is documented behaviour, so it is not a contract violation.
USAGE_FIRST = {"help", "-h", "--help"}


def usage_allowed(args: list[str]) -> bool:
    return not args or (bool(args) and args[0] in USAGE_FIRST)


def check(flow: str, repo: str, args: list[str], timeout: float) -> dict:
    """Run one invocation; return a dict with the finding or None."""
    started = time.monotonic()
    try:
        run = subprocess.run(
            [flow, *args],
            cwd=repo,
            capture_output=True,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired:
        return {
            "kind": "timeout",
            "detail": f"no exit within {timeout}s",
            "args": args,
        }
    elapsed = time.monotonic() - started
    stdout = run.stdout.decode("utf-8", "replace")
    stderr = run.stderr.decode("utf-8", "replace")
    if "panicked at" in stderr or "RUST_BACKTRACE" in stderr:
        return {
            "kind": "panic",
            "detail": stderr[:2000],
            "args": args,
            "stdout": stdout[:2000],
        }
    if run.returncode not in (0, 1, 2):
        return {
            "kind": "unexpected-exit",
            "detail": f"exit code {run.returncode}",
            "args": args,
            "stdout": stdout[:2000],
            "stderr": stderr[:2000],
        }
    if stdout.strip() and not usage_allowed(args):
        try:
            json.loads(stdout)
        except json.JSONDecodeError as err:
            return {
                "kind": "non-json-stdout",
                "detail": f"{err}: {stdout[:500]}",
                "args": args,
            }
    return {"kind": None, "code": run.returncode, "elapsed": elapsed}


def save_finding(out: str, index: int, finding: dict, seed: int) -> str:
    crash_dir = os.path.join(out, "crashes")
    os.makedirs(crash_dir, exist_ok=True)
    path = os.path.join(crash_dir, f"finding-{index:04}.json")
    payload = {
        "harness": "experiments/quality/fuzz/cli_fuzz.py",
        "seed": seed,
        "kind": finding["kind"],
        "detail": finding.get("detail", ""),
        "argv": finding["args"],
        "repro": "[flow, *argv] in a disposable repository",
    }
    for key in ("stdout", "stderr"):
        if key in finding:
            payload[key] = finding[key]
    with open(path, "w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2)
    return path


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--flow", default=DEFAULT_FLOW)
    parser.add_argument("--seed", type=int, default=20261110)
    parser.add_argument("--seconds", type=float, default=60.0)
    parser.add_argument("--iters", type=int, default=0)
    parser.add_argument("--timeout", type=float, default=20.0)
    parser.add_argument("--out", default=DEFAULT_OUT)
    parser.add_argument("--replay", default="")
    parser.add_argument(
        "--keep-fixture", action="store_true", help="do not delete the fixture repo"
    )
    options = parser.parse_args(argv)

    flow = os.path.abspath(options.flow)
    if not os.path.exists(flow):
        print(f"flow binary not found: {flow} (build it with `cargo build`)", file=sys.stderr)
        return 2

    if options.replay:
        with open(options.replay, "r", encoding="utf-8") as handle:
            payload = json.load(handle)
        root = make_fixture(flow, tempfile.mkdtemp(prefix="flow-fuzz-replay-"))
        try:
            result = check(flow, root, list(payload["argv"]), options.timeout)
            print(json.dumps({"replay": options.replay, "result": result}, indent=2))
            return 0 if result["kind"] is None else 1
        finally:
            if not options.keep_fixture:
                remove_tree(root)

    corpus_dir = os.path.join(options.out, "corpus")
    os.makedirs(corpus_dir, exist_ok=True)
    print(f"SEED={options.seed}")
    print("TARGET=flow CLI option parser (crates/flow/src/main.rs)")
    print(f"FLOW={flow}")
    print(f"OUT={options.out}")

    root = make_fixture(flow, tempfile.mkdtemp(prefix="flow-fuzz-"))
    rng = random.Random(options.seed)
    corpus = [list(seed) for seed in SEED_VECTORS]
    counters = {"ok": 0, "refused": 0, "usage": 0, "finding": 0}
    findings: list[dict] = []
    saved_corpus = 0
    started = time.monotonic()
    iterations = 0
    try:
        while True:
            if options.iters:
                if iterations >= options.iters:
                    break
            elif time.monotonic() - started >= options.seconds:
                break
            args = mutate(rng, corpus)
            iterations += 1
            result = check(flow, root, args, options.timeout)
            if result["kind"] is not None:
                counters["finding"] += 1
                findings.append(result)
                path = save_finding(options.out, counters["finding"], result, options.seed)
                print(
                    f"FINDING kind={result['kind']} iteration={iterations} saved={path}",
                    file=sys.stderr,
                )
                if result["kind"] in ("panic", "unexpected-exit"):
                    break
            else:
                if result["code"] == 0:
                    counters["ok"] += 1
                    if saved_corpus < 100 and args not in corpus:
                        saved_corpus += 1
                        corpus.append(args)
                        with open(
                            os.path.join(corpus_dir, f"accepted-{saved_corpus:03}.json"),
                            "w",
                            encoding="utf-8",
                        ) as handle:
                            json.dump(args, handle)
                elif result["code"] == 2:
                    counters["usage"] += 1
                else:
                    counters["refused"] += 1
            if iterations % 200 == 0:
                elapsed = time.monotonic() - started
                print(
                    f"progress iterations={iterations} ok={counters['ok']} "
                    f"usage={counters['usage']} refused={counters['refused']} "
                    f"findings={counters['finding']} elapsed={elapsed:.1f}s",
                    file=sys.stderr,
                )
    finally:
        if not options.keep_fixture:
            remove_tree(root)

    elapsed = time.monotonic() - started
    stats = {
        "harness": "experiments/quality/fuzz/cli_fuzz.py",
        "target": "flow CLI option parser (crates/flow/src/main.rs)",
        "seed": options.seed,
        "iterations": iterations,
        "ok": counters["ok"],
        "usage": counters["usage"],
        "refused": counters["refused"],
        "findings": counters["finding"],
        "elapsed_s": round(elapsed, 3),
        "seconds_budget": options.seconds,
        "per_run_timeout_s": options.timeout,
        "toolchain": {
            "python": sys.version.split()[0],
            "platform": sys.platform,
        },
        "note": "seed fixes the argument sequence; with --seconds the prefix length depends on machine speed",
    }
    if findings:
        stats["first_finding"] = {
            "kind": findings[0]["kind"],
            "detail": findings[0].get("detail", "")[:1000],
            "argv": findings[0]["args"],
        }
    with open(os.path.join(options.out, "stats.json"), "w", encoding="utf-8") as handle:
        json.dump(stats, handle, indent=2)
    print(f"ITERATIONS={iterations}")
    print(f"OK={counters['ok']}")
    print(f"USAGE={counters['usage']}")
    print(f"REFUSED={counters['refused']}")
    print(f"FINDINGS={counters['finding']}")
    print(f"ELAPSED_S={elapsed:.3f}")
    print("RESULT=" + ("PASSED" if not findings else "FAILED"))
    return 0 if not findings else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
