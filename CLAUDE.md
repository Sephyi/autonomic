# Autonomic

Persistent, self-improving orchestrator for autonomous AI-assisted software development. Rust daemon managing Claude Code instances across projects with containerized agents, archive-based evolution, and structural enforcement.

## Quick Reference

```bash
cargo build --workspace          # Build all 13 crates
cargo test --workspace           # Run all tests
cargo clippy --workspace --all-targets -- -D warnings  # Lint
cargo fmt --check --all          # Format check
cargo deny check                 # Dependency audit
podman compose up -d             # Start Postgres (or: docker compose up -d)
podman compose ps                # Check infrastructure health
```

## Architecture

13-crate Rust workspace. Daemon runs on host, agents run in ephemeral Podman/Docker containers, shared state in PostgreSQL.

```txt
HOST
  +-- autonomicd (daemon)     +-- podman compose
  +-- autonomic-watchdog      |     +-- postgres:17
                              |     +-- agent containers (ephemeral)
```

### Crate Map

| Crate | Purpose |
| --- | --- |
| `autonomic-core` | Types, traits, config (figment), errors (thiserror) |
| `autonomic-db` | sqlx migrations, schemas, compile-time checked queries |
| `autonomic-container` | Podman/Docker abstraction (`ContainerRuntime` trait) |
| `autonomic-memory` | Memory store (tsvector FTS + pgvector semantic) |
| `autonomic-evolution` | Archive-based evolution, flip gating, convergence |
| `autonomic-session` | Container lifecycle + Claude Code management |
| `autonomic-scheduler` | Cron + 6 trigger types, warm pool, rate budget |
| `autonomic-hooks` | Hook management (standalone-capable) |
| `autonomic-routing` | Task classification, model routing, performance matrix |
| `autonomic-state` | Git-backed config state + container volumes |
| `autonomic-daemon` | Main binary (`autonomicd`) |
| `autonomic-watchdog` | Lightweight health monitor |
| `autonomic-cli` | CLI interface (`autonomic`) |

## Current Phase

**Phase 0: Environment Setup** -- In progress. Next: Phase 1 -- Foundation (FR-001 through FR-005).

## Core Principles

1. **Structure > Willpower** -- Hooks enforce behavior. Instructions suggest it. A 10-line hook is a guarantee.
2. **Archive-Based Evolution** -- Append-only variant archive, performance + novelty selection, never prune.
3. **Gate on Regressions** -- Flip-centered gating (P2P/P2F/F2P/F2F). Regression rate is the safety metric.
4. **Bounded Self-Modification** -- Lyapunov energy function, provable convergence, frozen modification frontier.
5. **Verifiable Outcomes Only** -- Scaffold improvements converge. Model capability improvements do not.

## Dispatch Table

When working on a specific subsystem, read the corresponding architecture doc first:

| Area | Read First |
| --- | --- |
| Evolution engine | `docs/architecture/evolution-engine.md` |
| Memory system | `docs/architecture/memory-system.md` |
| Session management | `docs/architecture/session-management.md` |
| Hook system | `docs/architecture/hook-system.md` |
| Scheduling | `docs/architecture/scheduler.md` |
| State management | `docs/architecture/state-management.md` |
| Model routing | `docs/architecture/model-routing.md` |
| Full overview | `docs/architecture/overview.md` |
| Requirements | `PRD.md` (v0.5, 26 FRs, 6 phases) |
| All sources | `SOURCES.md` |

## Key Constraints (Always Active)

- **XD-001**: Scheduler MUST spawn Claude via SessionManager, never `Command::new("claude")` directly
- **XD-002**: All experience traces go to PostgreSQL via SessionManager, not ad-hoc JSONL
- **XD-005**: stderr must be consumed concurrently with stdout (deadlock prevention)
- **XD-006**: Single `RateBudget` contract in `autonomic-core`, shared by all subsystems
- **XD-008**: `rollback` must preserve gitignored files (`secrets.toml`, WAL files) -- use `git checkout` on tracked files only

Full list: `.claude/specs/cross-document-constraints.md` (XD-001 through XD-011)

## Agents

| Agent | When to Use |
| --- | --- |
| `rust-coder` | General Rust implementation (worktree isolation) |
| `evolution-engineer` | Evolution engine, archive, gating, convergence work |
| `session-architect` | Session management, subprocess, container lifecycle |
| `security-reviewer` | Safety review of PRs and implementations (read-only) |
| `cargo-dep-auditor` | Dependency audit, version checks (read-only) |

## Testing

- `proptest` for property-based invariant testing
- `insta` for serialization snapshot testing
- `#[tokio::test]` for async tests
- Integration tests per crate in `tests/` directories
- `cargo test -p <crate>` for targeted testing

## Commits

Conventional commits, one concern per commit. Hooks enforce `cargo fmt` + `cargo clippy` before commit.

```txt
feat(core): add RateBudget type with sliding window tracking
fix(session): consume stderr concurrently to prevent deadlock
docs(evolution): update flip-centered gating algorithm
test(memory): add proptest for decay score invariants
build(container): add pgvector extension to migration
```
