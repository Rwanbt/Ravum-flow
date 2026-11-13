"""RESOURCE-14 child: spawns a long-lived grandchild, reports both PIDs, sleeps.

Used to show that killing the direct child leaves the grandchild alive unless a
tree kill (taskkill /T) is used.
"""
import json
import os
import subprocess
import sys
import time
from pathlib import Path

pidfile = Path(sys.argv[1])
pidfile.parent.mkdir(parents=True, exist_ok=True)
proc = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(60)"])
pidfile.write_text(json.dumps({"parent": os.getpid(), "grandchild": proc.pid}), encoding="utf-8")
time.sleep(45)
