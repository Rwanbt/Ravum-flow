"""Robust reader: a partial or inconsistent journal is refused, never repaired silently."""
import json
import subprocess
import sys
from pathlib import Path


def git(repo, *args):
    return subprocess.run(["git", "-C", str(repo), *args],
                          capture_output=True, text=True, check=True).stdout.strip()


def refuse(reason, detail=None):
    print(json.dumps({"status": "REFUSED_CORRUPT_JOURNAL", "reason": reason,
                      "detail": detail}), flush=True)
    sys.exit(2)


repo = Path(sys.argv[1])
gd = git(repo, "rev-parse", "--git-dir")
journal = (repo / gd if not Path(gd).is_absolute() else Path(gd)) / "flow-journal.jsonl"
if not journal.exists():
    refuse("journal_missing")
raw = journal.read_bytes()
if not raw:
    refuse("journal_empty")
if not raw.endswith(b"\n"):
    refuse("last_line_not_terminated", {"bytes": len(raw)})
lines = raw.split(b"\n")[:-1]
records = []
for i, line in enumerate(lines, 1):
    if not line.strip():
        refuse("empty_line", {"line": i})
    try:
        rec = json.loads(line.decode("utf-8"))
    except Exception as exc:
        refuse("invalid_json", {"line": i, "error": str(exc)})
    if not isinstance(rec, dict) or rec.get("event") not in {"intent", "outcome"}:
        refuse("schema", {"line": i, "record": rec})
    for key in ("event",):
        if key not in rec:
            refuse("missing_field", {"line": i, "key": key})
    records.append(rec)

events = [r["event"] for r in records]
if events != ["intent", "outcome"] and events != ["intent"]:
    refuse("unexpected_sequence", {"events": events})
ref = git(repo, "rev-parse", "main")
intent = records[0]
if ref != intent.get("new"):
    refuse("ref_does_not_match_intent", {"ref": ref, "intent_new": intent.get("new")})
if events == ["intent"]:
    print(json.dumps({"status": "RECONCILED_PUBLISHED_FROM_REF", "records": len(records),
                      "ref": ref}), flush=True)
    sys.exit(0)
outcome = records[1]
if outcome.get("new") != intent.get("new") or outcome.get("status") != "PUBLISHED":
    refuse("outcome_inconsistent", {"outcome": outcome})
print(json.dumps({"status": "COMPLETE", "records": len(records), "ref": ref}), flush=True)
