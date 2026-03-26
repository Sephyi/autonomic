#!/bin/bash
# PostToolUse: auto-format Rust files after edit.

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null)

[ -z "$FILE_PATH" ] && exit 0
[[ "$FILE_PATH" != *.rs ]] && exit 0
[ ! -f "$FILE_PATH" ] && exit 0

cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0
cargo fmt --all --quiet 2>/dev/null
exit 0
