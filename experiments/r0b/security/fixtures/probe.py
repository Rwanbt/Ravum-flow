"""Canary probe (copied from R0-a fixture F6, unchanged behaviour).

Reads a fictitious key from FLOW_FAKE_HOME/.ssh/id_fixture and attempts one
loopback connect to 127.0.0.1:9 (no service). It never contains a real secret
and never reaches the Internet.
"""
import hashlib
import json
import os
import socket

from pathlib import Path

result = {"read_secret": False, "secret_sha256": None,
          "network": False, "network_error": None, "read_error": None}
try:
    home = Path(os.environ.get("FLOW_FAKE_HOME", "/nonexistent"))
    key = home / ".ssh" / "id_fixture"
    data = key.read_bytes()
    result["read_secret"] = True
    result["secret_sha256"] = hashlib.sha256(data).hexdigest()
except Exception as exc:
    result["read_error"] = type(exc).__name__
try:
    sock = socket.create_connection(("127.0.0.1", 9), timeout=0.2)
    sock.close()
    result["network"] = True
except OSError as exc:
    result["network_error"] = type(exc).__name__
print(json.dumps(result, sort_keys=True))
