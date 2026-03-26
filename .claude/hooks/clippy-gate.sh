#!/bin/bash
# PostToolUse: run clippy on the affected crate after .rs edits.

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null)

[ -z "$FILE_PATH" ] && exit 0
[[ "$FILE_PATH" != *.rs ]] && exit 0
[ ! -f "$FILE_PATH" ] && exit 0

case "$FILE_PATH" in
  */crates/autonomic-core/*)       CRATE="autonomic-core" ;;
  */crates/autonomic-db/*)         CRATE="autonomic-db" ;;
  */crates/autonomic-container/*)  CRATE="autonomic-container" ;;
  */crates/autonomic-memory/*)     CRATE="autonomic-memory" ;;
  */crates/autonomic-evolution/*)  CRATE="autonomic-evolution" ;;
  */crates/autonomic-session/*)    CRATE="autonomic-session" ;;
  */crates/autonomic-scheduler/*)  CRATE="autonomic-scheduler" ;;
  */crates/autonomic-hooks/*)      CRATE="autonomic-hooks" ;;
  */crates/autonomic-routing/*)    CRATE="autonomic-routing" ;;
  */crates/autonomic-state/*)      CRATE="autonomic-state" ;;
  */crates/autonomic-daemon/*)     CRATE="autonomic-daemon" ;;
  */crates/autonomic-watchdog/*)   CRATE="autonomic-watchdog" ;;
  */crates/autonomic-cli/*)        CRATE="autonomic-cli" ;;
  *) exit 0 ;;
esac

cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0
cargo clippy -p "$CRATE" --all-targets -- -D warnings 2>&1
