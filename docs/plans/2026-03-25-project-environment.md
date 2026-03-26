# Project Environment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Set up the complete Claude Code project environment for Autonomic so any future session can implement Phase 1 autonomously.

**Architecture:** Layered context system — rules (always loaded), agents (on-dispatch), specs (on-demand), hooks (mechanical enforcement). Rust workspace with 13 crate stubs. Containerized agents (Podman/Docker). PostgreSQL for shared state. Git-backed config.

**Tech Stack:** Rust 2024 (1.94), tokio, axum, sqlx (Postgres+SQLite), gix, figment, clap, croner, Podman/Docker, PostgreSQL 17

**Spec:** `docs/specs/2026-03-25-project-environment-design.md`

## File Map

```txt
CREATE: Cargo.toml                          # Workspace root (13 crates)
CREATE: rust-toolchain.toml                 # Pin Rust 1.94
CREATE: Containerfile                       # OCI base image for agent containers
CREATE: compose.yaml                        # Postgres + network infrastructure
CREATE: migrations/001_initial.sql          # Initial Postgres schema
CREATE: clippy.toml                         # Workspace clippy config
CREATE: deny.toml                           # cargo-deny config
CREATE: LICENSE                             # Placeholder
CREATE: README.md                           # Project README
CREATE: CLAUDE.md                           # Project instructions
CREATE: .claude/settings.json               # Hook registration
CREATE: .claude/rules/rust.md               # Always-loaded Rust rules
CREATE: .claude/rules/architecture-invariants.md  # Always-loaded constraints
CREATE: .claude/agents/rust-coder.md        # Implementation agent
CREATE: .claude/agents/evolution-engineer.md # Evolution agent
CREATE: .claude/agents/session-architect.md  # Session agent
CREATE: .claude/agents/security-reviewer.md  # Safety agent
CREATE: .claude/agents/cargo-dep-auditor.md  # Dep audit agent
CREATE: .claude/specs/rust-1.94-features.md  # Rust feature catalog
CREATE: .claude/specs/async-concurrency.md   # Tokio patterns
CREATE: .claude/specs/cross-document-constraints.md  # XD-001..011
CREATE: .claude/hooks/rust-fmt.sh           # PostToolUse: cargo fmt
CREATE: .claude/hooks/clippy-gate.sh        # PostToolUse: clippy
CREATE: .claude/hooks/cargo-test-gate.sh    # PostToolUse: cargo test
CREATE: .claude/hooks/block-generated-files.sh  # PreToolUse: block generated
CREATE: .claude/hooks/pre-commit-gate.sh    # PreToolUse: commit guard
CREATE: .claude/hooks/fmt-drift-guard.sh    # Stop: workspace fmt check
CREATE: .claude/hooks/stop-uncommitted-check.sh  # Stop: uncommitted warn
CREATE: .claude/hooks/session-context.sh    # SessionStart: context inject
CREATE: .claude/known-dep-versions.toml     # Version pins
CREATE: crates/autonomic-core/Cargo.toml    # Core types crate
CREATE: crates/autonomic-core/src/lib.rs
CREATE: crates/autonomic-db/Cargo.toml      # Database crate (sqlx)
CREATE: crates/autonomic-db/src/lib.rs
CREATE: crates/autonomic-container/Cargo.toml  # Container runtime crate
CREATE: crates/autonomic-container/src/lib.rs
CREATE: crates/autonomic-memory/Cargo.toml  # Memory crate
CREATE: crates/autonomic-memory/src/lib.rs
CREATE: crates/autonomic-evolution/Cargo.toml  # Evolution crate
CREATE: crates/autonomic-evolution/src/lib.rs
CREATE: crates/autonomic-session/Cargo.toml # Session crate
CREATE: crates/autonomic-session/src/lib.rs
CREATE: crates/autonomic-scheduler/Cargo.toml  # Scheduler crate
CREATE: crates/autonomic-scheduler/src/lib.rs
CREATE: crates/autonomic-hooks/Cargo.toml   # Hooks crate (standalone)
CREATE: crates/autonomic-hooks/src/lib.rs
CREATE: crates/autonomic-routing/Cargo.toml # Routing crate
CREATE: crates/autonomic-routing/src/lib.rs
CREATE: crates/autonomic-state/Cargo.toml   # State crate
CREATE: crates/autonomic-state/src/lib.rs
CREATE: crates/autonomic-daemon/Cargo.toml  # Daemon binary
CREATE: crates/autonomic-daemon/src/main.rs
CREATE: crates/autonomic-watchdog/Cargo.toml  # Watchdog binary
CREATE: crates/autonomic-watchdog/src/main.rs
CREATE: crates/autonomic-cli/Cargo.toml     # CLI binary
CREATE: crates/autonomic-cli/src/main.rs
```

## Task 1: Workspace Root and Build Config

**Files:**
- Create: `Cargo.toml`
- Create: `rust-toolchain.toml`
- Create: `clippy.toml`
- Create: `deny.toml`

- [ ] **Step 1: Create workspace Cargo.toml**

Copy the exact content from spec §Cargo.toml Workspace (lines 165-212). This is a virtual workspace — no `[package]` section at root.

- [ ] **Step 2: Create rust-toolchain.toml**

```toml
[toolchain]
channel = "1.94"
```

- [ ] **Step 3: Create clippy.toml**

Copy the exact content from spec §clippy.toml (lines 246-259).

- [ ] **Step 4: Create deny.toml**

Copy the exact content from spec §deny.toml (lines 263-288). Add `"LicenseRef-Proprietary"` to the `[licenses] allow` array so the workspace's own crates pass license checks:

```toml
allow = [
    "MIT",
    "Apache-2.0",
    # ... rest of list from spec ...
    "LicenseRef-Proprietary",
]
```

**Note: Do NOT commit yet — the workspace Cargo.toml references members that don't exist until the crate stubs are created. Continue to Steps 6+ below before committing.**

## Task 2: Crate Stubs (Libraries) — same commit as Task 1

**Files:**
- Create: 8 library crate stubs (Cargo.toml + src/lib.rs each)

- [ ] **Step 1: Create autonomic-core**

```bash
mkdir -p crates/autonomic-core/src
```

`crates/autonomic-core/Cargo.toml`:

```toml
[package]
name = "autonomic-core"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Core types, traits, and configuration for Autonomic"

[dependencies]
```

`crates/autonomic-core/src/lib.rs`:

```rust
//! Core types, traits, and configuration for Autonomic.
```

- [ ] **Step 2: Create remaining 7 library crates**

Repeat the pattern for: `autonomic-memory`, `autonomic-evolution`, `autonomic-session`, `autonomic-scheduler`, `autonomic-hooks`, `autonomic-routing`, `autonomic-state`. Each gets the same template with appropriate `name` and `description`.

Descriptions:
- `autonomic-memory`: "PostgreSQL memory store with tsvector FTS, pgvector semantic search, context assembly, and MEMORY.md sync"
- `autonomic-evolution`: "Archive-based self-improvement with flip-centered gating and convergence bounds"
- `autonomic-session`: "Claude Code subprocess management, output parsing, and cost tracking"
- `autonomic-scheduler`: "Cron-based job scheduling with rate budget awareness"
- `autonomic-hooks`: "Hook management for Claude Code lifecycle events (standalone-capable)"
- `autonomic-routing`: "Task classification and model routing with empirical learning"
- `autonomic-state`: "Git-backed state management with point-in-time recovery"

**Note: Do NOT commit yet — binary stubs still needed. Continue to Task 3.**

## Task 3: Crate Stubs (Binaries) — same commit as Tasks 1-2

**Files:**
- Create: 3 binary crate stubs

- [ ] **Step 1: Create autonomic-daemon**

`crates/autonomic-daemon/Cargo.toml`:

```toml
[package]
name = "autonomic-daemon"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Autonomic orchestrator daemon"

[[bin]]
name = "autonomicd"
path = "src/main.rs"

[dependencies]
```

`crates/autonomic-daemon/src/main.rs`:

```rust
//! Autonomic orchestrator daemon.

fn main() {
    // TODO: implement — Phase 1 FR-001
}
```

- [ ] **Step 2: Create autonomic-watchdog**

Same Cargo.toml pattern. Binary name: `autonomic-watchdog`. Description: "Lightweight watchdog monitor for the Autonomic daemon". `src/main.rs`: `fn main() { // TODO: implement — Phase 1 FR-001 }`.

- [ ] **Step 3: Create autonomic-cli**

Same Cargo.toml pattern. Binary name: `autonomic`. Description: "CLI interface for the Autonomic orchestrator". `src/main.rs`: `fn main() { // TODO: implement — Phase 1 FR-001 }`.

- [ ] **Step 4: Verify workspace builds**

```bash
cargo check --workspace
```

Expected: clean check with no errors. All 13 crates resolve.

- [ ] **Step 5: Commit all workspace files together**

```bash
git add Cargo.toml rust-toolchain.toml clippy.toml deny.toml crates/
git commit -m "build: add Rust workspace with 13 crate stubs, toolchain pin, clippy and deny config"
```

## Task 4: LICENSE and README

**Files:**
- Create: `LICENSE`
- Create: `README.md`

- [ ] **Step 1: Create LICENSE**

```txt
All Rights Reserved

Copyright (c) 2026 Sephyi <me@sephy.io>

This software is proprietary. No license is granted for use, modification,
or distribution. A formal license will be announced in a future release.

For inquiries, contact: me@sephy.io
```

- [ ] **Step 2: Create README.md**

Follow the structure from spec §README.md Structure (lines 377-410). Include:
- Vision quote from PRD
- 3-4 sentence description
- Status section (Phase 0: Environment Setup)
- Architecture diagram from `docs/architecture/overview.md` (the text-based system diagram)
- Crate map listing all 13 crates
- Getting started: prerequisites (Rust 1.94, cargo-deny, **jq** for hook scripts), build, test, lint
- Documentation links (PRD.md, docs/architecture/, docs/research/, SOURCES.md)
- License notice

- [ ] **Step 3: Commit**

```bash
git add LICENSE README.md
git commit -m "docs: add LICENSE placeholder and README"
```

## Task 5: Claude Code Hook Scripts

**Files:**
- Create: 8 hook scripts in `.claude/hooks/`

- [ ] **Step 1: Create rust-fmt.sh**

Adapt from vox-scribe's `rust-fmt.sh`. Parse `$TOOL_INPUT` JSON for `file_path`, skip non-.rs files, run `cargo fmt` on the file.

```bash
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
```

- [ ] **Step 2: Create clippy-gate.sh**

Adapt from vox-scribe. Parse file path, map to crate via path prefix, run clippy on that crate.

```bash
#!/bin/bash
# PostToolUse: run clippy on the affected crate after .rs edits.

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null)

[ -z "$FILE_PATH" ] && exit 0
[[ "$FILE_PATH" != *.rs ]] && exit 0
[ ! -f "$FILE_PATH" ] && exit 0

case "$FILE_PATH" in
  */crates/autonomic-core/*)       CRATE="autonomic-core" ;;
  */crates/autonomic-memory/*)     CRATE="autonomic-memory" ;;
  */crates/autonomic-evolution/*)  CRATE="autonomic-evolution" ;;
  */crates/autonomic-session/*)    CRATE="autonomic-session" ;;
  */crates/autonomic-scheduler/*)  CRATE="autonomic-scheduler" ;;
  */crates/autonomic-hooks/*)      CRATE="autonomic-hooks" ;;
  */crates/autonomic-routing/*)    CRATE="autonomic-routing" ;;
  */crates/autonomic-state/*)      CRATE="autonomic-state" ;;
  */crates/autonomic-daemon/*)     CRATE="autonomic-daemon" ;;
  */crates/autonomic-watchdog/*)   CRATE="autonomic-watchdog" ;;
  */crates/autonomic-cli/*)        CRATE="autonomic-cli" ;;
  *) exit 0 ;;
esac

cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0
cargo clippy -p "$CRATE" --all-targets -- -D warnings 2>&1
```

- [ ] **Step 3: Create cargo-test-gate.sh**

```bash
#!/bin/bash
# PostToolUse: run tests on the affected crate after .rs edits.

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null)

[ -z "$FILE_PATH" ] && exit 0
[[ "$FILE_PATH" != *.rs ]] && exit 0
[ ! -f "$FILE_PATH" ] && exit 0

case "$FILE_PATH" in
  */crates/autonomic-core/*)       CRATE="autonomic-core" ;;
  */crates/autonomic-memory/*)     CRATE="autonomic-memory" ;;
  */crates/autonomic-evolution/*)  CRATE="autonomic-evolution" ;;
  */crates/autonomic-session/*)    CRATE="autonomic-session" ;;
  */crates/autonomic-scheduler/*)  CRATE="autonomic-scheduler" ;;
  */crates/autonomic-hooks/*)      CRATE="autonomic-hooks" ;;
  */crates/autonomic-routing/*)    CRATE="autonomic-routing" ;;
  */crates/autonomic-state/*)      CRATE="autonomic-state" ;;
  */crates/autonomic-daemon/*)     CRATE="autonomic-daemon" ;;
  */crates/autonomic-watchdog/*)   CRATE="autonomic-watchdog" ;;
  */crates/autonomic-cli/*)        CRATE="autonomic-cli" ;;
  *) exit 0 ;;
esac

cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0
cargo test -p "$CRATE" --quiet 2>&1
```

- [ ] **Step 4: Create block-generated-files.sh**

```bash
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
```

- [ ] **Step 5: Create pre-commit-gate.sh**

```bash
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
```

- [ ] **Step 6: Create fmt-drift-guard.sh**

```bash
#!/bin/bash
# Stop: catch cross-file rustfmt drift after a session.

cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0

if ! cargo fmt --check --all --quiet 2>/dev/null; then
  echo "WARNING: rustfmt drift detected across workspace. Run 'cargo fmt --all' before committing." >&2
fi

exit 0
```

- [ ] **Step 7: Create stop-uncommitted-check.sh**

```bash
#!/bin/bash
# Stop: warn about uncommitted changes.

cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0

CHANGES=$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ')
if [ "$CHANGES" -gt 0 ]; then
  echo "NOTE: $CHANGES uncommitted change(s). Consider committing before ending session." >&2
fi

exit 0
```

- [ ] **Step 8: Create session-context.sh**

Copy the exact content from spec §session-context.sh Design (lines 320-354).

- [ ] **Step 9: Make all hooks executable**

```bash
chmod +x .claude/hooks/*.sh
```

- [ ] **Step 10: Commit**

```bash
git add .claude/hooks/
git commit -m "ci: add Claude Code hook scripts (8 hooks)"
```

## Task 6: Hook Registration (settings.json)

**Files:**
- Create: `.claude/settings.json`

- [ ] **Step 1: Create settings.json**

Copy the JSON from spec §settings.json (lines 65-139). **Fix:** add `"timeout": 5` to the `block-generated-files.sh` entry (missing in spec JSON but specified in spec hook table as 5s).

- [ ] **Step 2: Commit**

```bash
git add .claude/settings.json
git commit -m "ci: register hooks in .claude/settings.json"
```

## Task 7: Rules (Always Loaded)

**Files:**
- Create: `.claude/rules/rust.md`
- Create: `.claude/rules/architecture-invariants.md`

- [ ] **Step 1: Create rust.md**

~60 lines covering: Edition 2024, MSRV 1.94, `#![forbid(unsafe_code)]` policy, thiserror 2.x for errors, tokio async rules (never block runtime, spawn_blocking for FFI, bounded channels), naming conventions (lowerCamelCase NO — snake_case YES), testing (proptest for properties, insta for snapshots, `#[tokio::test]` for async), commit conventions (conventional commits, one concern per commit).

- [ ] **Step 2: Create architecture-invariants.md**

~50 lines covering: crate dependency direction (core has no deps on siblings, daemon depends on everything), SessionManager is the ONLY way to spawn Claude Code (XD-001), all experience traces go to PostgreSQL (XD-002), stderr consumed concurrently with stdout (XD-005), single RateBudget contract (XD-006), rollback preserves gitignored files (XD-008), evolution engine never modifies frozen files, gix for git operations (not git2), secrets in secrets.toml (never config.toml).

- [ ] **Step 3: Commit**

```bash
git add .claude/rules/
git commit -m "docs: add Claude Code rules (rust.md, architecture-invariants.md)"
```

## Task 8: Specs (On-Demand Reference) — created before agents so referenced files exist

**Files:**
- Create: 3 spec files in `.claude/specs/`

- [ ] **Step 1: Create rust-1.94-features.md**

Adapt from vox-scribe's `.claude/specs/rust-1.94-features.md`. Keep the general Rust 1.94 features (let chains, generic_arg_infer, LazyLock::get, async closures, precise capturing, trait upcasting). Remove vox-scribe-specific audio pipeline features. Add relevant ones for Autonomic: process management patterns, PostgreSQL patterns, git operations.

- [ ] **Step 2: Create async-concurrency.md**

Tokio patterns for Autonomic specifically: subprocess management with `tokio::process::Command`, concurrent stdout/stderr consumption with `tokio::io::BufReader` in `select!`, `tokio::time::timeout` for session enforcement, `croner` integration with tokio for scheduling, `tokio::sync::mpsc` for inter-component communication, `tokio::sync::watch` for config broadcast, never block the runtime (spawn_blocking for gix operations if needed).

- [ ] **Step 3: Create cross-document-constraints.md**

XD-001 through XD-011 from PRD §7, each with:
- Constraint statement
- Which architecture docs are affected
- How to verify compliance
- What breaks if violated

- [ ] **Step 4: Commit**

```bash
git add .claude/specs/
git commit -m "docs: add Claude Code specs (Rust features, async patterns, constraints)"
```

## Task 9: Agents (On-Dispatch)

**Files:**
- Create: 5 agent files in `.claude/agents/`

- [ ] **Step 1: Create rust-coder.md**

YAML frontmatter: name, description, isolation: worktree, tools list. Body: knows crate structure (list all 11), Rust 1.94 features (reference .claude/specs/rust-1.94-features.md), async patterns (reference .claude/specs/async-concurrency.md), error handling (thiserror), testing (proptest + insta). Commit after each logical unit.

- [ ] **Step 2: Create evolution-engineer.md**

Frontmatter with worktree isolation. Body: knows archive architecture (reference docs/architecture/evolution-engine.md), flip-centered gating (P2P/P2F/F2P/F2F, regression rate formula), energy function and convergence (Lyapunov), cross-project evolution rounds (GEA pattern), sidecar deployment, timed verification windows (1h/24h/72h). Key research: reference docs/research/gea-group-evolving-agents.md, docs/research/agentdevel-release-engineering.md, docs/research/maria-os-godel-agent.md.

- [ ] **Step 3: Create session-architect.md**

Frontmatter with worktree isolation. Body: knows Claude Code CLI flags (--print, --output-format stream-json, --permission-mode acceptEdits, --resume, --append-system-prompt), NDJSON output parsing, completion detection (message.type == "result"), cost tracking (total_cost_usd), CLAUDECODE=1 env filtering, worktree isolation via gix, rate budget contract. Reference docs/architecture/session-management.md. Critical constraint: system prompt does NOT persist across --resume.

- [ ] **Step 4: Create security-reviewer.md**

Frontmatter with NO isolation (read-only). Tools: Read, Bash, Grep, Glob only. Body: knows modification frontier (what's frozen, what's modifiable), command guard patterns, filesystem permissions (chmod 444 on frozen files), secret management (secrets.toml, never config.toml), subprocess orphan prevention, CooperBench warning (50% lower success collaborating). Reviews PRs and implementations for safety violations.

- [ ] **Step 5: Create cargo-dep-auditor.md**

Frontmatter with NO isolation (read-only). Tools: Read, Bash, Grep, Glob only. Body: runs cargo-deny, checks advisory DB, verifies versions against .claude/known-dep-versions.toml, checks for yanked crates, checks license compliance.

- [ ] **Step 6: Commit**

```bash
git add .claude/agents/
git commit -m "docs: add Claude Code agents (5 specialist agents)"
```

## Task 10: CLAUDE.md and Known Deps

**Files:**
- Create: `CLAUDE.md`
- Create: `.claude/known-dep-versions.toml`

- [ ] **Step 1: Create CLAUDE.md**

Follow the 10-section structure from spec §CLAUDE.md Structure (lines 150-161). Keep to ~150-200 lines. Reference architecture docs via dispatch table. Include build commands, current phase status, core principles, agent overview.

Key sections:
1. "Autonomic — persistent, self-improving orchestrator for autonomous AI development"
2. Quick reference: `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo deny check`
3. Architecture summary + crate map (all 13 crates, one line each)
4. Current Phase: Phase 0 — Environment Setup (complete). Next: Phase 1 — Foundation.
5. Core principles: Structure > Willpower, Archive-Based Evolution, Gate on Regressions, Bounded Self-Modification, Verifiable Outcomes Only
6. Dispatch table mapping work areas to docs/architecture/*.md files
7. Top 5 XD constraints inline
8. Agent overview: when to dispatch each of the 5 agents
9. Testing: proptest for properties, insta for snapshots, integration tests per crate
10. Commits: conventional commits, one concern per commit, hooks enforce fmt+clippy before commit

- [ ] **Step 2: Create known-dep-versions.toml**

Copy exact content from spec §known-dep-versions.toml (lines 292-314).

- [ ] **Step 3: Commit**

```bash
git add CLAUDE.md .claude/known-dep-versions.toml
git commit -m "docs: add CLAUDE.md project instructions and dependency version pins"
```

## Task 11: Container Infrastructure

**Files:**
- Create: `Containerfile`
- Create: `compose.yaml`
- Create: `migrations/001_initial.sql`

- [ ] **Step 1: Create Containerfile**

```dockerfile
# Autonomic agent base image
# OCI-compatible — works with Podman and Docker
FROM rust:1.94-slim-bookworm

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    jq curl git ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install Claude Code CLI
RUN curl -fsSL https://cli.claude.ai/install.sh | sh

WORKDIR /workspace

# Default: run claude in print mode
ENTRYPOINT ["claude"]
CMD ["--help"]
```

- [ ] **Step 2: Create compose.yaml**

```yaml
# Autonomic infrastructure
# Usage: podman compose up -d  (or docker compose up -d)
services:
  postgres:
    image: postgres:17-alpine
    restart: unless-stopped
    environment:
      POSTGRES_DB: autonomic
      POSTGRES_USER: autonomic
      POSTGRES_PASSWORD_FILE: /run/secrets/pg_password
    volumes:
      - pgdata:/var/lib/postgresql/data
      - ./migrations:/docker-entrypoint-initdb.d:ro
    networks:
      - autonomic-net
    ports:
      - "127.0.0.1:5432:5432"  # Localhost only
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U autonomic"]
      interval: 10s
      timeout: 5s
      retries: 5

networks:
  autonomic-net:
    internal: true  # No internet access for agent containers

volumes:
  pgdata:

secrets:
  pg_password:
    file: ./secrets/pg_password.txt
```

- [ ] **Step 3: Create initial migration**

```sql
-- migrations/001_initial.sql
-- Autonomic initial schema

-- Enable extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- pgvector for semantic search (install separately if needed)
-- CREATE EXTENSION IF NOT EXISTS "vector";

-- Memory entries with full-text search
CREATE TABLE memory_entries (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    category TEXT NOT NULL,
    scope_type TEXT NOT NULL,
    scope_value TEXT,
    tags JSONB NOT NULL DEFAULT '[]',
    source_session TEXT,
    source_project TEXT,
    helpful_count INTEGER NOT NULL DEFAULT 0,
    misleading_count INTEGER NOT NULL DEFAULT 0,
    decay_rate DOUBLE PRECISION NOT NULL,
    retirement_policy TEXT NOT NULL DEFAULT 'auto',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_accessed TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    retired_at TIMESTAMPTZ,
    search_vector TSVECTOR GENERATED ALWAYS AS (
        to_tsvector('english', content || ' ' || COALESCE(tags::text, ''))
    ) STORED
);

CREATE INDEX idx_memory_search ON memory_entries USING GIN (search_vector);
CREATE INDEX idx_memory_scope ON memory_entries (scope_type, scope_value);
CREATE INDEX idx_memory_active ON memory_entries (retired_at) WHERE retired_at IS NULL;

-- Sessions
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    model TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'running',
    cost_usd DOUBLE PRECISION,
    duration_ms BIGINT,
    container_id TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

-- Experience traces
CREATE TABLE experience_traces (
    id TEXT PRIMARY KEY,
    session_id TEXT REFERENCES sessions(id),
    project_id TEXT NOT NULL,
    variant_id TEXT,
    model_used TEXT NOT NULL,
    task_type TEXT,
    outcome TEXT NOT NULL,
    duration_ms BIGINT,
    cost_usd DOUBLE PRECISION,
    compaction_count INTEGER DEFAULT 0,
    trace_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Model performance matrix
CREATE TABLE model_performance (
    model TEXT NOT NULL,
    task_type TEXT NOT NULL,
    outcome TEXT NOT NULL,
    tokens_used BIGINT,
    cost_usd DOUBLE PRECISION,
    duration_ms BIGINT,
    session_id TEXT,
    project_id TEXT,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_model_perf ON model_performance (model, task_type);
```

- [ ] **Step 4: Create secrets directory (gitignored)**

```bash
mkdir -p secrets
echo "autonomic_dev_password" > secrets/pg_password.txt
echo "secrets/" >> .gitignore
```

- [ ] **Step 5: Commit**

```bash
git add Containerfile compose.yaml migrations/ .gitignore
git commit -m "build: add container infrastructure (Containerfile, compose, initial migration)"
```

## Task 12: Final Verification

- [ ] **Step 1: Verify workspace builds**

```bash
cargo check --workspace
```

Expected: clean check, all 13 crates resolve, no errors.

- [ ] **Step 2: Verify clippy passes**

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: clean.

- [ ] **Step 3: Verify fmt is clean**

```bash
cargo fmt --check --all
```

Expected: clean.

- [ ] **Step 4: Verify hooks are executable**

```bash
ls -la .claude/hooks/*.sh
```

Expected: all 8 scripts have execute permission.

- [ ] **Step 5: Verify settings.json parses**

```bash
python3 -c "import json; json.load(open('.claude/settings.json'))" && echo "OK"
```

Expected: OK.

- [ ] **Step 6: Verify compose.yaml is valid**

```bash
podman compose config --quiet 2>/dev/null || docker compose config --quiet 2>/dev/null && echo "compose OK"
```

Expected: compose OK (validates YAML structure).

- [ ] **Step 7: Verify git state is clean**

```bash
git status
```

Expected: clean working tree, all files committed.

- [ ] **Step 8: Tag the milestone**

```bash
git tag -a milestone/environment-setup -m "Project environment: 13 crate stubs, 8 hooks, 5 agents, 3 specs, 2 rules, container infra"
```
