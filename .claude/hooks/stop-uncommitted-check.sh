#!/bin/bash
# Stop: warn about uncommitted changes.

cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0

CHANGES=$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ')
if [ "$CHANGES" -gt 0 ]; then
  echo "NOTE: $CHANGES uncommitted change(s). Consider committing before ending session." >&2
fi

exit 0
