#!/usr/bin/env bash
# Build the Linux release artefact for Ravum Flow from one committed SHA.
#
# It mirrors the shape of scripts/release/build-windows.ps1 and of the
# reproducibility step in .github/workflows/flow-ci.yml:
#   * two clean `git archive` checkouts of the SAME commit in two directories;
#   * built sequentially with `-C codegen-units=1 --remap-path-prefix=<dir>= `
#     (the MSVC-only `-C link-arg=/Brepro` has no GNU equivalent; this script
#     therefore does NOT claim that Windows and Linux were built with the same
#     flag set, only that the Linux result is reproducible on this host);
#   * refuses to package when the two binaries differ, unless ALLOW_DIFF=1, in
#     which case it packages and records `identical: false` (never a false claim).
#
# It runs the FULL test suite on the archived commit (RUN_TESTS=1, default) and
# records the measured counts in PROVENANCE.json. A failing suite aborts before
# packaging unless ALLOW_TEST_FAILURE=1.
#
# Requirements: bash, git, tar, cargo, sha256sum, python3 (build-time only; the
# product has no dependencies).
#
# Usage:
#   bash scripts/release/build-linux.sh <full-sha> [version] [triple]
#   RUN_TESTS=0 TEST_LOG=test-output.txt bash scripts/release/build-linux.sh <sha>
#
# Exit: 0 when the artefact was produced; 1 on any failed precondition,
# non-reproducible build (unless ALLOW_DIFF=1) or failed test suite.
set -euo pipefail

fail() { echo "ERROR: $*" >&2; exit 1; }

# Flags (all optional): --skip-tests uses the log from the CI gated step instead
# of running the suite a second time; --test-log <path> supplies that log.
RUN_TESTS="${RUN_TESTS:-1}"
TEST_LOG="${TEST_LOG:-}"
ALLOW_DIFF="${ALLOW_DIFF:-0}"
ALLOW_TEST_FAILURE="${ALLOW_TEST_FAILURE:-0}"
POSITIONAL=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        --skip-tests) RUN_TESTS=0; shift ;;
        --test-log)
            [ "$#" -ge 2 ] || fail "--test-log needs a path"
            TEST_LOG="$2"; shift 2 ;;
        --test-log=*) TEST_LOG="${1#*=}"; shift ;;
        -h|--help)
            sed -n '2,40p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
            exit 0 ;;
        *) POSITIONAL+=("$1"); shift ;;
    esac
done

SHA="${POSITIONAL[0]:-}"
TRIPLE="${POSITIONAL[2]:-x86_64-unknown-linux-gnu}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# All defaults are derived, so the script runs unchanged from a CI checkout.
REPO="${REPO:-$(cd "$SCRIPT_DIR/../.." && pwd)}"
# The crate manifest is the single source of the version. A hardcoded default made
# every artefact and every PROVENANCE.json claim a stale version (found by the
# 2026-11-13 release audit: the crate was 0.2.0-rc.2 and the scripts said rc.1).
if [ -n "${POSITIONAL[1]:-}" ]; then
    VERSION="${POSITIONAL[1]}"
else
    VERSION="$(sed -n 's/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$REPO/crates/flow/Cargo.toml" | head -n1)"
    [ -n "$VERSION" ] || fail "cannot read the crate version from $REPO/crates/flow/Cargo.toml"
fi
SRC="${SRC:-${TMPDIR:-/tmp}/flow-linux-src}"
TARGET_ROOT="${TARGET_ROOT:-${TMPDIR:-/tmp}/flow-target-release}"
DIST="${DIST:-$REPO/dist}"

[ -n "$SHA" ] || fail "usage: build-linux.sh <full-sha> [version] [triple] [--skip-tests] [--test-log <path>]"

command -v git >/dev/null || fail "git not found"
command -v cargo >/dev/null || fail "cargo not found"
command -v sha256sum >/dev/null || fail "sha256sum not found"
command -v python3 >/dev/null || fail "python3 not found (build-time JSON writer)"

git -C "$REPO" cat-file -e "$SHA^{commit}" 2>/dev/null || fail "commit $SHA not found in $REPO"
SHA="$(git -C "$REPO" rev-parse "$SHA^{commit}")"
[ "${#SHA}" -eq 40 ] || fail "expected a full 40-hex SHA, got $SHA"
TREE_OID="$(git -C "$REPO" rev-parse "$SHA^{tree}")"
LICENSE_BLOB_SIZE="$(git -C "$REPO" cat-file -s "$SHA:LICENSE")"
LICENSE_BLOB_OID="$(git -C "$REPO" rev-parse "$SHA:LICENSE")"
CORE_AUTOCRLF="$(git -C "$REPO" config --get core.autocrlf || true)"
BRANCH="$(git -C "$REPO" rev-parse --abbrev-ref HEAD)"
NOW="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
SHORT="${SHA:0:7}"

echo "== commit $SHA (tree $TREE_OID), version $VERSION, target $TRIPLE =="

# ---------------------------------------------------------------- clean archives
rm -rf "$SRC"
mkdir -p "$SRC/a" "$SRC/b"
for n in a b; do
    git -C "$REPO" archive "$SHA" | tar -x -C "$SRC/$n" \
        || fail "git archive | tar -x failed for archive $n"
done

# Source identity: the tar stream the artefacts were built from.
git -C "$REPO" archive -o "$SRC/source-$SHA.tar" "$SHA"
ARCHIVE_SHA="$(sha256sum "$SRC/source-$SHA.tar" | cut -d' ' -f1)"
ARCHIVE_BYTES="$(stat -c %s "$SRC/source-$SHA.tar")"
git -C "$REPO" ls-tree -r --full-tree "$SHA" > "$SRC/tree-manifest.txt"
MANIFEST_SHA="$(sha256sum "$SRC/tree-manifest.txt" | cut -d' ' -f1)"
MANIFEST_ENTRIES="$(wc -l < "$SRC/tree-manifest.txt" | tr -d ' ')"

# ---------------------------------------------------------------- tests
TEST_COUNTS_LOG=""
if [ "$RUN_TESTS" = "1" ]; then
    echo "== cargo test --locked --all-targets --offline =="
    TEST_COUNTS_LOG="$SRC/test.log"
    set +e
    ( cd "$SRC/a" && CARGO_TARGET_DIR="$TARGET_ROOT/test" cargo test --locked --all-targets --offline ) \
        > "$TEST_COUNTS_LOG" 2>&1
    TEST_EXIT=$?
    set -e
    grep -E '^(running|test result)' "$TEST_COUNTS_LOG" || true
    if [ "$TEST_EXIT" -ne 0 ]; then
        echo "--- test log tail ---"
        tail -40 "$TEST_COUNTS_LOG" >&2
        if [ "$ALLOW_TEST_FAILURE" != "1" ]; then
            fail "the test suite failed (exit $TEST_EXIT); refusing to package a build whose tests do not pass"
        fi
        echo "WARNING: tests failed (exit $TEST_EXIT); packaging anyway because ALLOW_TEST_FAILURE=1"
    fi
elif [ -n "$TEST_LOG" ]; then
    [ -f "$TEST_LOG" ] || fail "test log not found: $TEST_LOG"
    TEST_COUNTS_LOG="$TEST_LOG"
else
    echo "WARNING: no test log; PROVENANCE will not claim a test result"
fi

# ---------------------------------------------------------------- two builds
for n in a b; do
    dir="$SRC/$n"
    target="$TARGET_ROOT/$n"
    rm -rf "$target"
    echo "== building archive $n =="
    echo "   CARGO_TARGET_DIR=$target"
    echo "   RUSTFLAGS=-C codegen-units=1 --remap-path-prefix=$dir= "
    (
        cd "$dir"
        CARGO_TARGET_DIR="$target" \
        RUSTFLAGS="-C codegen-units=1 --remap-path-prefix=$dir= " \
            cargo build --locked --release --offline
    )
    [ -x "$target/release/flow" ] || fail "binary missing after build $n"
    eval "HASH_${n}=\$(sha256sum \"\$target/release/flow\" | cut -d' ' -f1)"
done

REPRO=yes
if [ "$HASH_a" != "$HASH_b" ]; then
    REPRO=no
    echo "NOT REPRODUCIBLE on this host: a=$HASH_a b=$HASH_b"
    if [ "$ALLOW_DIFF" != "1" ]; then
        fail "the two clean builds differ; refusing to package (set ALLOW_DIFF=1 to package with identical=false)"
    fi
else
    echo "reproducible on this host: both archives produced $HASH_a"
fi

# ---------------------------------------------------------------- package
DIST_DIR="$DIST/flow-$VERSION-$TRIPLE"
echo "== packaging $DIST_DIR =="
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"
cp "$TARGET_ROOT/a/release/flow" "$DIST_DIR/flow"
cp "$SRC/a/LICENSE" "$DIST_DIR/LICENSE"
chmod 755 "$DIST_DIR/flow"
( cd "$DIST_DIR" && sha256sum flow LICENSE > SHA256SUMS.txt )

RUSTC="$(rustc --version)"
CARGO="$(cargo --version)"
RUSTC_HOST="$(rustc -vV | sed -n 's/^host: //p')"
HOST_OS="$( . /etc/os-release 2>/dev/null && echo "${PRETTY_NAME:-Linux}"; ) $(uname -r) $(uname -m)"
HOST_ARCH="$(uname -m)"
case "$TRIPLE" in
    *aarch64*) TARGET_ARCH=aarch64 ;;
    *x86_64*)  TARGET_ARCH=x86_64 ;;
    *)         TARGET_ARCH="$HOST_ARCH" ;;
esac

export SHA VERSION TRIPLE REPO SRC TARGET_ROOT TREE_OID ARCHIVE_SHA ARCHIVE_BYTES MANIFEST_SHA MANIFEST_ENTRIES
export LICENSE_BLOB_SIZE LICENSE_BLOB_OID CORE_AUTOCRLF
export HASH_A="$HASH_a" HASH_B="$HASH_b" REPRO NOW RUSTC CARGO RUSTC_HOST HOST_OS HOST_ARCH BRANCH
export DIST_DIR TEST_COUNTS_LOG TARGET_ARCH

python3 - <<'PY'
import hashlib, json, os, re, sys, uuid
from pathlib import Path

env = os.environ
dist = Path(env["DIST_DIR"])
sha = env["SHA"]
version = env["VERSION"]
triple = env["TRIPLE"]
now = env["NOW"]

def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()

binary = dist / "flow"
license_path = dist / "LICENSE"
sums = dist / "SHA256SUMS.txt"
binary_hash = sha256(binary)
license_hash = sha256(license_path)
license_first = license_path.read_text(encoding="utf-8", errors="replace").splitlines()[0].strip()
spdx = "MPL-2.0" if "Mozilla Public License Version 2.0" in license_first else None
license_raw = license_path.read_bytes()
license_normalized = license_raw.replace(b"\r\n", b"\n")
license_normalized_hash = hashlib.sha256(license_normalized).hexdigest()
license_blob_bytes = int(env["LICENSE_BLOB_SIZE"])
license_blob_oid = env["LICENSE_BLOB_OID"]
license_line_endings = "LF" if license_path.stat().st_size == license_blob_bytes else "CRLF"

# Deterministic serial: same commit/target -> same UUID, so two rebuilds do not
# look like two products. Version 4/variant bits keep it a well-formed UUID.
seed = hashlib.sha256(f"flow|{version}|{sha}|{triple}".encode()).hexdigest()
serial = f"{seed[0:8]}-{seed[8:12]}-4{seed[13:16]}-8{seed[17:20]}-{seed[20:32]}"

bom_ref = f"flow@{version}:{triple}"
license_entry = ({"license": {"id": spdx, "acknowledgement": "declared"}} if spdx
                 else {"license": {"name": license_first, "acknowledgement": "declared"}})
sbom = {
    "bomFormat": "CycloneDX",
    "specVersion": "1.5",
    "serialNumber": f"urn:uuid:{serial}",
    "version": 1,
    "metadata": {
        "timestamp": now,
        "component": {
            "type": "application",
            "bom-ref": bom_ref,
            "name": "flow",
            "version": version,
            "purl": f"pkg:cargo/flow@{version}",
            "licenses": [license_entry],
            "hashes": [{"alg": "SHA-256", "content": binary_hash}],
            "externalReferences": [
                {"type": "vcs", "url": f"https://github.com/Rwanbt/Ravum-flow/commit/{sha}"}
            ],
        },
    },
    "components": [],
    "dependencies": [{"ref": bom_ref, "dependsOn": []}],
}
sbom_path = dist / "SBOM.json"
sbom_path.write_text(json.dumps(sbom, indent=2) + "\n", encoding="utf-8")

# Test counts: measured from the log, never assumed.
tests = None
log = env.get("TEST_COUNTS_LOG") or ""
if log and Path(log).is_file():
    passed = failed = ignored = 0
    per = {}
    current = None
    for line in Path(log).read_text(encoding="utf-8", errors="replace").splitlines():
        m = re.match(r"^\s*Running\s+(.+?)\s+\(", line)
        if m:
            current = m.group(1).strip()
        m = re.match(r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored", line)
        if m:
            p, f, i = (int(x) for x in m.groups())
            passed += p; failed += f; ignored += i
            if current:
                per[current] = {"passed": p, "failed": f, "ignored": i}
            current = None
    if passed or failed:
        tests = {
            "command": "cargo test --locked --all-targets --offline",
            "passed": passed,
            "failed": failed,
            "ignored": ignored,
            "per_binary": per,
        }
if not tests and log:
    raise SystemExit(f"no test result lines in {log}; refusing to write an ambiguous PROVENANCE")

verification = {
    "smoke_tests": "docs/reviews/FLOW-PACKAGING-WAVE3.md (section 5: clean-room smoke matrix, both platforms)"
}
if tests:
    verification["tests"] = tests

provenance = {
    "schema": "flow-provenance/1",
    "product": "flow",
    "version": version,
    "source": {
        "commit": sha,
        "branch": env["BRANCH"],
        "repository": "https://github.com/Rwanbt/Ravum-flow.git",
        "git_tree_oid": env["TREE_OID"],
        "git_archive_format": "tar",
        "git_archive_sha256": env["ARCHIVE_SHA"],
        "git_archive_bytes": int(env["ARCHIVE_BYTES"]),
        "tree_manifest_sha256": env["MANIFEST_SHA"],
        "tree_manifest_entries": int(env["MANIFEST_ENTRIES"]),
        "tree_manifest_command": f"git -C {env['REPO']} ls-tree -r --full-tree {sha}  (SHA-256 of the LF-terminated stdout)",
        "archive_command": f"git -C {env['REPO']} archive {sha} | tar -x -C <clean-dir>  (the pipeline used by flow-ci.yml)",
        "host_git_core_autocrlf": env.get("CORE_AUTOCRLF", ""),
        "archive_extraction_note": "the extraction is the flow-ci.yml pipeline with this host's Git config; the commit, tree OID and tree-manifest are blob-based and unaffected",
        "git_archive_note": "the archive hash is host-specific (tar serialisation and host core.autocrlf); the tree OID and tree-manifest are the portable source identity",
    },
    "license": {
        "spdx": spdx or "unknown",
        "shipped_bytes": license_path.stat().st_size,
        "shipped_sha256": license_hash,
        "shipped_line_endings": license_line_endings,
        "committed_blob_bytes": license_blob_bytes,
        "committed_blob_oid": license_blob_oid,
        "normalized_text_sha256": license_normalized_hash,
        "note": "same licence text: any size difference against the Windows artefact is line endings only (host core.autocrlf); the normalized text hash is the comparison key",
    },
    "target": {"triple": triple, "os": "linux", "arch": env.get("TARGET_ARCH", "x86_64"), "abi": "gnu"},
    "toolchain": {
        "rustc": env["RUSTC"],
        "cargo": env["CARGO"],
        "rustc_host": env["RUSTC_HOST"],
        "pinned_by": "rust-toolchain.toml (channel 1.95.0)",
    },
    "build": {
        "command": "cargo build --locked --release --offline",
        "working_directory": f"{env.get('SRC', '/tmp/flow-linux-src')}/a",
        "env": {
            "RUSTFLAGS": f"-C codegen-units=1 --remap-path-prefix={env.get('SRC', '/tmp/flow-linux-src')}/a= ",
            "CARGO_TARGET_DIR": f"{env.get('TARGET_ROOT', '/tmp/flow-target-release')}/a",
        },
        "offline": True,
        "locked": True,
        "dependencies": "none (Cargo.lock has one package: flow)",
        "windows_only_flag_absent": "-C link-arg=/Brepro is MSVC-only and is not used on the GNU target",
    },
    "reproducibility": {
        "method": "two clean git-archive checkouts of the same commit in two different directories, built sequentially with -C codegen-units=1 and --remap-path-prefix",
        "build_a_sha256": env["HASH_A"],
        "build_b_sha256": env["HASH_B"],
        "identical": env["REPRO"] == "yes",
        "scope": "per host and per toolchain: measured on this WSL Ubuntu with rustc 1.95.0; cross-host identity is NOT claimed",
        "measured_at_utc": now,
    },
    "artifacts": [
        {"path": "flow", "sha256": binary_hash, "bytes": binary.stat().st_size},
        {"path": "LICENSE", "sha256": license_hash, "bytes": license_path.stat().st_size},
        {"path": "SHA256SUMS.txt", "sha256": sha256(sums), "bytes": sums.stat().st_size},
        {"path": "SBOM.json", "sha256": sha256(sbom_path), "bytes": sbom_path.stat().st_size},
    ],
    "artifacts_note": "PROVENANCE.json cannot list its own hash; SHA256SUMS.txt covers the distributable binary and LICENSE only",
    "build_date_utc": now,
    "host": {"os": env["HOST_OS"], "arch": env["HOST_ARCH"], "rustc_target": env["RUSTC_HOST"]},
    "verification": verification,
    "verification_summary_reference": "docs/reviews/FLOW-PACKAGING-WAVE3.md (sections 4-6: tests, smoke tests, reproducibility)",
    "unsigned": True,
}
prov_path = dist / "PROVENANCE.json"
prov_path.write_text(json.dumps(provenance, indent=2) + "\n", encoding="utf-8")

print(f"flow        {binary_hash} ({binary.stat().st_size} bytes)")
print(f"LICENSE     {license_hash} ({license_path.stat().st_size} bytes)")
print(f"SBOM.json   {sha256(sbom_path)}")
print(f"SHA256SUMS  {sha256(sums)}")
print(f"PROVENANCE  {sha256(prov_path)}")
if tests:
    print(f"tests       {tests['passed']} passed / {tests['failed']} failed / {tests['ignored']} ignored")
PY

echo
echo "== result =="
echo "dist:   $DIST_DIR"
echo "sha256: $(sha256sum "$DIST_DIR/flow" | cut -d' ' -f1)"
if [ "$REPRO" = "yes" ]; then
    echo "reproducibility: IDENTICAL on this host ($HASH_a)"
else
    echo "reproducibility: NOT REPRODUCIBLE on this host (a=$HASH_a b=$HASH_b)"
fi
