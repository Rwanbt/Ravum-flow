"""UNMANAGED-17 writer: holds an exclusive OS file lock, then checks the ref.

Models a Flow writer that serialises its own publications with a lock file. The
point of the test is that an *external* `git` process does not consult this lock,
so the lock alone is not prevention; only a re-read before the CAS detects it.
"""
import json
import msvcrt
import os
import subprocess
import sys
import time
from pathlib import Path


def head(repo: Path) -> str:
    return subprocess.run(["git", "-C", str(repo), "rev-parse", "HEAD"],
                          capture_output=True, text=True).stdout.strip()


def main() -> None:
    repo, lockfile, statusfile = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
    hold = float(sys.argv[4]) if len(sys.argv) > 4 else 4.0
    lockfile.parent.mkdir(parents=True, exist_ok=True)
    statusfile.parent.mkdir(parents=True, exist_ok=True)
    with open(lockfile, "w+") as fh:
        fh.write("lock\n")
        fh.flush()
        fh.seek(0)
        msvcrt.locking(fh.fileno(), msvcrt.LK_NBLCK, 1)
        try:
            before = head(repo)
            statusfile.write_text(json.dumps({"state": "locked", "pid": os.getpid(),
                                              "head_before": before}), encoding="utf-8")
            time.sleep(hold)
            after = head(repo)
            statusfile.write_text(json.dumps({
                "state": "released", "pid": os.getpid(),
                "head_before": before, "head_after": after,
                "change_detected_by_epoch_reread": before != after,
            }), encoding="utf-8")
        finally:
            fh.seek(0)
            msvcrt.locking(fh.fileno(), msvcrt.LK_UNLCK, 1)


if __name__ == "__main__":
    main()
