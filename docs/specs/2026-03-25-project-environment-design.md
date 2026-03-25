# Project Environment Design — Autonomic

**Date**: 2026-03-25
**Status**: Approved (revised after spec review)
**Scope**: Claude Code project environment setup for Autonomic

## Overview

Configure the Autonomic repo so that any future Claude Code session can implement the project autonomously from session one. The environment uses a layered context system: rules (always loaded), agents (on-dispatch), specs (on-demand), hooks (mechanical enforcement).

## Files to Create

### Top-Level Files

| File | Purpose |
| --- | --- |
| `CLAUDE.md` | Project instructions (~150-200 lines). Build commands, principles, crate map, phase status, dispatch table. References docs/architecture/ for depth. |
| `README.md` | Project README with vision, architecture overview, getting started, status. |
| `LICENSE` | Placeholder: "All Rights Reserved. License to be determined." |
| `Cargo.toml` | Workspace root with all 11 crate stubs as members. |
| `rust-toolchain.toml` | Pin to Rust 1.94 stable. |
| `clippy.toml` | Workspace clippy config with MSRV and disallowed methods/macros. |
| `deny.toml` | cargo-deny: license allowlist, advisory DB, duplicate detection. |

### Rules (Always Loaded Every Session)

| File | ~Lines | Purpose |
| --- | --- | --- |
| `.claude/rules/rust.md` | ~60 | Rust coding standards: Edition 2024, `#![forbid(unsafe_code)]`, thiserror 2.x, tokio async, naming, testing patterns |
| `.claude/rules/architecture-invariants.md` | ~50 | Hard constraints: crate dependency direction, SessionManager sole spawner, git-backed mutations, modification frontier, rate budget contract |

### Agents (Loaded on Dispatch)

| File | Isolation | Tools | Purpose |
| --- | --- | --- | --- |
| `.claude/agents/rust-coder.md` | worktree | Read, Write, Edit, Bash, Grep, Glob | Implementation specialist. Knows crate structure, Rust 1.94 features, async patterns, error handling. |
| `.claude/agents/evolution-engineer.md` | worktree | Read, Write, Edit, Bash, Grep, Glob | Evolution engine specialist. Knows archive, flip gating, energy function, cross-project rounds. |
| `.claude/agents/session-architect.md` | worktree | Read, Write, Edit, Bash, Grep, Glob | Session/subprocess specialist. Knows CLI flags, stream-json, completion detection, rate budget. |
| `.claude/agents/security-reviewer.md` | none | Read, Bash, Grep, Glob | Safety review. Knows modification frontier, command guard, permissions. Read-only. |
| `.claude/agents/cargo-dep-auditor.md` | none | Read, Bash, Grep, Glob | Dependency audit. Runs cargo-deny, checks advisory DB, verifies version pins. Read-only. |

### Specs (Loaded on Demand)

| File | Purpose |
| --- | --- |
| `.claude/specs/rust-1.94-features.md` | Full Rust 1.94 feature catalog with usage examples (adapted from vox-scribe). |
| `.claude/specs/async-concurrency.md` | Tokio patterns: subprocess management, scheduling, concurrent I/O, spawn_blocking, channels, select!. |
| `.claude/specs/cross-document-constraints.md` | XD-001 through XD-011 from Codex review. Cross-cutting implementation constraints. |

### Hooks

| File | Event | Matcher | Blocking | Timeout | Purpose |
| --- | --- | --- | --- | --- | --- |
| `rust-fmt.sh` | PostToolUse | Edit\|Write\|MultiEdit | no | 10s | Run `cargo fmt` on .rs file edits |
| `clippy-gate.sh` | PostToolUse | Edit\|Write\|MultiEdit | no | 30s | Run clippy on affected crate |
| `cargo-test-gate.sh` | PostToolUse | Edit\|Write\|MultiEdit | no | 30s | Run cargo test on affected crate |
| `block-generated-files.sh` | PreToolUse | Edit\|Write\|MultiEdit | yes | 5s | Prevent editing Cargo.lock, target/, etc. |
| `pre-commit-gate.sh` | PreToolUse | Bash | yes | 60s | Guard git commit: require fmt + clippy clean |
| `fmt-drift-guard.sh` | Stop | * | no | 15s | Workspace-wide rustfmt check |
| `stop-uncommitted-check.sh` | Stop | * | no | 10s | Warn about uncommitted changes |
| `session-context.sh` | SessionStart | * | no | 5s | Inject current phase, key constraints, dispatch table |

### settings.json (Hook Registration)

```json
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/session-context.sh",
            "timeout": 5
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "Edit|Write|MultiEdit",
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/rust-fmt.sh",
            "timeout": 10
          },
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/clippy-gate.sh",
            "timeout": 30
          },
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/cargo-test-gate.sh",
            "timeout": 30
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "Edit|Write|MultiEdit",
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/block-generated-files.sh"
          }
        ]
      },
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/pre-commit-gate.sh",
            "timeout": 60
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/fmt-drift-guard.sh",
            "timeout": 15
          },
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/stop-uncommitted-check.sh",
            "timeout": 10
          }
        ]
      }
    ]
  }
}
```

### Other Config

| File | Purpose |
| --- | --- |
| `.claude/known-dep-versions.toml` | Pin known-good versions of key dependencies |

## CLAUDE.md Structure

```txt
1. Project identity (name, what it is, one-liner)
2. Quick reference (build, test, lint, run commands)
3. Architecture summary (one-paragraph + crate map)
4. Current phase + status
5. Core principles (Structure > Willpower, etc. — 5 lines)
6. Dispatch table: "When working on X, read docs/architecture/Y.md"
7. Key constraints (the top 5 from XD-001..011, inline)
8. Agent overview: which agents exist and when to use them
9. Testing strategy (proptest, insta, integration tests)
10. Commit conventions
```

## Cargo.toml Workspace

```toml
[workspace]
resolver = "3"
members = [
    "crates/autonomic-core",
    "crates/autonomic-memory",
    "crates/autonomic-evolution",
    "crates/autonomic-session",
    "crates/autonomic-scheduler",
    "crates/autonomic-hooks",
    "crates/autonomic-routing",
    "crates/autonomic-state",
    "crates/autonomic-daemon",
    "crates/autonomic-watchdog",
    "crates/autonomic-cli",
]

[workspace.package]
edition = "2024"
rust-version = "1.94"
license = "LicenseRef-Proprietary"
repository = "https://github.com/Sephyi/autonomic"

[workspace.dependencies]
# Shared dependency versions — crates reference via { workspace = true }
tokio = { version = "1", features = ["full"] }
axum = "0.8"
rusqlite = { version = "0.39", features = ["bundled"] }  # FTS5 included via modern_sqlite in bundled
gix = { version = "0.81", default-features = false, features = ["revision", "worktree-mutation"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
figment = { version = "0.10", features = ["toml", "env"] }  # Config reading (TOML + env vars)
toml = "1"                                                    # TOML serialization (writing state/registry files)
clap = { version = "4.6", features = ["derive"] }
croner = "3"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
thiserror = "2"
ulid = "1"
chrono = { version = "0.4", features = ["serde"] }
tempfile = "3"
fs4 = "0.13"
proptest = "1"
insta = { version = "1", features = ["json"] }

[profile.release]
overflow-checks = true
```

**Binary names** (reviewer fix — no collision):
- `autonomic-daemon` -> binary: `autonomicd`
- `autonomic-watchdog` -> binary: `autonomic-watchdog`
- `autonomic-cli` -> binary: `autonomic`

**Crate stub template** (each crate's `Cargo.toml`):

```toml
[package]
name = "autonomic-core"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
```

Each crate gets `src/lib.rs` (libraries) or `src/main.rs` (binaries: daemon, watchdog, cli).

## rust-toolchain.toml

```toml
[toolchain]
channel = "1.94"
```

Pins to exact version 1.94 (not "stable" which floats). Enforces MSRV.

## clippy.toml

```toml
msrv = "1.94"

disallowed-methods = [
    { path = "std::thread::sleep", reason = "Use tokio::time::sleep — never block the async runtime" },
    { path = "std::process::Command::new", reason = "Use tokio::process::Command for async subprocess management" },
]

disallowed-macros = [
    { path = "std::dbg", reason = "Use tracing::debug! for structured logging" },
    { path = "std::println", reason = "Use tracing::info! for structured logging" },
    { path = "std::eprintln", reason = "Use tracing::warn! or tracing::error!" },
]
```

## deny.toml

```toml
[advisories]
vulnerability = "deny"
unmaintained = "warn"
yanked = "warn"
notice = "warn"

[licenses]
unlicensed = "deny"
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Zlib",
    "Unicode-3.0",
    "Unicode-DFS-2016",
    "OpenSSL",
    "BSL-1.0",
]

[bans]
multiple-versions = "warn"
wildcards = "deny"
```

## known-dep-versions.toml

```toml
# Known-good dependency versions for Autonomic.
# The cargo-dep-auditor agent checks these against Cargo.lock.

[dependencies]
tokio = "1.50"
axum = "0.8.8"
rusqlite = "0.39.0"
gix = "0.81.0"
serde = "1.0"
serde_json = "1.0"
figment = "0.10.19"
toml = "1.1"
clap = "4.6.0"
croner = "3.0.1"
tracing = "0.1"
tracing-subscriber = "0.3"
thiserror = "2.0.18"
ulid = "1.2.1"
chrono = "0.4"
tempfile = "3.27"
fs4 = "0.13.1"
```

## session-context.sh Design

This hook reads the current implementation phase from CLAUDE.md (line matching `**Phase**:` or `**Status**:`) and injects:

```bash
#!/bin/bash
# SessionStart: inject phase status and key constraints

cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0

echo "=== AUTONOMIC SESSION CONTEXT ==="

# Current phase from CLAUDE.md
PHASE=$(grep -m1 "Current Phase" CLAUDE.md 2>/dev/null || echo "Unknown")
echo "Phase: $PHASE"

echo ""
echo "Key Constraints (always active):"
echo "  XD-001: Scheduler MUST spawn Claude via SessionManager, never directly"
echo "  XD-002: All traces go to SQLite via SessionManager, not ad-hoc JSONL"
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
```

## Hook Adaptation from vox-scribe

| vox-scribe | Autonomic | Change |
| --- | --- | --- |
| `rust-fmt.sh` | `rust-fmt.sh` | Same pattern, updated crate names |
| `clippy-gate.sh` | `clippy-gate.sh` | Updated crate-case mapping for 11 crates |
| `cargo-test-gate.sh` | `cargo-test-gate.sh` | Same pattern |
| `spdx-header-check.sh` | REMOVED | Not required |
| `edition-check.sh` | REMOVED | Handled by rust-toolchain.toml |
| `privacy-guard.sh` | REMOVED | No biometric data |
| `async-safety-guard.sh` | REMOVED | Covered by rules/rust.md |
| `dep-freshness.sh` | Incorporated into `cargo-dep-auditor` agent | Agent-based |
| `edit-counter.sh` | REMOVED | Not needed |
| `block-generated-files.sh` | `block-generated-files.sh` | Same pattern |
| `pre-commit-gate.sh` | `pre-commit-gate.sh` | Same pattern |
| `fmt-drift-guard.sh` | `fmt-drift-guard.sh` | Same pattern |
| `stop-uncommitted-check.sh` | `stop-uncommitted-check.sh` | Same pattern |
| `superpowers-check.sh` | `session-context.sh` | Replaced with phase/constraint injection |

## README.md Structure

```txt
# Autonomic

> "You describe the outcome. The system builds, verifies, improves,
> and learns -- without you watching."

Brief description (3-4 sentences).

## Status

Current phase, what's implemented, what's next.

## Architecture

System diagram (text), crate map, link to docs/architecture/overview.md.

## Getting Started

Prerequisites (Rust 1.94, cargo-deny)
Build: cargo build --workspace
Test: cargo test --workspace
Lint: cargo clippy --workspace -- -D warnings

## Documentation

- PRD.md — Product requirements (v0.4)
- docs/architecture/ — Implementation specs (8 files, 226KB)
- docs/research/ — Research paper analyses (9 files, 197KB)
- SOURCES.md — All research sources

## License

License not yet determined. All rights reserved until announced.
```

## License Placeholder

```txt
All Rights Reserved

Copyright (c) 2026 Sephyi <me@sephy.io>

This software is proprietary. No license is granted for use, modification,
or distribution. A formal license will be announced in a future release.

For inquiries, contact: me@sephy.io
```

## Reviewer Fixes Applied

| Issue | Fix |
| --- | --- |
| `git2 = "0.19"` outdated + C dependency | Replaced with `gix = "0.81"` (pure Rust, used in operator's commitbee project, verified features on crates.io) |
| `rusqlite = "0.32"` outdated | Updated to `"0.39"` with `features = ["bundled"]` (FTS5 included via modern_sqlite, no separate feature flag) |
| `croner = "8"` does not exist | Fixed to `"3"` (actual latest) |
| `toml = "0.8"` outdated | Updated to `"1"` (current major). Used for TOML serialization; figment handles reading. |
| `fts5` feature doesn't exist on rusqlite | Removed — `bundled` includes FTS5 via `modern_sqlite` compile flag (Codex review) |
| `Agent` tool name wrong | Removed from agent tool lists — agents use worktree isolation instead of subagent spawning (Codex review) |
| known-dep-versions stale | Updated all versions to crates.io latest as of 2026-03-25 (Codex review) |
| Missing `settings.json` | Full hook registration JSON added |
| Binary name collision (both `autonomic`) | Daemon renamed to `autonomicd` |
| `rust-toolchain.toml` wrong fields | Pin to `channel = "1.94"` (enforces MSRV) |
| Missing `clippy.toml` content | Added MSRV, disallowed-methods, disallowed-macros |
| Missing `deny.toml` content | Added license allowlist, advisory, bans |
| Missing agent tool lists | Added Tools column to agents table |
| Missing crate stub template | Added template with `workspace = true` pattern |
| Missing `session-context.sh` design | Added full pseudocode |
| Missing `[profile.release]` | Added `overflow-checks = true` |
| Missing `known-dep-versions.toml` content | Added all workspace dependencies with versions |
| Missing hook timeouts | Added timeout column to hooks table |
