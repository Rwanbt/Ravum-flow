"""Naive reader: trusts every line to be JSON. A truncated line crashes it."""
import json
import subprocess
import sys
from pathlib import Path

repo = Path(sys.argv[1])
gd = subprocess.run(["git", "-C", str(repo), "rev-parse", "--git-dir"],
                    capture_output=True, text=True, check=True).stdout.strip()
journal = (repo / gd if not Path(gd).is_absolute() else Path(gd)) / "flow-journal.jsonl"
records = [json.loads(line) for line in journal.read_text(encoding="utf-8").splitlines()]
print(json.dumps({"records": len(records), "last": records[-1] if records else None},
                 sort_keys=True))
