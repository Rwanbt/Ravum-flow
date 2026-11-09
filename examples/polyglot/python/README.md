# Python fixture — `unittest` obligation

A standard-library-only fixture laid out so `python -m unittest discover -v`
works from the repository root.

## 1. What is here

| Path | Purpose |
|---|---|
| `src/math_util.py` | `add(left, right)` |
| `src/__init__.py`, `tests/__init__.py` | make both directories importable packages |
| `tests/test_math_util.py` | two `unittest` cases |
| `.flow/config.json` | the declared policy (`flow-config/1`, `local-trusted`) |
| `.flow/.gitignore` | `*` — Flow's own state never enters the verified tree |

## 2. Declared policy

```json
{
  "schema": "flow-config/1",
  "policy_version": "FLOW-POLICY-1",
  "execution_boundary": "local-trusted",
  "obligations": [
    { "id": "unittest", "program": "python", "args": ["-m", "unittest", "discover", "-v"], "required": true, "timeout_s": 120 }
  ],
  "disabled": false
}
```

## 3. Exact commands

```bash
python -m unittest discover -v
```

Through Flow from a disposable copy:

```bash
pwsh -NoProfile -File examples/polyglot/run-demo.ps1 -Fixture python
./examples/polyglot/run-demo.sh python
```

## 4. Measured results (this host, 2026-11-09)

Windows 11, `Python 3.13.7`, `git 2.51.1.windows.1`:

| Command | Result |
|---|---|
| `python -m unittest discover -v` | PASS (exit 0): 2 tests, `OK` |
| `flow run` on the fixture index tree | `PASSED`, exit 0, `unittest=PASSED` |
| `flow publish --expected-old <head>` | `PUBLISHED`, exit 0 |

Linux (WSL2 Ubuntu), `Python 3.12.3`: the same obligation ran through Flow and
the demo reported `PASSED` and `PUBLISHED`. This is a local WSL measurement, not
CI evidence.

## 5. The `python` versus `python3` name

The policy declares `program: "python"`, which exists on this Windows host and
in the WSL distribution used above. A host that only provides `python3` cannot
execute this policy as written: the obligation process cannot start and the
result is `UNSUPPORTED`, never `PASSED`. To use such a host, change the program
to `python3` **and adopt the policy again** (`flow approve` in the final CLI;
on the pre-adoption binary in this branch the config is read directly). A policy
edit changes the effective policy digest and invalidates existing receipts; that
is intentional, not a workaround.

## 6. Red path (deliberately available)

`tests/test_math_util.py:13`:

```python
self.assertEqual(add(2, 3), 5)
```

Change `5` to `6`, then `git add -A` and run Flow:

```bash
flow run          # FAILED, exit 1, obligation "unittest" FAILED (python exit 1)
flow publish      # REFUSED, exit 1, code REQUIRED_NOT_PASSED; the ref does not move
```

## 7. Notes

- No third-party package is imported and no network is used.
- `__pycache__/` and `*.pyc` are ignored; Flow runs the materialised copy, so
  bytecode never lands in this directory.
- The import style (`from src.math_util import add`) relies on discovery running
  from the repository root with the `src` package importable, which is exactly
  the declared command.
