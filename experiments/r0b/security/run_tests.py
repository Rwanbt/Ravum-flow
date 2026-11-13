#!/usr/bin/env python
"""R0-b security/process experiments — Windows, unprivileged user, no Internet.

Scope contract (see REPORT.md):
  * every write stays under experiments/r0b/security/ ;
  * the only network attempt is connect 127.0.0.1:9 (loopback, no service);
  * the canary is fictitious and is redacted BEFORE any log is persisted;
  * no real credential, no Ravum access, no heavy compilation (git + python).

Usage:  python experiments/r0b/security/run_tests.py
Writes: RESULTS.json, logs/*.log (redacted), scratch/** (disposable, git-ignored)
"""
from __future__ import annotations

import ctypes
import hashlib
import json
import os
import shutil
import stat
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from redact import CANARY, leaks, redact, write_redacted  # noqa: E402

ROOT = HERE.parents[2]
SCRATCH = HERE / "scratch"
LOGS = HERE / "logs"
SRC = HERE / "fixtures"
PRIVATE_GITCONFIG = SCRATCH / "gitconfig-private"
FAKE_HOME = SCRATCH / "fake-home"
FAKE_KEY = FAKE_HOME / ".ssh" / "id_fixture"
PY = sys.executable
STILL_ACTIVE = 259
PROCESS_QUERY_LIMITED_INFORMATION = 0x1000

ALLOWLIST_NAMES = (
    "PATH", "SYSTEMROOT", "SYSTEMDRIVE", "WINDIR", "COMSPEC", "PATHEXT",
    "TEMP", "TMP", "NUMBER_OF_PROCESSORS", "PROCESSOR_ARCHITECTURE",
    "PYTHONIOENCODING", "PYTHONDONTWRITEBYTECODE",
)

RECORDS: list[dict] = []
RESULTS: dict = {}


# --------------------------------------------------------------------- utils
def child_env(extra: dict | None = None) -> dict:
    """Child environment with a private HOME/git config (never the user's)."""
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    env.update({
        "GIT_CONFIG_NOSYSTEM": "1",
        "GIT_CONFIG_GLOBAL": str(PRIVATE_GITCONFIG),
        "GIT_CONFIG_SYSTEM": str(PRIVATE_GITCONFIG),
        "GIT_TERMINAL_PROMPT": "0",
        "HOME": str(SCRATCH / "home"),
        "USERPROFILE": str(SCRATCH / "home"),
        "LC_ALL": "C",
        "PYTHONIOENCODING": "utf-8",
        "PYTHONDONTWRITEBYTECODE": "1",
    })
    if extra:
        env.update(extra)
    return env


def allowlisted_env(extra: dict | None = None) -> dict:
    """Minimal allowlist: no HOME/USERPROFILE, no token-shaped variable."""
    env = {k: os.environ[k] for k in ALLOWLIST_NAMES if k in os.environ}
    if extra:
        env.update(extra)
    return env


def run(cmd: list[str], cwd: Path | None = None, env: dict | None = None,
        timeout: float = 60.0) -> subprocess.CompletedProcess:
    try:
        cp = subprocess.run([str(c) for c in cmd], cwd=str(cwd) if cwd else None,
                            env=env if env is not None else child_env(),
                            capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired as exc:
        return subprocess.CompletedProcess(cmd, 124, exc.stdout or "", exc.stderr or "[TIMEOUT]")
    return cp


def sanitize(text: str) -> str:
    """Replace machine-specific paths before anything is persisted."""
    pairs = [
        (str(ROOT), "<REPO>"),
        (str(Path(PY).parent), "<PYTHON_DIR>"),
        (os.environ.get("USERPROFILE", ""), "<USERPROFILE>"),
        (os.environ.get("TEMP", ""), "<TEMP>"),
    ]
    for needle, label in pairs:
        if needle:
            text = text.replace(needle, label)
    return text


def sanitize_obj(obj):
    """Recursively sanitize strings before JSON encoding (paths are escaped)."""
    if isinstance(obj, str):
        return sanitize(obj)
    if isinstance(obj, dict):
        return {k: sanitize_obj(v) for k, v in obj.items()}
    if isinstance(obj, list):
        return [sanitize_obj(v) for v in obj]
    return obj


def cmd_text(cmd: list[str]) -> str:
    return sanitize(redact(" ".join(str(c) for c in cmd))[0])


def record(rid: str, verdict: str, fixture: str, commands: list[list[str]],
           observation: str, scope: str, evidence: dict) -> None:
    RECORDS.append({
        "id": rid,
        "verdict": verdict,
        "fixture": fixture,
        "os": f"{sys.platform} / {os.name} (Windows, unprivileged user)",
        "commands": [cmd_text(c) for c in commands],
        "observation": observation,
        "scope": scope,
        "evidence": evidence,
    })
    print(f"[{rid}] {verdict}")


def pid_alive(pid: int) -> bool:
    kernel32 = ctypes.windll.kernel32
    handle = kernel32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, False, int(pid))
    if not handle:
        return False
    try:
        code = ctypes.c_ulong()
        ok = kernel32.GetExitCodeProcess(handle, ctypes.byref(code))
        return bool(ok) and code.value == STILL_ACTIVE
    finally:
        kernel32.CloseHandle(handle)


def taskkill(pid: int, tree: bool = True) -> int:
    args = ["taskkill", "/PID", str(pid), "/F"]
    if tree:
        args.insert(3, "/T")
    return subprocess.run(args, capture_output=True, text=True).returncode


def wait_for(predicate, timeout: float = 15.0, interval: float = 0.1):
    deadline = time.time() + timeout
    while time.time() < deadline:
        if predicate():
            return True
        time.sleep(interval)
    return False


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def naive_inside(ws: Path, candidate: Path) -> bool:
    """A plausible-but-wrong containment check: string prefix, no resolution."""
    return str(candidate).lower().startswith(str(ws).lower())


def canonical_inside(ws: Path, candidate: Path) -> bool:
    try:
        real = Path(os.path.realpath(candidate))
    except OSError:
        return False
    return real == ws or ws in real.parents


def _clear_readonly(func, path, exc_info) -> None:
    """Git objects are read-only on Windows: clear the bit then retry once."""
    try:
        os.chmod(path, stat.S_IWRITE | stat.S_IREAD)
        func(path)
    except OSError:
        pass


def _rmtree(path: Path) -> None:
    shutil.rmtree(path, onexc=_clear_readonly)


# ------------------------------------------------------------------- fixtures
def setup() -> None:
    if SCRATCH.exists():
        _rmtree(SCRATCH)
    (SCRATCH / "home").mkdir(parents=True)
    (SCRATCH / "logs").mkdir(parents=True)
    (SCRATCH / "fixtures").mkdir(parents=True)
    (SCRATCH / ".gitignore").write_text("*\n!.gitignore\n", encoding="utf-8")
    for name in ("probe.py", "env_child.py", "spawn_grandchild.py", "writer_lock.py"):
        shutil.copy2(SRC / name, SCRATCH / "fixtures" / name)
    PRIVATE_GITCONFIG.write_text("", encoding="utf-8")
    FAKE_HOME.mkdir(parents=True, exist_ok=True)
    (FAKE_HOME / ".ssh").mkdir(parents=True, exist_ok=True)
    FAKE_KEY.write_text(f"-----BEGIN FIXTURE KEY-----\n{CANARY}\n-----END FIXTURE KEY-----\n",
                        encoding="utf-8")
    (FAKE_HOME / ".ssh" / "known_hosts").write_text("fixture.invalid ssh-ed25519 AAAAfake\n",
                                                    encoding="utf-8")
    LOGS.mkdir(parents=True, exist_ok=True)


# --------------------------------------------------------------------- ENV-04a
def test_env_04a() -> None:
    child = SCRATCH / "fixtures" / "env_child.py"
    expected_prefix = hashlib.sha256(CANARY.encode()).hexdigest()[:12]
    logs: list[str] = []

    naive_env = dict(os.environ)
    naive_env["FLOW_TEST_SECRET"] = CANARY
    cp_naive = run([PY, str(child)], env=naive_env)
    naive = json.loads(cp_naive.stdout)
    logs.append(f"naive rc={cp_naive.returncode} {cp_naive.stdout.strip()}")

    cp_allow = run([PY, str(child)], env=allowlisted_env())
    allow = json.loads(cp_allow.stdout)
    logs.append(f"allowlist rc={cp_allow.returncode} {cp_allow.stdout.strip()}")

    inherited = bool(naive["canary_var_present"]
                     and naive["canary_value_sha256_12"] == expected_prefix)
    eliminated = not allow["canary_var_present"]

    write_redacted(LOGS / "ENV-04a.log", sanitize("\n".join(logs)) + "\n", label="ENV-04a")
    record(
        "ENV-04a", "REPRODUCED" if (inherited and eliminated) else "NOT_REPRODUCED",
        "env_child.py (fixture), no repository",
        [[PY, str(child)], [PY, str(child)]],
        "Un enfant lancé avec l'environnement brut du parent voit la variable canari "
        "(valeur confirmée par empreinte SHA-256, jamais imprimée) ; un enfant lancé "
        "avec une allowlist explicite ne la voit pas.",
        "Mécanisme OS (héritage d'environnement à la création de processus) mesuré par "
        "deux lancements du même script. Aucun code Flow n'existe : ce n'est pas une "
        "mesure du produit, seulement de la primitive d'exécution que Flow devra contraindre.",
        {
            "naive": {"canary_var_present": naive["canary_var_present"],
                      "sha256_prefix_matches": naive["canary_value_sha256_12"] == expected_prefix,
                      "env_var_count": naive["env_var_count"],
                      "sensitive_names_present": naive["names_present"]},
            "allowlist": {"canary_var_present": allow["canary_var_present"],
                          "env_var_count": allow["env_var_count"],
                          "names_present": allow["names_present"]},
            "inheritance_reproduced": inherited,
            "allowlist_mitigation_measured": eliminated,
        },
    )


# --------------------------------------------------------------------- ENV-04b
def test_env_04b() -> None:
    probe = SCRATCH / "fixtures" / "probe.py"
    expected = sha256_bytes(FAKE_KEY.read_bytes())

    cp = run([PY, str(probe)], env=child_env({"FLOW_FAKE_HOME": str(FAKE_HOME)}))
    positive = json.loads(cp.stdout)

    cp_neg = run([PY, str(probe)],
                 env=child_env({"FLOW_FAKE_HOME": str(SCRATCH / "missing-home")}))
    negative = json.loads(cp_neg.stdout)

    readable = bool(positive["read_secret"] and positive["secret_sha256"] == expected)
    logs = [
        f"positive rc={cp.returncode} {cp.stdout.strip()}",
        f"negative-control rc={cp_neg.returncode} {cp_neg.stdout.strip()}",
        "expected_sha256=" + expected,
        "isolation=none: same user, same filesystem, no sandbox",
    ]
    write_redacted(LOGS / "ENV-04b.log", sanitize("\n".join(logs)) + "\n", label="ENV-04b")
    record(
        "ENV-04b", "REPRODUCED" if readable and not negative["read_secret"] else "NOT_REPRODUCED",
        "probe.py + fake-home/.ssh/id_fixture (copie locale de f6-canary)",
        [[PY, str(probe)]],
        "Sous les droits de l'utilisateur, le processus enfant lit intégralement la clé "
        "fictive (empreinte SHA-256 identique à la source) sans aucune barrière ; le "
        "contrôle négatif (HOME inexistant) échoue comme attendu, donc le résultat n'est "
        "pas un faux positif du probe.",
        "Droits utilisateur Windows normaux, aucun sandbox ni ACL particulière. Ce n'est "
        "PAS une sandbox : l'absence d'isolation FS est une limite mesurée, pas un bug. "
        "Non qualifié : autre compte utilisateur, intégrité AppContainer, ACL restreinte.",
        {
            "read_secret": positive["read_secret"],
            "sha256_matches_source": positive["secret_sha256"] == expected,
            "secret_sha256": positive["secret_sha256"],
            "read_error": positive.get("read_error"),
            "negative_control_read_secret": negative["read_secret"],
            "negative_control_error": negative.get("read_error"),
            "isolation_observed": "none",
        },
    )


# --------------------------------------------------------------------- ENV-04c
def test_env_04c() -> None:
    cmd = [PY, "-c",
           "import sys; print('OUT:'+%r); print('ERR:'+%r, file=sys.stderr)" % (CANARY, CANARY)]
    cp = run(cmd)
    raw = f"STDOUT:\n{cp.stdout}\nSTDERR:\n{cp.stderr}\n"
    naive_count = raw.count(CANARY)

    transient = SCRATCH / "naive-env04c.log"
    transient.write_text(raw, encoding="utf-8")
    persisted_naive = transient.read_text(encoding="utf-8").count(CANARY)
    transient.unlink()
    transient_gone = not transient.exists()

    redacted_info = write_redacted(LOGS / "ENV-04c-redacted.log",
                                   sanitize(raw + f"\nCOMMAND: {cmd_text(cmd)}\n"),
                                   label="ENV-04c")
    stored = (LOGS / "ENV-04c-redacted.log").read_text(encoding="utf-8")
    stored_count = stored.count(CANARY)

    ok = naive_count >= 2 and persisted_naive == naive_count and transient_gone and stored_count == 0
    record(
        "ENV-04c", "REPRODUCED" if ok else "NOT_REPRODUCED",
        "python child printing the fictitious canary on stdout+stderr",
        [cmd],
        "Un log naïf (copie brute de stdout+stderr) persiste le canari (2 occurrences "
        "mesurées dans un fichier transitoire immédiatement supprimé) ; la rédaction "
        "avant persistance ramène le fichier conservé à 0 occurrence, avec contrôle "
        "fail-closed sur les marqueurs résiduels.",
        "Démontre le mécanisme de rédaction, pas la sûreté des logs d'un futur produit. "
        "La rédaction est une liste de marqueurs : un secret de forme inconnue passerait. "
        "Le fichier transitoire n'a jamais été conservé (supprimé et absence vérifiée).",
        {
            "naive_occurrences_in_memory": naive_count,
            "naive_occurrences_persisted_transiently": persisted_naive,
            "transient_file_deleted": transient_gone,
            "stored_log": redacted_info["path"],
            "stored_log_occurrences": stored_count,
            "redactions_applied": redacted_info["redactions"],
            "redaction_is_denylist": True,
        },
    )


# ------------------------------------------------------------------ SANDBOX-05
def test_sandbox_05() -> None:
    probe = SCRATCH / "fixtures" / "probe.py"
    cp = run([PY, str(probe)], env=child_env({"FLOW_FAKE_HOME": str(FAKE_HOME)}))
    probe_result = json.loads(cp.stdout)

    def loopback_attempt(timeout: float | None) -> dict:
        guard = f"s.settimeout({timeout!r})\n" if timeout else ""
        code = ("import json,socket,time\n"
                "s=socket.socket()\n" + guard +
                "t=time.time()\n"
                "rc=s.connect_ex(('127.0.0.1',9))\n"
                "s.close()\n"
                "print(json.dumps({'rc':rc,'ms':round((time.time()-t)*1000)}))\n")
        result = run([PY, "-c", code])
        row = json.loads(result.stdout)
        row["mode"] = f"timeout_{timeout}" if timeout else "blocking"
        return row

    attempts = [loopback_attempt(None) for _ in range(3)]
    attempts += [loopback_attempt(0.5) for _ in range(3)]
    names = {0: "SUCCESS", 10061: "WSAECONNREFUSED", 10013: "WSAEACCES",
             10051: "WSAENETUNREACH", 10065: "WSAEHOSTUNREACH", 10060: "WSAETIMEDOUT",
             10035: "WSAEWOULDBLOCK"}

    denied = [a for a in attempts if a["rc"] in (10013, 10051, 10065)]
    codes = sorted({a["rc"] for a in attempts})
    logs = [
        f"probe rc={cp.returncode} {cp.stdout.strip()}",
        "errno attempts: " + json.dumps(attempts),
        "codes: " + ", ".join(f"{c} ({names.get(c, 'UNKNOWN')})" for c in codes),
        f"policy-denial codes seen: {[a['rc'] for a in denied]}",
        "verdict: loopback attempt permitted; outcome differs by socket mode "
        "(refused after ~2 s when blocking, WSAEWOULDBLOCK at timeout otherwise)",
        "NOT_QUALIFIED: Internet egress (no Internet access allowed in R0-b)",
        "NOT_QUALIFIED: host firewall / AppContainer / container network",
    ]
    write_redacted(LOGS / "SANDBOX-05.log", sanitize("\n".join(logs)) + "\n", label="SANDBOX-05")
    record(
        "SANDBOX-05", "REPRODUCED" if not denied else "BLOCKED",
        "probe.py loopback 127.0.0.1:9 (no service) + connect_ex matrix (blocking / 0.5 s)",
        [[PY, str(probe)],
         [PY, "-c", "<connect_ex 127.0.0.1:9 x6>"]],
        "Six tentatives loopback mesurées : en mode bloquant, WSAECONNREFUSED (10061) après "
        "~2 s ; avec un délai, WSAEWOULDBLOCK (10035) à l'expiration. Aucun code de refus "
        "d'accès ou de réseau injoignable (WSAEACCES/WSAENETUNREACH) : aucune couche de "
        "politique n'a bloqué la tentative. Le délai avant refus suggère un filtrage local "
        "silencieux du SYN (pare-feu d'hôte), ce qui n'est pas une isolation.",
        "Loopback uniquement (127.0.0.1:9), aucun serveur local, aucun accès Internet. "
        "NOT_QUALIFIED : egress externe, DNS, proxy, pare-feu d'hôte (non instrumenté), "
        "capacité réseau AppContainer/Windows Firewall, réseau d'un conteneur. Une "
        "connexion loopback réussie ne serait pas non plus une preuve d'egress.",
        {
            "probe_network_result": probe_result["network"],
            "probe_network_error": probe_result.get("network_error"),
            "attempts": attempts,
            "distinct_codes": codes,
            "distinct_code_names": [names.get(c, "UNKNOWN") for c in codes],
            "policy_denial_observed": bool(denied),
            "not_qualified": ["external egress", "DNS", "proxy", "host firewall",
                              "AppContainer network capability", "container network"],
        },
    )


# --------------------------------------------------------------------- PATH-08
def test_path_08() -> None:
    base = SCRATCH / "path-08"
    ws = base / "ws"
    outside = base / "outside"
    ws.mkdir(parents=True)
    outside.mkdir(parents=True)
    sub: dict[str, dict] = {}
    logs: list[str] = []

    # (a) lexical traversal
    cand_a = Path(os.path.join(str(ws), "..", "evil.txt"))
    naive_a = naive_inside(ws, cand_a)
    cand_a.write_text("escaped by traversal\n", encoding="utf-8")
    escaped_a = (base / "evil.txt").exists()
    canon_a = canonical_inside(ws, cand_a)
    sub["a_traversal"] = {"candidate": str(cand_a), "naive_accepts": naive_a,
                          "canonical_accepts": canon_a, "write_outside_ws": escaped_a}
    logs.append(f"(a) candidate={cand_a} naive={naive_a} canonical={canon_a} escaped={escaped_a}")

    # (b) Windows junction inside the workspace pointing outside it
    junction = ws / "vendor"
    junction_rc = subprocess.run(["cmd", "/c", "mklink", "/J", str(junction), str(outside)],
                                 capture_output=True, text=True).returncode
    junction_created = junction.exists() or junction.is_symlink()
    if junction_created:
        target = junction / "pwn.txt"
        naive_b = naive_inside(ws, target)
        target.write_text("written through a junction\n", encoding="utf-8")
        landed = (outside / "pwn.txt").exists()
        canon_b = canonical_inside(ws, target)
        realpath_b = os.path.realpath(target)
        subprocess.run(["cmd", "/c", "rmdir", str(junction)], capture_output=True, text=True)
        junction_removed = not junction.exists()
        target_survives = (outside / "pwn.txt").exists()
        sub["b_junction"] = {"junction_rc": junction_rc, "naive_accepts": naive_b,
                             "canonical_accepts": canon_b, "realpath": realpath_b,
                             "write_outside_ws": landed, "junction_removed": junction_removed,
                             "outside_file_survives": target_survives}
        logs.append(f"(b) junction_rc={junction_rc} naive={naive_b} canonical={canon_b} "
                    f"realpath={realpath_b} landed={landed}")
    else:
        sub["b_junction"] = {"junction_rc": junction_rc, "naive_accepts": None,
                             "canonical_accepts": None, "write_outside_ws": False,
                             "blocked_reason": "mklink /J refused"}
        logs.append(f"(b) BLOCKED junction_rc={junction_rc}")

    # (c) Windows reserved names and trailing dot/space
    reserved = {}
    for name in ("CON", "aux.txt", "trail.", "space "):
        cand = ws / name
        entry = {"naive_accepts": naive_inside(ws, cand),
                 "canonical_accepts": canonical_inside(ws, cand),
                 "realpath_name": Path(os.path.realpath(cand)).name}
        before = set(os.listdir(ws))
        try:
            with open(cand, "x", encoding="utf-8") as fh:
                fh.write("x")
            entry["create"] = "ok"
        except OSError as exc:
            entry["create"] = type(exc).__name__
        after = set(os.listdir(ws))
        entry["created_names"] = sorted(after - before)
        entry["maps_to_disk_name"] = entry["created_names"][0] if entry["created_names"] else None
        entry["name_differs_from_request"] = bool(
            entry["created_names"] and entry["created_names"][0] != name)
        reserved[name] = entry
        logs.append(f"(c) {name!r} create={entry['create']} on_disk={entry['created_names']}")
    # trailing-dot collision: "trail." and "trail" are two logical paths, one file
    collision = (ws / "trail").exists() and (ws / "trail.").exists()
    sub["c_reserved"] = {"cases": reserved, "dot_collision": collision}
    logs.append(f"(c) trailing-dot collision with 'trail': {collision}")

    # (c2) FS/Git divergence: the filesystem accepts a name Git cannot index
    gitres = base / "gitres"
    gitres.mkdir()
    run(["git", "-C", str(gitres), "init", "-q", "-b", "main"], env=child_env())
    created_reserved = []
    for name in ("CON", "aux.txt"):
        try:
            with open(gitres / name, "x", encoding="utf-8") as fh:
                fh.write("x")
            created_reserved.append(name)
        except OSError:
            pass
    status = run(["git", "-C", str(gitres), "status", "--porcelain=v1", "-uall"],
                 env=child_env())
    git_add: dict[str, dict] = {}
    for name in created_reserved:
        cp_add = run(["git", "-C", str(gitres), "add", "--", name], env=child_env())
        head = (cp_add.stderr.strip().splitlines() or [""])[0][:120]
        git_add[name] = {"rc": cp_add.returncode, "stderr_head": sanitize(head)}
    sub["c2_git_divergence"] = {
        "created_on_disk": created_reserved,
        "git_status_lines": [sanitize(ln) for ln in status.stdout.strip().splitlines()],
        "git_add": git_add,
    }
    git_divergence = bool(created_reserved) and any(v["rc"] != 0 for v in git_add.values())
    logs.append(f"(c2) created={created_reserved} git_status={status.stdout.strip().splitlines()} "
                f"git_add_rc={ {k: v['rc'] for k, v in git_add.items()} }")

    # (d) case collision on a case-insensitive filesystem
    (ws / "Case.txt").write_text("UPPER\n", encoding="utf-8")
    (ws / "case.txt").write_text("lower\n", encoding="utf-8")
    same_file = os.path.samefile(ws / "Case.txt", ws / "case.txt")
    matching = [n for n in os.listdir(ws) if n.lower() == "case.txt"]
    content = (ws / "Case.txt").read_text(encoding="utf-8")
    sub["d_case_collision"] = {"naive_distinct_paths": len({"Case.txt", "case.txt"}),
                               "fs_entries_matching": len(matching),
                               "same_file": same_file,
                               "content_after_second_write": content.strip()}
    logs.append(f"(d) naive_distinct=2 fs_entries={matching} same_file={same_file} "
                f"content={content.strip()}")

    reproduced = bool(escaped_a and sub["b_junction"].get("write_outside_ws")
                      and same_file and len(matching) == 1 and git_divergence)
    write_redacted(LOGS / "PATH-08.log", sanitize("\n".join(logs)) + "\n", label="PATH-08")
    record(
        "PATH-08", "REPRODUCED" if reproduced else "NOT_REPRODUCED",
        "workspace synthétique scratch/path-08 (ws/, outside/, gitres/, jonction cmd mklink /J)",
        [["cmd", "/c", "mklink", "/J", str(junction), str(outside)],
         ["cmd", "/c", "rmdir", str(junction)],
         ["git", "-C", str(gitres), "add", "--", "CON"]],
        "Quatre variantes mesurées : (a) une validation naïve par préfixe de chaîne accepte "
        "une traversée '..' qui écrit hors du workspace simulé, alors que la canonisation "
        "realpath la rejette ; (b) une jonction Windows interne au workspace redirige une "
        "écriture hors périmètre (fichier retrouvé dans la cible) alors que la validation "
        "naïve l'accepte et que realpath la rejette ; (c) 'trail.' et 'space ' sont acceptés "
        "puis silencieusement renommés en 'trail'/'space' (collision de deux chemins "
        "logiques sur un fichier) ; (d) deux chemins ne différant que par la casse désignent "
        "un seul fichier, la seconde écriture écrasant la première. Cas additionnel (c2) : "
        "les noms 'CON' et 'aux.txt' sont créés sur disque et listés par git status, mais "
        "git add échoue ('unable to stat'/'unable to index') — divergence FS/Git mesurée.",
        "Workspace simulé (scratch/path-08/ws) : l'échappement reste à l'intérieur de "
        "experiments/r0b/security/. Aucune écriture hors du périmètre autorisé. Le cas (b) "
        "dépend de la possibilité de créer une jonction sans admin (mesurée ici). "
        "NOT_REVIEWED : chemins non-UTF-8, partages réseau, volumes montés, et politique "
        "d'un futur code Flow (inexistant). Le devenir des fichiers 'CON' vis-à-vis des "
        "outils Win32 (device namespace) n'est pas qualifié au-delà de git add.",
        {
            "a_traversal": sub["a_traversal"],
            "b_junction": sub["b_junction"],
            "c_reserved": sub["c_reserved"],
            "c2_git_divergence": sub["c2_git_divergence"],
            "d_case_collision": sub["d_case_collision"],
            "all_subcases_reproduced": reproduced,
            "fs_git_divergence_reproduced": git_divergence,
        },
    )


# ----------------------------------------------------------------- RESOURCE-14
def test_resource_14() -> None:
    script = SCRATCH / "fixtures" / "spawn_grandchild.py"
    work = SCRATCH / "resource-14"
    work.mkdir(parents=True, exist_ok=True)
    logs: list[str] = []
    survivors: list[int] = []

    def launch(tag: str) -> tuple[subprocess.Popen, dict]:
        pidfile = work / f"pids-{tag}.json"
        proc = subprocess.Popen([PY, str(script), str(pidfile)], cwd=str(work),
                                env=child_env(), stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                text=True)
        waited = wait_for(pidfile.exists, timeout=20)
        data = json.loads(pidfile.read_text()) if waited else {}
        return proc, data

    # phase 1 — kill the direct child only
    proc1, pids1 = launch("direct")
    direct = bool(pids1)
    if direct:
        proc1.kill()
        proc1.wait(timeout=10)
        time.sleep(0.5)
        alive_after_direct = pid_alive(pids1["grandchild"])
        logs.append(f"phase1 parent={pids1['parent']} grandchild={pids1['grandchild']} "
                    f"parent_rc={proc1.returncode} grandchild_alive={alive_after_direct}")
        if alive_after_direct:
            survivors.append(pids1["grandchild"])
    else:
        alive_after_direct = None
        logs.append("phase1 BLOCKED: pidfile never appeared")

    # phase 2 — tree kill on a fresh pair
    proc2, pids2 = launch("tree")
    tree_killed = None
    kill_rc = None
    if pids2:
        kill_rc = taskkill(pids2["parent"], tree=True)
        proc2.wait(timeout=10)
        time.sleep(0.5)
        tree_killed = not pid_alive(pids2["grandchild"])
        logs.append(f"phase2 parent={pids2['parent']} grandchild={pids2['grandchild']} "
                    f"taskkill_rc={kill_rc} grandchild_alive_after_tree_kill={not tree_killed}")
        if not tree_killed:
            survivors.append(pids2["grandchild"])
    else:
        logs.append("phase2 BLOCKED: pidfile never appeared")

    for pid in survivors:
        if pid_alive(pid):
            taskkill(pid, tree=False)
    cleanup_ok = not any(pid_alive(p) for p in survivors)

    reproduced = bool(direct and alive_after_direct and tree_killed)
    write_redacted(LOGS / "RESOURCE-14.log", sanitize("\n".join(logs)) + "\n", label="RESOURCE-14")
    record(
        "RESOURCE-14", "REPRODUCED" if reproduced else "BLOCKED",
        "spawn_grandchild.py : enfant direct + petit-enfant python time.sleep(60)",
        [[PY, str(script), str(work / "pids-direct.json")],
         ["taskkill", "/PID", "<parent>", "/T", "/F"]],
        "Tueur l'enfant direct par TerminateProcess : le petit-enfant reste vivant. "
        "Un kill d'arbre (taskkill /T /F) sur un second couple tue bien les deux. "
        "Le timeout d'un runner qui ne tue que l'enfant direct laisse donc des processus "
        "orphelins.",
        "Windows : taskkill /T est une mitigation mesurée, pas une garantie (course possible "
        "sur la création de descendants, pas de Job Object). Aucun Job Object / cgroup, "
        "aucune limite mémoire/CPU mesurée. Linux (groupe de processus, pidfd, cgroup) "
        "NOT_REVIEWED dans cette session.",
        {
            "phase1_direct_child_kill": {"grandchild_alive": alive_after_direct,
                                          "parent_returncode": proc1.returncode},
            "phase2_tree_kill": {"taskkill_rc": kill_rc, "grandchild_dead": tree_killed},
            "descendant_survives_direct_kill": alive_after_direct,
            "tree_kill_mitigation_measured": tree_killed,
            "cleanup_all_survivors_dead": cleanup_ok,
        },
    )


# ---------------------------------------------------------------- UNMANAGED-17
def test_unmanaged_17() -> None:
    base = SCRATCH / "unmanaged-17"
    repo = base / "repo"
    repo.mkdir(parents=True)
    git = ["git", "-C", str(repo)]
    setup_cmds: list[list[str]] = []
    for args in (["init", "-q", "-b", "main"],
                 ["config", "user.name", "Flow R0-b Fixture"],
                 ["config", "user.email", "fixture@example.invalid"],
                 ["config", "commit.gpgsign", "false"]):
        run(git + args, env=child_env())
        setup_cmds.append(git + args)
    (repo / "seed.txt").write_text("seed\n", encoding="utf-8")
    run(git + ["add", "seed.txt"], env=child_env())
    run(git + ["commit", "-q", "-m", "fixture: seed"], env=child_env())
    head0 = run(git + ["rev-parse", "HEAD"], env=child_env()).stdout.strip()

    lockfile = base / "writer.lock"
    statusfile = base / "writer-status.json"
    writer_script = SCRATCH / "fixtures" / "writer_lock.py"
    writer = subprocess.Popen([PY, str(writer_script), str(repo), str(lockfile), str(statusfile), "5"],
                              cwd=str(base), env=child_env(),
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    def writer_locked() -> bool:
        try:
            return (statusfile.exists()
                    and json.loads(statusfile.read_text(encoding="utf-8")).get("state") == "locked")
        except (OSError, json.JSONDecodeError):
            return False

    locked = wait_for(writer_locked, timeout=20)
    before = json.loads(statusfile.read_text()) if locked else {}

    external_rc = None
    external_err = None
    if locked:
        cp = run(git + ["commit", "--allow-empty", "-m", "external edit while lock held"],
                 env=child_env())
        external_rc, external_err = cp.returncode, cp.stderr.strip()[:200]
    writer.wait(timeout=20)
    final = json.loads(statusfile.read_text()) if statusfile.exists() else {}
    head1 = run(git + ["rev-parse", "HEAD"], env=child_env()).stdout.strip()

    reproduced = bool(locked and external_rc == 0 and final.get("change_detected_by_epoch_reread"))
    logs = [
        f"head_before={head0}",
        f"writer_locked={locked} state_before={before.get('state')}",
        f"external_commit_rc={external_rc}",
        f"head_after={head1}",
        f"writer_release_state={final.get('state')} detected={final.get('change_detected_by_epoch_reread')}",
        f"heads_differ={head0 != head1}",
    ]
    write_redacted(LOGS / "UNMANAGED-17.log", sanitize("\n".join(logs)) + "\n", label="UNMANAGED-17")
    record(
        "UNMANAGED-17", "REPRODUCED" if reproduced else "NOT_REPRODUCED",
        "dépôt git synthétique scratch/unmanaged-17/repo + verrou fichier OS (msvcrt)",
        setup_cmds + [["git", "-C", str(repo), "commit", "--allow-empty", "-m", "<externe>"]],
        "Pendant qu'un « writer » détient un verrou fichier exclusif, un git commit externe "
        "réussit (rc=0) : le verrou n'est pas consulté par Git, donc il ne prévient pas "
        "l'écriture concurrente. Le writer détecte ensuite le changement de ref par relecture "
        "(head_before != head_after) : détection possible, prévention non.",
        "Modèle simplifié à un verrou fichier + relecture, pas le protocole Flow. Le git "
        "externe utilisé ici est un commit local ; push distant, index concurrent et "
        "répertoires de travail non sujets ne sont pas qualifiés.",
        {
            "writer_held_lock": locked,
            "external_commit_returncode": external_rc,
            "external_commit_stderr": external_err,
            "head_before": head0,
            "head_after": head1,
            "heads_differ": head0 != head1,
            "detection_by_reread": bool(final.get("change_detected_by_epoch_reread")),
            "prevention_by_lock": False,
        },
    )


# ----------------------------------------------------------------------- main
def canary_scan() -> dict:
    """Assert the canary never reached a persisted artifact (fixture excluded)."""
    hits, scanned = [], 0
    for path in sorted(HERE.rglob("*")):
        if not path.is_file() or path.name == "__pycache__" or path.suffix == ".pyc":
            continue
        if SCRATCH in path.parents:  # the fake key must contain the canary
            continue
        scanned += 1
        try:
            if CANARY in path.read_text(encoding="utf-8", errors="replace"):
                hits.append(str(path.relative_to(HERE)))
        except OSError as exc:
            hits.append(f"{path.relative_to(HERE)}: unreadable {type(exc).__name__}")
    fixture_hits = 0
    if FAKE_KEY.exists():
        fixture_hits = FAKE_KEY.read_text(encoding="utf-8").count(CANARY)
    return {
        "files_scanned": scanned,
        "excluded": ["scratch/ (fixture canary: fake-home/.ssh/id_fixture)"],
        "canary_occurrences_in_persisted_artifacts": len(hits),
        "hits": hits,
        "fixture_canary_occurrences": fixture_hits,
        "passed": not hits,
    }


def main() -> int:
    started = datetime.now(timezone.utc)
    t0 = time.time()
    setup()
    tests = [(test_env_04a, "ENV-04a"), (test_env_04b, "ENV-04b"), (test_env_04c, "ENV-04c"),
             (test_sandbox_05, "SANDBOX-05"), (test_path_08, "PATH-08"),
             (test_resource_14, "RESOURCE-14"), (test_unmanaged_17, "UNMANAGED-17")]
    for test, rid in tests:
        try:
            test()
        except Exception as exc:  # a failing test is data, never a crash
            record(rid, "BLOCKED", "unknown", [], f"exception: {type(exc).__name__}",
                   "Test interrompu par une exception ; aucune conclusion.", {"error": str(exc)[:400]})

    scan = canary_scan()
    counts: dict[str, int] = {}
    for item in RECORDS:
        counts[item["verdict"]] = counts.get(item["verdict"], 0) + 1

    results = {
        "schema": "r0b-security-results/1",
        "generated_at_utc": started.isoformat(timespec="seconds"),
        "duration_s": round(time.time() - t0, 1),
        "host": {
            "os": sys.platform,
            "os_name": os.name,
            "python": sys.version.split()[0],
            "git": run(["git", "--version"]).stdout.strip(),
            "privilege": "standard user, no elevation, no admin",
            "internet": "not used; only loopback 127.0.0.1:9",
        },
        "records": RECORDS,
        "summary": {
            "by_verdict": counts,
            "canary_scan": scan,
            "blocked_or_not_run": [r["id"] for r in RECORDS
                                   if r["verdict"] in ("BLOCKED", "NOT_RUN")],
        },
    }
    # fail closed: never persist a result that contains the marker
    payload = json.dumps(sanitize_obj(results), indent=2, ensure_ascii=False) + "\n"
    if leaks(payload):
        raise SystemExit("refusing to persist RESULTS.json: marker survived")
    (HERE / "RESULTS.json").write_text(payload, encoding="utf-8")

    print("\n== summary ==")
    for verdict, n in sorted(counts.items()):
        print(f"  {verdict}: {n}")
    print(f"  canary scan passed: {scan['passed']} "
          f"({scan['files_scanned']} files, fixture occurrences={scan['fixture_canary_occurrences']})")
    return 0 if not scan["hits"] and not results["summary"]["blocked_or_not_run"] else 1


if __name__ == "__main__":
    sys.exit(main())
