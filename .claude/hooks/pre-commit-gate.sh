#!/bin/bash
# PreToolUse(Bash): guard git commits — must pass fmt + clippy first.

INPUT=$(cat)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null)

# Only intercept git commit commands
echo "$COMMAND" | grep -qE '^git commit' || exit 0

cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0

ERRORS=""

if ! cargo fmt --check --all --quiet 2>/dev/null; then
  ERRORS="${ERRORS}FORMAT: cargo fmt --check failed. Run 'cargo fmt --all' first.\n"
fi

if ! cargo clippy --workspace --all-targets -- -D warnings 2>/dev/null; then
  ERRORS="${ERRORS}CLIPPY: cargo clippy found warnings. Fix before committing.\n"
fi

if [ -n "$ERRORS" ]; then
  echo -e "$ERRORS" >&2
  exit 2
fi

exit 0
