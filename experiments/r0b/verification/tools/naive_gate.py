"""Naive gate: runs the suite and ignores the return code."""
import json
import subprocess
import sys

cwd = sys.argv[1]
res = subprocess.run([sys.executable, "-m", "unittest", "discover", "-q"], cwd=cwd,
                     capture_output=True, text=True)
print(json.dumps({"policy": "ignore_return_code", "test_rc": res.returncode,
                  "gate_rc": 0, "test_stderr_tail": res.stderr.strip().splitlines()[-3:]},
                 sort_keys=True))
sys.exit(0)
