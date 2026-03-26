# Cross-Document Implementation Constraints

These constraints span multiple architecture docs. Identified by Codex (gpt-5.4) reviewing all docs simultaneously. Each must be resolved during implementation.

## XD-001: Scheduler via SessionManager

**Constraint**: Scheduler MUST spawn Claude via SessionManager, never `Command::new("claude")` directly.

**Affected docs**: `session-management.md`, `scheduler.md`

**Verification**: `grep -r 'Command::new.*claude' crates/` should return zero results outside `autonomic-session`.

**If violated**: Sessions bypass env filtering, cost tracking, timeout enforcement, container lifecycle, and orphan prevention.

## XD-002: Traces to PostgreSQL

**Constraint**: All experience traces go to PostgreSQL via SessionManager Stop handler, not ad-hoc JSONL.

**Affected docs**: `evolution-engine.md`, `hook-system.md`, `memory-system.md`

**Verification**: No `.jsonl` trace files should exist in `~/.autonomic/`. All traces queryable via `SELECT * FROM experience_traces`.

**If violated**: Evolution engine cannot find traces. Memory system and evolution engine diverge on data sources.

## XD-003: Commit Granularity

**Constraint**: JSON/TOML file mutations commit immediately via git. PostgreSQL data commits at snapshot boundaries only.

**Affected docs**: `state-management.md`, `PRD.md`

**Verification**: "Every mutation" means every CONFIG FILE mutation, not every DB row insert.

**If violated**: Git history becomes either too noisy (committing DB changes) or too sparse (missing config changes).

## XD-004: Trigger Types Phased

**Constraint**: 6 trigger types are phased. Cron in Phase 3. Remaining triggers (once, interval, poll, on_message, webhook) in Phase 3.5.

**Affected docs**: `scheduler.md`

**Verification**: `JobDefinition` starts with `Trigger::Cron` only. `Trigger` enum expanded in Phase 3.5.

**If violated**: Over-engineering in Phase 3. Keep it simple — cron first.

## XD-005: Concurrent stderr Consumption

**Constraint**: stderr must be consumed concurrently with stdout using `tokio::io::BufReader` in `select!` loop.

**Affected docs**: `session-management.md`

**Verification**: Session spawn code must show `select!` over both `stdout_lines` and `stderr_lines`.

**If violated**: OS pipe buffer fills when Claude writes to stderr → stdout read blocks → deadlock.

## XD-006: Single RateBudget Contract

**Constraint**: Single `RateBudget` struct defined in `autonomic-core`, shared by PRD, scheduler, and session manager.

**Affected docs**: `scheduler.md`, `session-management.md`, `PRD.md`

**Verification**: `grep -r 'RateBudget' crates/` should show definition in `autonomic-core` and usage in other crates.

**If violated**: Subsystems track budget independently → overspend or conflicting allocations.

## XD-007: Per-Project Active Variants

**Constraint**: Evolution archive needs per-project active variants, not a single global one.

**Affected docs**: `evolution-engine.md`

**Verification**: `active_variant` should be `HashMap<ProjectId, VariantId>` + `global_active: VariantId`.

**If violated**: One project's evolution deployment changes all projects' configurations.

## XD-008: Rollback Preserves Gitignored

**Constraint**: `rollback` must preserve gitignored files. Use `git checkout` on tracked files only, never `git clean -f`.

**Affected docs**: `state-management.md`

**Verification**: Rollback function does NOT call `git clean`. `secrets.toml`, WAL files, Podman volumes survive rollback.

**If violated**: Rollback deletes secrets, corrupts database, breaks container infrastructure.

## XD-009: Memory Sync Metadata

**Constraint**: PostgreSQL needs a `memory_md_sync` table for MEMORY.md diffing: `project_id TEXT, last_hash TEXT, last_sync TIMESTAMPTZ`.

**Affected docs**: `memory-system.md`

**Verification**: Migration includes `memory_md_sync` table. Sync logic uses last_hash to detect changes.

**If violated**: MEMORY.md sync re-processes unchanged content on every session, or misses changes.

## XD-010: Single Database

**Constraint**: All tables (sessions, metrics, traces, variants, probes, proposals, learnings, energy_history, memory_entries) belong in one PostgreSQL database, not separate files.

**Affected docs**: `state-management.md`, `evolution-engine.md`, `memory-system.md`

**Verification**: Single `DATABASE_URL` in config. All migrations in `migrations/` directory.

**If violated**: Multiple database connections, complex transaction coordination, deployment complexity.

## XD-011: LaunchAgent Secrets

**Constraint**: LaunchAgent plist does not reliably inherit shell env. Secrets must load from `secrets.toml` file, not env vars.

**Affected docs**: `state-management.md`, `PRD.md`

**Verification**: Daemon startup reads `secrets.toml` directly. No `$API_KEY` in plist `EnvironmentVariables`.

**If violated**: Daemon starts but can't authenticate to Postgres, Codex, or Gemini.
