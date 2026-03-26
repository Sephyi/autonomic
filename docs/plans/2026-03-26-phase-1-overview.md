# Phase 1: Foundation — Overview

> **For agentic workers:** This is an overview plan. Execute the sub-plans in order: Phase 1A → 1B → 1C.

**Goal:** Persistent daemon with project registry, session management, git-backed state, and basic metrics (FR-001 through FR-005).

**Architecture:** Bottom-up build across 8 crates. Core types first, then database + state, then session management, then daemon + watchdog, then CLI. Each sub-plan produces working, testable software independently.

**Tech Stack:** Rust 2024 (1.94), tokio, axum, sqlx (Postgres), gix, figment, clap, tracing, thiserror

## Sub-Plans

| Plan | Crates | FRs Covered | Depends On |
| --- | --- | --- | --- |
| [Phase 1A: Foundation Layer](2026-03-26-phase-1a-foundation-layer.md) | `autonomic-core`, `autonomic-db`, `autonomic-state` | FR-004 (Git State), FR-005 partial (trace schema) | Environment setup | **DONE** |
| [Phase 1B: Session Pipeline](2026-03-26-phase-1b-session-pipeline.md) | `autonomic-session`, `autonomic-container`, `autonomic-daemon`, `autonomic-watchdog` | FR-001 (Daemon+Watchdog), FR-003 (Sessions) | Phase 1A | **DONE** |
| [Phase 1C: User Interface](2026-03-26-phase-1c-user-interface.md) | `autonomic-cli` | FR-002 (Project Registry), FR-005 (Metrics), FR-001 remaining | Phase 1A + 1B | Planned |

## Dependency Graph

```txt
Phase 1A: Foundation Layer
  autonomic-core (types, errors, config, RateBudget)
    ↓
  autonomic-db (pool, migrations, query helpers)
    ↓
  autonomic-state (git-backed state, lock, atomic writes, snapshots)

Phase 1B: Session Pipeline (depends on 1A)
  autonomic-container (ContainerRuntime trait, host fallback)
    ↓
  autonomic-session (SessionConfig, output parser, cost tracker)
    ↓
  autonomic-daemon (axum HTTP, PID, signals, tracing, launchd)
    ↓
  autonomic-watchdog (health polling, crash rollback)

Phase 1C: User Interface (depends on 1A + 1B)
  autonomic-cli
    ├── project add/list (FR-002)
    ├── status dashboard
    ├── metrics show/export (FR-005)
    ├── snapshot/rollback (FR-004)
    └── budget show
```

## Key Decisions for Phase 1

1. **Host-first, containers later** (transitional): Phase 1 spawns Claude directly on the host via `tokio::process::Command`. The `ContainerRuntime` trait exists with a `HostRuntime` implementation. This is a pragmatic stepping stone — the PRD v0.5 centers on container architecture, and `PodmanRuntime` will be the primary implementation by Phase 2. The trait abstraction ensures zero refactoring when containers arrive.

2. **PostgreSQL required from day 1**: All shared state (sessions, metrics, traces) goes to Postgres per XD-002/XD-010. The `infra/compose.yaml` must be running before the daemon starts.

3. **Single-session first**: No worktree isolation or parallel sessions in 1A. Session management handles one session at a time. Concurrency added in 1B.

4. **gix for git ops**: Pure Rust, no libgit2. All state mutations auto-commit per architecture spec.

5. **Secrets from file**: Per XD-011, daemon reads `secrets.toml` directly — no reliance on env vars from launchd.

## Milestones

| Milestone | Tag | What Works |
| --- | --- | --- |
| Phase 1A complete | `milestone/phase-1a` | Core types compile, DB pool connects, state store init/commit/snapshot/rollback works, all with tests |
| Phase 1B complete | `milestone/phase-1b` | Daemon starts via launchd, spawns Claude sessions, parses output, tracks cost, watchdog monitors health |
| Phase 1C complete | `milestone/phase-1` | Full CLI: project management, session status, metrics display, snapshot/rollback. End-to-end integration tested. |
