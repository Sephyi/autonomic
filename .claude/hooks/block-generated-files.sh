#!/bin/bash
# PreToolUse: block editing generated or managed files.

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null)

[ -z "$FILE_PATH" ] && exit 0

case "$FILE_PATH" in
  */Cargo.lock)
    echo "BLOCKED: Cargo.lock is auto-generated. Modify Cargo.toml instead." >&2
    exit 2 ;;
  */target/*)
    echo "BLOCKED: target/ is a build artifact. Do not edit." >&2
    exit 2 ;;
esac

exit 0
