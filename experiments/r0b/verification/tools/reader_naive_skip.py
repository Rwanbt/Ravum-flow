"""Naive reader: swallows malformed lines and keeps going (silent recovery)."""
import json
import subprocess
import sys
from pathlib import Path

repo = Path(sys.argv[1])
gd = subprocess.run(["git", "-C", str(repo), "rev-parse", "--git-dir"],
                    capture_output=True, text=True, check=True).stdout.strip()
journal = (repo / gd if not Path(gd).is_absolute() else Path(gd)) / "flow-journal.jsonl"
records, bad = [], 0
for line in journal.read_text(encoding="utf-8").splitlines():
    if not line.strip():
        continue
    try:
        records.append(json.loads(line))
    except Exception:
        bad += 1
last = records[-1] if records else None
status = (last or {}).get("event", "NONE")
print(json.dumps({"records": len(records), "bad_lines": bad, "last_event": status,
                  "would_republish": status != "outcome", "last": last}, sort_keys=True))
