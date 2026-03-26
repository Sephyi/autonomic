# Architecture Invariants — Autonomic

These constraints are non-negotiable. Violating any of them will break the system.

## Crate Dependencies

- `autonomic-core` has NO dependencies on sibling crates (it's the foundation)
- `autonomic-db` depends only on `autonomic-core`
- `autonomic-container` depends only on `autonomic-core`
- `autonomic-daemon` depends on everything (it's the composition root)
- No circular dependencies. If A depends on B, B must not depend on A.

## Session Management (XD-001)

- **SessionManager is the ONLY way to spawn Claude Code.** No crate may call `Command::new("claude")` directly.
- The scheduler, evolution engine, and all other subsystems call `session_manager.run_session()`.
- This ensures: env filtering (`CLAUDECODE=1`), cost tracking, timeout enforcement, container lifecycle, orphan prevention.

## Data Flow (XD-002)

- **All experience traces go to PostgreSQL via SessionManager**, not ad-hoc JSONL files.
- Hooks write to a temp buffer; SessionManager flushes to `experience_traces` table on session end.

## Subprocess Safety (XD-005)

- **stderr MUST be consumed concurrently with stdout.** Use `tokio::io::BufReader` on both in a `select!` loop.
- Failing to consume stderr while reading stdout will deadlock when the OS pipe buffer fills.

## Rate Budget (XD-006)

- **Single `RateBudget` contract** defined in `autonomic-core`, shared by all subsystems.
- The scheduler, session manager, and decision engine all reference the same struct.

## Rollback Safety (XD-008)

- **`rollback` must preserve gitignored files.** Use `git checkout` on tracked files only, never `git clean -f`.
- `secrets.toml`, SQLite WAL files, and Podman volumes must survive rollback.

## Evolution Engine

- **Never modify frozen files.** The modification frontier is enforced at filesystem level (`chmod 444`).
- The watchdog verifies permissions on startup.
- The evolution engine's core logic, safety predicates, and energy function are outside the frontier.

## Technology Choices

- **gix** for git operations (pure Rust, not git2). No C/libgit2 dependency.
- **sqlx** for database (compile-time checked queries). Postgres primary, SQLite fallback.
- **Podman/Docker** for agent containers. OCI-compatible images.
- Secrets in `secrets.toml` (gitignored), **never** in `config.toml` (git-tracked).

## Container Architecture

- Orchestrator runs on **host**. Agent sessions run in **ephemeral containers**.
- Project directories bind-mounted into containers (rw). `.claude/` config mounted (ro).
- Containers connect to Postgres via internal network. No internet access by default.
