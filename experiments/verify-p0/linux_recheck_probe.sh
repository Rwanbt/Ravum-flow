#!/bin/bash
# Linux (WSL) data point for 7a034ff: the post-run re-check proves the Unix mode
# of declared files. An obligation chmods a declared 100644 file to 0755; the
# re-check must refuse with CHECKOUT_MUTATED_DURING_RUN and write no receipt.
# Built from `git archive 7a034ff` (pinned), not from the moving worktree.
set -u

SRC=/tmp/flow-7a-linux-src
TGT=/tmp/flow-7a-linux-target
rm -rf "$SRC" "$TGT"
mkdir -p "$SRC"
( cd /mnt/d/App/Ravum-flow && git archive 7a034ff ) | tar -x -C "$SRC" || {
  echo "ARCHIVE_FAILED"; exit 1; }
( cd "$SRC" && CARGO_TARGET_DIR="$TGT" cargo build --locked --offline >/dev/null 2>&1 ) || {
  echo "BUILD_FAILED"; exit 1; }
FLOW="$TGT/debug/flow"
echo "flow version: $($FLOW version | tr -d '\n' | head -c 70)"

# committed gates at the pin
( cd "$SRC" && CARGO_TARGET_DIR="$TGT" cargo test --locked --offline --test git_fidelity 2>&1 \
    | tail -5 )

R=$(mktemp -d /tmp/flow-7a-mode.XXXXXX)
git -C "$R" init -q -b main
git -C "$R" config user.name p
git -C "$R" config user.email p@x
git -C "$R" config core.autocrlf false
printf 'alpha\n' > "$R/a.txt"
git -C "$R" add -A
git -C "$R" commit -q -m seed
mkdir -p "$R/.flow"
OBL='{"schema":"flow-config/1","policy_version":"FLOW-POLICY-1","execution_boundary":"local-trusted","obligations":[{"id":"chmod","program":"python3","args":["-c","import os; os.chmod(\"a.txt\", 0o755); print(\"chmod done\")"],"required":true,"timeout_s":30}],"disabled":false}'
printf '%s' "$OBL" > "$R/.flow/config.json"
( cd "$R" && "$FLOW" init --adopt-policy >/dev/null )
TREE=$(git -C "$R" write-tree)
echo "tree=$TREE mode_before=$(git -C "$R" ls-tree "$TREE" a.txt | cut -d' ' -f1)"
OUT=$(cd "$R" && "$FLOW" run 2>&1)
RC=$?
echo "MODE-CHANGE exit=$RC"
echo "$OUT" | tr -d '\n' | head -c 560
echo
echo "receipt written: $([ -f "$R/.flow/receipts/$TREE.json" ] && echo yes || echo no)"

# control: clean run on the same fixture shape records checkout_recheck OK
R2=$(mktemp -d /tmp/flow-7a-mode-ok.XXXXXX)
git -C "$R2" init -q -b main
git -C "$R2" config user.name p
git -C "$R2" config user.email p@x
git -C "$R2" config core.autocrlf false
printf 'alpha\n' > "$R2/a.txt"
git -C "$R2" add -A
git -C "$R2" commit -q -m seed
mkdir -p "$R2/.flow"
printf '%s' '{"schema":"flow-config/1","policy_version":"FLOW-POLICY-1","execution_boundary":"local-trusted","obligations":[{"id":"green","program":"git","args":["--version"],"required":true,"timeout_s":30}],"disabled":false}' > "$R2/.flow/config.json"
( cd "$R2" && "$FLOW" init --adopt-policy >/dev/null )
T2=$(git -C "$R2" write-tree)
OUT2=$(cd "$R2" && "$FLOW" run 2>&1)
RC2=$?
echo "CLEAN exit=$RC2 checkout_recheck=$(echo "$OUT2" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("checkout_recheck"))' 2>/dev/null)"
