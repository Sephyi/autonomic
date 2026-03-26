#!/bin/bash
# Stop: catch cross-file rustfmt drift after a session.

cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0

if ! cargo fmt --check --all --quiet 2>/dev/null; then
  echo "WARNING: rustfmt drift detected across workspace. Run 'cargo fmt --all' before committing." >&2
fi

exit 0
