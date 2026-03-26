#!/bin/bash
# SessionStart: inject phase status and key constraints.

cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0

echo "=== AUTONOMIC SESSION CONTEXT ==="

# Current phase from CLAUDE.md
PHASE=$(grep -m1 "Current Phase" CLAUDE.md 2>/dev/null || echo "Unknown")
echo "Phase: $PHASE"

echo ""
echo "Key Constraints (always active):"
echo "  XD-001: Scheduler MUST spawn Claude via SessionManager, never directly"
echo "  XD-002: All traces go to PostgreSQL via SessionManager, not ad-hoc JSONL"
echo "  XD-005: stderr must be consumed concurrently with stdout (deadlock prevention)"
echo "  XD-006: Single RateBudget contract shared across all subsystems"
echo "  XD-008: Rollback must preserve gitignored files (secrets.toml, WAL)"

echo ""
echo "Dispatch Table:"
echo "  Evolution work -> docs/architecture/evolution-engine.md"
echo "  Memory work    -> docs/architecture/memory-system.md"
echo "  Session work   -> docs/architecture/session-management.md"
echo "  Hook work      -> docs/architecture/hook-system.md"
echo "  Scheduler work -> docs/architecture/scheduler.md"
echo "  State work     -> docs/architecture/state-management.md"
echo "  Model routing  -> docs/architecture/model-routing.md"
echo "  Full overview  -> docs/architecture/overview.md"

echo ""
echo "=== END SESSION CONTEXT ==="
exit 0
