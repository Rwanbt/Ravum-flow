"""ENV-04a child: reports what it inherited, never printing a value.

Prints booleans plus the *names* of sensitive variables and a truncated SHA-256
of the injected value, so the canary can never appear in a log.
"""
import hashlib
import json
import os

NAME = "FLOW_TEST_SECRET"
SENSITIVE = [
    "HOME", "USERPROFILE", "APPDATA", "LOCALAPPDATA", "TEMP",
    "SSH_AUTH_SOCK", "GIT_ASKPASS", "GIT_CONFIG_GLOBAL", "GIT_SSH_COMMAND",
    "AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY", "AWS_SESSION_TOKEN",
    "GH_TOKEN", "GITHUB_TOKEN", "NPM_TOKEN", "HF_TOKEN", "OPENAI_API_KEY",
]

value = os.environ.get(NAME)
report = {
    "canary_var_present": value is not None,
    "canary_value_sha256_12": hashlib.sha256(value.encode()).hexdigest()[:12] if value else None,
    "env_var_count": len(os.environ),
    "names_present": sorted(n for n in SENSITIVE if n in os.environ),
    "argv_env_is_inherited": True,
}
print(json.dumps(report, sort_keys=True))
