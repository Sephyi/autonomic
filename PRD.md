<!-- SPDX-FileCopyrightText: 2026 Sephyi <me@sephy.io> -->
<!-- SPDX-License-Identifier: LicenseRef-Proprietary -->

# Autonomic -- Product Requirements Document

**Version**: v0.6
**Date**: 2026-03-26
**Status**: In Development (Phase 1 — Foundation)
**Author**: [Sephyi](https://github.com/Sephyi) + [Claude Opus 4.6](https://www.anthropic.com/news/claude-opus-4-6) + [Gemini 3 Pro](https://deepmind.google/technologies/gemini/) + [Codex gpt-5.4](https://openai.com/index/codex/)  
**Edition**: Rust 2024 | **MSRV**: 1.94 | **Toolchain**: stable  
**License**: LicenseRef-Proprietary | **REUSE compliant**  
**Platform**: macOS 14+ (primary), Linux (secondary)  

<details>
<summary>Changelog</summary>

| Version | Date | Summary |
| --- | --- | --- |
| 0.6 | 2026-03-26 | Phase 1 implementation progress: 7 of 13 crates implemented (autonomic-core, autonomic-db, autonomic-state, autonomic-container, autonomic-session, autonomic-daemon, autonomic-watchdog). 86 tests passing. FR-001 partially complete (daemon + watchdog binaries, HTTP API, tracing, PID, graceful shutdown, crash rollback — missing launchd plists and frozen-file enforcement). FR-003 partially complete (session spawning, stream-json parsing, cost tracking, timeout, env filtering — missing worktree isolation and resume re-injection). FR-004 partially complete (git init, atomic commits — missing CLI commands and daily snapshot). Milestones tagged: `milestone/phase-1a`, `milestone/phase-1b`. |
| 0.5 | 2026-03-26 | Container architecture: Podman/Docker sandboxing for agent sessions. PostgreSQL (in container) replaces SQLite for shared state. sqlx (compile-time checked queries, multi-backend) replaces rusqlite. pgvector for optional semantic search. K8s-like container scheduling (ephemeral agents, warm pool, resource limits, network isolation). New crates: autonomic-db (sqlx migrations/queries), autonomic-container (Podman/Docker abstraction). compose.yaml + Containerfile added. 13 crates total (was 11). |
| 0.4 | 2026-03-25 | Codex (gpt-5.4) review: 5 critical issues, 5 design concerns, 7 missing elements, 11 specific corrections. Fixed: Laplace math error, missing `start` variable, orphan process handling, secrets management. Added §7 Cross-Document Implementation Constraints (11 items). Clarified runtime deps. Per-project active variants. |
| 0.3 | 2026-03-25 | Second research wave: ARTEMIS, TT-SI, MAS design patterns, Agent Skills Standard, ClaudeClaw, Clawith, Rust self-evolving agent, MARIA OS SEAA, MAS Orchestration Survey, Agyn, Awesome AI Agents 2026 (9 additional research docs, 197KB total). Key additions: daemon+watchdog two-process architecture, filesystem-level modification frontier, 6 trigger types beyond cron, SKILL.md as capability format, ARTEMIS config formalization, uncertainty-guided selective adaptation, CooperBench agent collaboration warning, system prompt re-injection on --resume constraint. 22 files, 560KB total documentation. |
| 0.2 | 2026-03-25 | Major revision after full paper analysis (SICA, AgentDevel, GEA, DGM, DGM-Hyperagent, MARIA OS, Godel Agent — all read as full HTML from arxiv). Evolution engine redesigned: linear sidecar replaced by archive-based group evolution (GEA), flip-centered gating (AgentDevel), Lyapunov convergence bounds (MARIA OS), timed verification windows, implementation-blind analysis, cross-project evolution as primary mechanism. Architecture docs created (7 files, 218KB). Research docs created (5 files, 108KB). |
| 0.1 | 2026-03-25 | Initial PRD from comprehensive research: 11 parallel research agents analyzed Instar, OpenClaw (334K stars), Gas Town (12.8K stars), Claude Agent SDK, Google A2A protocol (v1.0), 10+ research papers on self-improving agents. User setup audit: 12+ existing hooks, 4 custom agents, multi-model routing, Mem0, Qdrant. |

</details>

> **Implementation reference**: This PRD is the what and why. The how is in `docs/architecture/*.md` (8 files, 218KB of implementation-grade specifications with Rust types, SQL schemas, algorithms, and exact CLI flags). Research foundation is in `docs/research/*.md` (9 files, 197KB of full paper extractions). Sources are in `SOURCES.md` (22 files, 560KB total corpus).

## 1. Vision

> **"You describe the outcome. The system builds, verifies, improves, and learns -- without you watching."**

Autonomic is a persistent, self-improving orchestrator for autonomous AI-assisted software development. It runs as a native Rust daemon on macOS, managing multiple Claude Code instances across development projects. It automatically routes tasks to optimal model tiers, enforces quality through structural hooks, accumulates and retrieves knowledge across sessions, evolves its own configuration based on measured outcomes, and coordinates external AI agents (Codex, Gemini) for verification and diverse perspectives.

Autonomic replaces the human as the orchestration layer. The developer defines goals. Autonomic breaks them down, delegates, verifies, learns, and improves.

### 1.1 Core Principles

1. **Structure > Willpower** -- Every behavior that matters is enforced by hooks, gates, or validation ratchets -- never by instructions alone. A 10-line hook is a guarantee. A 1,000-line prompt is a wish. (Instar; validated by Claude Code issues #19635, #32161, #34197, #35309)
2. **Archive-Based Evolution** -- Self-improvement maintains an append-only archive of configuration variants, selecting by performance + novelty, never pruning. Linear improvement misses alternative paths. (GEA: SWE-bench 20% -> 71% in 30 iterations vs DGM: 50% in 60 iterations with linear approach)
3. **Gate on Regressions** -- Every proposed change is evaluated via flip-centered gating (P2P/P2F/F2P/F2F classification). Aggregate improvement is insufficient; regression rate is the true safety metric. (AgentDevel: 3.1% bad releases with gating vs 14.8% without)
4. **Bounded Self-Modification** -- A Lyapunov energy function must strictly decrease per modification. Convergence is provable: N_max = floor(V(M_0) / epsilon), typically 50-200 modifications. The modification frontier is frozen — the system cannot expand its own capabilities. (MARIA OS)
5. **Verifiable Outcomes Only** -- Self-improvement converges only when outcomes are verifiable (tests pass, metrics improve). Scaffold improvements help scaffold problems but cannot substitute for model capability. (SICA: zero gain on reasoning tasks)
6. **Eventual Consistency** -- Agents fail. Sessions crash. Context compacts. The architecture treats failure as expected and designs for aggregate reliability from individually unreliable components plus validation ratchets. (Gas Town)
7. **Graceful Degradation** -- External models (Codex, Gemini) are recommended for verification but never required. The system is fully functional with Claude alone.
8. **Point-in-Time Recovery** -- All state is git-backed. Every mutation is an atomic commit. Any historical state is recoverable. Major milestones are tagged. Failed evolution variants are kept as data points, never deleted.
9. **Rust-Native** -- Sub-millisecond startup, <10MB RSS, crash-safe, launchd-supervised. No runtime dependencies for the daemon binary itself. Hooks require managed system dependencies (bash, python3 for hook scripts; language-specific formatters/linters per project).

### 1.2 What This Is Not

- Not a general-purpose AI assistant (OpenClaw covers that)
- Not a multi-channel messaging platform (no Telegram/WhatsApp/Discord)
- Not a cloud service -- runs locally, all state on disk
- Not model-specific -- orchestrates Claude Code but coordinates with any model via CLI
- Not a replacement for Claude Code -- it orchestrates Claude Code instances

### 1.3 Compatibility Policy

| Version Range | Commitment |
| --- | --- |
| 0.x.y | No stability guarantees. Breaking changes in minor versions. |
| 1.x.y | SemVer. Config format stable. State schema stable with migrations. CLI flags stable. |

## 2. Competitive Landscape

### 2.1 Market Position

| Category | Key Players | Autonomic Advantage |
| --- | --- | --- |
| General AI assistants | OpenClaw (334K stars), ClaudeClaw (592 stars) | Not competing -- development orchestrator, not messaging assistant |
| Multi-agent orchestrators | Gas Town (12.8K stars, Go), Kelos (86 stars, K8s) | Rust-native, self-improving, solo-dev focused |
| Claude Code infrastructure | Instar (6 stars), Claude Agent Teams (experimental) | Extracts Instar's best concepts in Rust without overhead |
| Self-improving agents | SICA, DGM, GEA, AgentDevel (research only) | First practical implementation of archive-based group evolution for a solo dev's daily workflow |

### 2.2 Unique Differentiators

1. **Archive-based group evolution** -- Maintains diverse configuration variants, selects by performance + novelty, cross-pollinates across projects. First practical application of GEA/DGM research to development tooling.
2. **Flip-centered gating** -- Every change is evaluated for per-scenario regressions (P2F), not just aggregate improvement. From AgentDevel: reduces bad releases from 14.8% to 3.1%.
3. **Formally bounded** -- Lyapunov energy function guarantees convergence. Modification frontier prevents scope creep. From MARIA OS.
4. **Rust-native daemon** -- Sub-ms startup, ~5MB RSS, crash-safe, launchd-supervised.
5. **Git-backed state with full history** -- Every mutation is atomic. Roll back to any point. Diff changes. Failed variants are data, not waste.
6. **Empirical model routing** -- Builds a performance matrix from measured outcomes, overriding heuristics when data is sufficient.
7. **Containerized memory** -- PostgreSQL with tsvector + pgvector replaces Mem0/Qdrant/Ollama. Compile-time checked queries via sqlx. Runs alongside agents in Podman/Docker.

### 2.3 Key Inspirations

| Source | What We Take | What We Leave | Detailed Analysis |
| --- | --- | --- | --- |
| **Instar** | Structure > Willpower, compaction recovery, sidecar pattern, graduated governance, rubber-stamp detection | 102KB template, Relationship Manager, Threadline, Telegram, `--dangerously-skip-permissions` | `instar-analysis.md` |
| **Gas Town** | Git-backed state, validation ratchets, ephemeral agents + persistent state, eventual consistency | Go, 20-50 agent scale, Mayor hierarchy | N/A |
| **OpenClaw** | Hybrid memory search, auto-capture decisions, hot-reloadable config | 70+ channels, messaging-first | N/A |
| **GEA** | Archive-based group evolution, performance+novelty selection, cross-project rounds, framework-level transfer | Research benchmark focus | `docs/research/gea-group-evolving-agents.md` |
| **AgentDevel** | Flip-centered gating, implementation-blind analysis, regression-aware pipeline | Academic framing | `docs/research/agentdevel-release-engineering.md` |
| **MARIA OS** | Lyapunov convergence, modification frontier, timed verification, energy function, 5-stage pipeline | Full formal framework | `docs/research/maria-os-godel-agent.md` |
| **DGM** | Append-only archive, exploration pressure (inverse child count), never-prune | Research benchmark focus | `docs/research/dgm-darwin-godel-machine.md` |
| **SICA** | Utility function, self-improvement scope limits, overseer pattern | Docker-based self-editing | `docs/research/sica-self-improving-coding-agent.md` |
| **ARTEMIS** | Config space formalization C=(P,T,M,Theta), semantic GA, hierarchical evaluation | No-code platform focus | `docs/research/artemis-and-evolution-surveys.md` |
| **TT-SI** | Uncertainty-guided selective adaptation (+5.48% vs +1.04% uniform) | LoRA fine-tuning focus | `docs/research/artemis-and-evolution-surveys.md` |
| **ClaudeClaw** | System prompt re-injection constraint, rate-limit regex detection, serial execution | Telegram/Discord focus | `docs/research/agent-skills-and-claude-wrappers.md` |
| **Clawith** | 6 trigger types, HEARTBEAT behavior template, Plaza knowledge sharing | Python, heavyweight | `docs/research/agent-skills-and-claude-wrappers.md` |
| **Agent Skills** | SKILL.md format (20+ platforms), progressive disclosure, YAML frontmatter | N/A | `docs/research/agent-skills-and-claude-wrappers.md` |
| **SEAA** | Tool synthesis pipeline, Capability Monotonicity Theorem, gap detection | Full framework scope | `docs/research/rust-evolution-and-workspace-patterns.md` |
| **Rust self-evolving** | Two-process architecture, forbidden zones at FS level, SQLite failure journal | Article scope | `docs/research/rust-evolution-and-workspace-patterns.md` |
| **CooperBench** | Agents 50% worse collaborating — orchestrator-mediated only | Warning, not pattern | `docs/research/mas-orchestration-and-patterns.md` |

## 3. Architecture

> **Full specification**: `docs/architecture/overview.md` (14KB) contains the system diagram, crate map, dependency graph, data flow, design decisions table, and failure modes. What follows is a summary.

### 3.1 System Overview

```txt
HOST (macOS / Linux)
  |
  +-- launchd (auto-start, KeepAlive)
  |     +-- autonomic-watchdog (~1MB, health monitor, crash rollback)
  |     +-- autonomicd (orchestrator, Rust, axum, ~5MB RSS)
  |           +-- Decision Engine (Opus 4.6 1M, on-demand)
  |           +-- Evolution Engine (archive-based, flip-gated, bounded)
  |           +-- Container Scheduler (K8s-like: pool, scaling, limits)
  |           +-- Hook Manager (compaction recovery, command guard, metrics)
  |           +-- External Coordinator (Codex, Gemini — optional)
  |
  +-- podman/docker compose (container infrastructure)
        +-- postgres:17 (always-on, persistent volume)
        |     Shared state, tsvector FTS, pgvector semantic, LISTEN/NOTIFY
        +-- agent containers (ephemeral, on-demand)
              Project dir mounted (rw), .claude/ config mounted (ro)
              Network: postgres + orchestrator only (no internet default)
              Resource limits: CPU/memory per container
```

**Two-process architecture**: Watchdog monitors daemon, auto-reverts to last known-good git tag on crash after evolution deployment.

**Container sandboxing** (from SICA, MARIA OS SEAA, OpenClaw): Agent sessions run in ephemeral Podman/Docker containers. The orchestrator manages lifecycle like a lightweight K8s scheduler — spin up on demand, tear down on completion, scale based on task queue depth and rate budget. Docker supported as alternative runtime via `ContainerRuntime` trait.

### 3.2 Workspace Structure

```txt
crates/
+-- autonomic-core/            # Types, traits, config (figment), errors (thiserror)
+-- autonomic-db/              # sqlx migrations, schemas, compile-time checked queries
+-- autonomic-container/       # Podman/Docker abstraction (ContainerRuntime trait)
+-- autonomic-memory/          # Memory store (tsvector FTS + pgvector semantic)
+-- autonomic-evolution/       # Archive, variant selection, flip gating, cross-project
+-- autonomic-session/         # Container lifecycle + Claude Code management
+-- autonomic-scheduler/       # Cron + 6 trigger types, warm pool, rate budget
+-- autonomic-hooks/           # Hook management (STANDALONE-CAPABLE crate)
+-- autonomic-routing/         # Task classification, performance matrix
+-- autonomic-state/           # Git-backed config state + container volume management
+-- autonomic-daemon/          # Main binary (autonomicd): axum, launchd
+-- autonomic-watchdog/        # Lightweight monitor: health check, crash rollback
+-- autonomic-cli/             # CLI (autonomic): status, evolve, memory, project, rollback
```

`autonomic-hooks` is deliberately standalone — usable without the full orchestrator. `autonomic-container` abstracts Podman/Docker via the `ContainerRuntime` trait — OCI-compatible images work with either runtime.

> **Full specification**: `docs/architecture/overview.md` §3 Crate Map, §3.1 Dependency Graph

### 3.3 Evolution Engine

The self-improvement system is the core differentiator. It is **not** a linear propose-test-deploy cycle. It is an **archive-based group evolution system** with formal convergence guarantees.

> **Full specification**: `docs/architecture/evolution-engine.md` (25KB, 586 lines)

**Variant Archive** (from GEA + DGM): Append-only collection of configuration variants. Each variant is a complete ConfigSurface (rules, hooks, routing, memory config, scheduler config). Never pruned — failed variants are data points for future selection. Selection uses `alpha * performance + (1-alpha) * novelty + 0.1 * exploration_bonus` via softmax (DGM's inverse child count for exploration pressure).

**Modification Frontier** (from MARIA OS + Rust self-evolving agent): Frozen boundary between what the evolution engine CAN and CANNOT touch. The frontier itself, safety predicates, core evolution logic, and the energy function are immutable. Everything else (hooks, rules, routing, memory config, scheduler) is fair game within bounds. **Filesystem-level enforcement**: frozen files are set read-only (`chmod 444`) and the watchdog verifies permissions on startup. Forbidden zones are not just prompt instructions — they are OS-enforced.

**Evolution Cycle**:

```txt
1. DISCOVER — Post-session hooks collect experience traces to shared pool
2. ANALYZE — Implementation-blind (AgentDevel): receives only traces + scores, never config content
3. PROPOSE — Separate repair step generates ConfigChanges from diagnostics
4. TEST — Flip-centered gating (AgentDevel):
     Classify each probe: P2P (stable pass), P2F (REGRESSION), F2P (improvement), F2F (stable fail)
     Gate: rho_P2F = |P2F| / (|passes| + epsilon) must be < 2%
     Plus: intent alignment, no critical probes in P2F, energy decrease check
5. DEPLOY — Timed verification (MARIA OS): 1h -> 24h -> 72h windows with auto-rollback
6. CROSS-PROJECT — Group evolution round (GEA): aggregate traces, reflect, propagate directives
```

**Energy Function and Convergence** (from MARIA OS):

```txt
V(M) = w_regression * L_regression + w_hook_overhead * L_hook_overhead
     + w_unused_rules * L_unused_rules + w_error_rate * L_error_rate
     + w_routing_waste * L_routing_waste + w_memory_staleness * L_memory_staleness

Each accepted modification: V(M_{t+1}) < V(M_t) - epsilon
Upper bound: N_max = floor(V(M_0) / epsilon)  ~50-200 modifications to convergence
```

**Governance** (from Instar): Four profiles (cautious / supervised / collaborative / autonomous). Rubber-stamp detection: 10+ fast approvals suggests governance upgrade. Two-phase operation (from GEA): Evolution Mode (R&D, higher cost, multiple concurrent evaluations) and Production Mode (single deployed variant, standard cost).

**Config Space Formalization** (from ARTEMIS): The configuration surface is formalized as `C = (P, T, M, Theta)` — Prompts (CLAUDE.md rules, system prompts), Tools (hooks, scripts), Memory (decay rates, assembly config), and Theta (numeric parameters: token budgets, thresholds, intervals). Evolution uses semantic GA with LLM-ensemble mutation/crossover for NL components (P), and standard optimization for numeric components (Theta). Hierarchical evaluation: cheap LLM scoring before expensive probe battery runs.

**Uncertainty-Guided Selective Adaptation** (from TT-SI): The analysis phase does not propose changes uniformly. It detects which task categories have the highest failure rates (via Relative Softmax Scoring over probe results) and focuses evolution proposals there. TT-SI showed +5.48% improvement with selective adaptation vs +1.04% with uniform — a 5x difference in evolution efficiency.

**Scope Limits** (from SICA): The evolution engine targets scaffold-shaped problems (hooks, rules, routing, memory, scheduling). It explicitly does NOT target model-capability-shaped problems (reasoning, architecture quality, planning). Model limitations create capability gap entries, not evolution proposals.

### 3.4 Memory System

> **Full specification**: `docs/architecture/memory-system.md` (16KB, 426 lines)

PostgreSQL with tsvector (full-text search) + pgvector (semantic search) replaces Mem0/Qdrant/Ollama. Runs in a Podman/Docker container alongside the orchestrator. sqlx for compile-time checked queries with multi-backend support (Postgres primary, SQLite fallback for users without containers).

**Entry types**: Decision (decay 0.01), Pattern (0.03), Gotcha (0.02), Preference (0.01), Tool (0.05), Error (0.08), Lesson (0.10), Ephemeral (0.20).

**Context assembly**: FTS5 keyword search -> scope filter -> score (`FTS_rank * usefulness_laplace * exp(-decay * days) * category_boost`, where `usefulness_laplace = (helpful+1)/(helpful+misleading+2)`) -> token-budgeted packing with primacy/recency ordering -> inject via SessionStart hook.

**MEMORY.md sync**: Bidirectional. Orchestrator generates per-project MEMORY.md from store. Claude Code auto-memory writes ingested back. PostgreSQL is source of truth.

### 3.5 Session Management

> **Full specification**: `docs/architecture/session-management.md` (43KB, 1,280 lines)

Agent sessions run in **ephemeral Podman/Docker containers**. The orchestrator spawns containers with the project directory bind-mounted (rw), `.claude/` config mounted (ro), and Postgres connection injected via env. Claude Code runs inside the container with `--print --output-format stream-json`. Container lifecycle: create -> run -> capture output -> destroy. The orchestrator manages a **warm container pool** (pre-started base images for instant task assignment) and enforces **resource limits** (CPU/memory per container) and **network isolation** (containers reach Postgres + orchestrator API only, no internet by default).

**Critical constraints**: System prompt does NOT persist across `--resume` (ClaudeClaw finding). `CLAUDECODE=1` filtered from env. **CooperBench warning**: agents achieve ~50% lower success collaborating vs solo — Agent Teams only for truly parallelizable work.

### 3.6 Model Routing

> **Full specification**: `docs/architecture/model-routing.md` (14KB, 407 lines)

Heuristic layer (day 1) + empirical layer (after 20 samples per model-task pair). External models (Codex, Gemini) recommended for verification, fallback to Opus self-review. Performance matrix built from measured outcomes, overrides heuristics when data is sufficient.

### 3.7 Scheduling and Rate Budget

> **Full specification**: `docs/architecture/scheduler.md` (38KB, 1,291 lines)

Six trigger types (adapted from Clawith's Aware system): `cron` (recurring schedule via croner), `once` (one-time future execution), `interval` (every N minutes/hours), `poll` (check condition, act if true), `on_message` (react to incoming request), `webhook` (external HTTP trigger). Priority levels (critical/normal/low). Model tier per job. Rate budget: 15% orchestrator, 55% active work, 15% scheduled, 10% external, 5% emergency reserve. Adaptive: degrade gracefully when budget is tight.

### 3.8 Hook System

> **Full specification**: `docs/architecture/hook-system.md` (46KB, 1,404 lines)

Global hooks: compaction-recovery (inject identity + memory after context compression), session-start-context (assemble and inject relevant memories), session-metrics (collect experience traces). Project hooks: format-on-edit, lint-on-edit, command-guard. Evolution hooks: dynamically proposed from error patterns.

### 3.9 State Management

> **Full specification**: `docs/architecture/state-management.md` (28KB, 957 lines)

Git-backed `~/.autonomic/`. Auto-commit every mutation. Tags for evolution events, milestones, daily snapshots. `autonomic rollback --to <tag>` restores any state. PostgreSQL for shared state. Single-writer principle for git operations.

## 4. Feature Requirements

### 4.1 Phase 1: Foundation -- v0.1.0 `IN PROGRESS`

**Target**: Persistent daemon with project registry, session management, and basic metrics.

#### FR-001: Daemon Process with Watchdog

**Priority**: P0 | **Phase**: 1

Two-process architecture: daemon (main binary) + watchdog (lightweight monitor). Both managed by launchd. Watchdog prevents self-bricking from evolution failures.

**Acceptance Criteria**:
- [ ] Daemon binary starts via launchd Launch Agent plist
- [ ] Watchdog binary starts via separate launchd plist, monitors daemon health
- [x] Watchdog auto-reverts to last known-good git tag if daemon crashes after evolution deployment
- [ ] Daemon auto-restarts within 10s of crash (KeepAlive)
- [x] Structured tracing to `~/.autonomic/logs/daemon.log`
- [x] HTTP API on localhost (axum) for CLI communication
- [x] PID file at `~/.autonomic/daemon.pid`
- [x] Graceful shutdown on SIGTERM (drain active sessions)
- [ ] Frozen files enforced at filesystem level (`chmod 444`) — watchdog verifies on startup

#### FR-002: Project Registry

**Priority**: P0 | **Phase**: 1

TOML-based project registration with auto-discovery.

**Acceptance Criteria**:
- [ ] `autonomic project add <path>` registers a project
- [ ] `autonomic project list` shows all projects with status
- [ ] Auto-discovers `.claude/` directories under configured roots
- [ ] Stores: path, language, last activity, last session cost, hook inventory

#### FR-003: Session Management

**Priority**: P0 | **Phase**: 1

Spawn Claude Code as subprocess, parse stream-json output, track completion and cost.

**Acceptance Criteria**:
- [x] Spawn `claude -p <prompt> --output-format stream-json` as subprocess
- [x] Parse typed messages from NDJSON stdout
- [x] Detect completion (`result` message with `success` subtype)
- [x] Track `total_cost_usd` per session
- [x] Enforce configurable timeout (default 30min)
- [x] Filter `CLAUDECODE=1` from subprocess environment
- [ ] Worktree isolation via `git worktree` for parallel sessions
- [ ] Re-inject system prompt on every `--resume` call (it does NOT persist across resumes — ClaudeClaw finding)

#### FR-004: Git-Backed State

**Priority**: P0 | **Phase**: 1

All state in `~/.autonomic/` managed as a git repository with auto-commit on every mutation.

**Acceptance Criteria**:
- [x] `git init` on first run (via gix)
- [x] Auto-commit on every state mutation (atomic: write tmp + rename + git add + commit)
- [ ] `autonomic snapshot <label>` creates tagged snapshot
- [ ] `autonomic rollback --to <tag-or-commit>` restores state
- [ ] Daily auto-snapshot via internal scheduler

#### FR-005: Basic Metrics Collection

**Priority**: P0 | **Phase**: 1

SessionStart and Stop hooks that collect structured experience traces.

**Acceptance Criteria**:
- [ ] Stop hook logs: duration, tool calls, errors, model used, cost, compaction count
- [ ] Traces stored in PostgreSQL (`experience_traces` table)
- [ ] `autonomic metrics show` displays recent session summaries
- [ ] `autonomic metrics export --format csv` for external analysis

### 4.2 Phase 2: Memory and Hooks -- v0.2.0

**Target**: Memory store with assembly, compaction recovery, and command guard.

#### FR-006: Memory Store

**Priority**: P0 | **Phase**: 2

PostgreSQL + tsvector with typed entries, scoped queries, decay-on-read, and usefulness tracking.

**Acceptance Criteria**:
- [ ] CRUD for memory entries with all schema fields (8 categories, 3 scopes)
- [ ] tsvector keyword search < 1ms for 10K entries
- [ ] Scope filtering: global, project, language
- [ ] Decay applied at query time (not on write)
- [ ] `mark_helpful(id)` / `mark_misleading(id)` for usefulness

#### FR-007: Context Assembly

**Priority**: P0 | **Phase**: 2

Token-budgeted assembly injected via SessionStart hook.

**Acceptance Criteria**:
- [ ] tsvector search with prompt keywords (OR semantics)
- [ ] Score: `ts_rank * usefulness * exp(-decay * days) * category_boost`
- [ ] Token budget (default 4K) with tiered rendering
- [ ] Primacy/recency ordering (decisions at start, patterns at end)
- [ ] Inject via hook stdout, compatible with Claude Code

#### FR-008: MEMORY.md Bidirectional Sync

**Priority**: P1 | **Phase**: 2

Two-way sync between PostgreSQL store and Claude Code's native MEMORY.md.

**Acceptance Criteria**:
- [ ] Generate per-project MEMORY.md from store (filtered by scope)
- [ ] Ingest Claude Code auto-memory additions into store
- [ ] PostgreSQL is source of truth; MEMORY.md is a projection
- [ ] Sync on session start and session end

#### FR-009: Compaction Recovery Hook

**Priority**: P0 | **Phase**: 2

Inject identity, memory, and task context directly after context compaction.

**Acceptance Criteria**:
- [ ] Fires on SessionStart with `compact` matcher
- [ ] Injects actual content (not pointers — "The 164th Lesson")
- [ ] Injects: operator identity, top-N relevant memories, current task context
- [ ] Total injection under 6K tokens

#### FR-010: Command Guard Hook

**Priority**: P0 | **Phase**: 2

PreToolUse hook blocking destructive commands before execution.

**Acceptance Criteria**:
- [ ] Blocks: `rm -rf`, force push, `DROP TABLE`, `prisma migrate reset`
- [ ] Always-blocked list for catastrophic ops (cannot be overridden)
- [ ] Two levels: L1 (block + ask) and L2 (self-verify prompt)
- [ ] Configurable per project

### 4.3 Phase 3: Scheduling -- v0.3.0

**Target**: Cron-based tasks with rate budget awareness.

#### FR-011: Job Scheduler with 6 Trigger Types

**Priority**: P0 | **Phase**: 3

TOML-defined jobs with multiple trigger types, model tier, and priority.

**Acceptance Criteria**:
- [ ] 6 trigger types: `cron` (croner), `once` (future timestamp), `interval` (every N duration), `poll` (condition check), `on_message` (incoming request), `webhook` (external HTTP)
- [ ] Model tier per job (Haiku/Sonnet/Opus)
- [ ] Priority: critical (always), normal (defer at 80% budget), low (defer at 60%)
- [ ] `autonomic schedule list` with next run times
- [ ] Hot-reloadable: config changes restart scheduler without daemon restart (from OpenClaw)

#### FR-012: Rate Budget Tracker

**Priority**: P0 | **Phase**: 3

Track and enforce token budget across all sessions.

**Acceptance Criteria**:
- [x] Track `total_cost_usd` per session
- [x] 5-hour sliding window aggregation
- [ ] Budget allocation enforcement (15/55/15/10/5 split)
- [ ] Adaptive degradation at 80% utilization
- [ ] `autonomic budget show`

#### FR-013: Built-in Jobs

**Priority**: P1 | **Phase**: 3

Default scheduled tasks.

**Acceptance Criteria**:
- [ ] Daily health check: all projects compile + test (Haiku tier)
- [ ] Daily reflection: review experience traces, auto-capture learnings (Haiku)
- [ ] Daily snapshot: tag git state
- [ ] Weekly evolution analysis: metric patterns (Haiku)

### 4.4 Phase 4: Evolution Engine -- v0.4.0

**Target**: Archive-based self-improvement with flip-centered gating and convergence bounds.

#### FR-014: Variant Archive

**Priority**: P0 | **Phase**: 4

Append-only archive of configuration variants.

**Acceptance Criteria**:
- [ ] `variants` table with full schema (id, parent, ancestors, surface, status)
- [ ] Append-only — variants never deleted or pruned
- [ ] Parent selection: softmax over `alpha * performance + (1-alpha) * novelty + 0.1 * exploration_bonus`
- [ ] Capability vectors (binary, per probe task) for novelty via KNN distance
- [ ] Seed variant created from operator's existing configuration on first run

#### FR-015: Probe Task Battery

**Priority**: P0 | **Phase**: 4

Standardized test scenarios for evaluating configuration variants.

**Acceptance Criteria**:
- [ ] 10-30 probe tasks per project (compilation, tests, hook timing, past failure scenarios)
- [ ] Probe tasks stored in `probe_tasks` table with `is_critical` flag
- [ ] Results stored per variant per probe in `probe_results` table
- [ ] Probes added when new failure modes discovered; never removed (only retired)

#### FR-016: Flip-Centered Gating

**Priority**: P0 | **Phase**: 4

Per-scenario regression detection before deployment.

**Acceptance Criteria**:
- [ ] Classify each probe: P2P, P2F (regression), F2P (improvement), F2F
- [ ] Gate: `rho_P2F = |P2F| / (|passes| + epsilon)` must be < threshold (default 2%)
- [ ] Intent alignment: F2P improvements correspond to proposal rationale
- [ ] No critical probes in P2F set (zero-regression on critical probes)
- [ ] Energy check: `V(proposed) < V(current) - epsilon`
- [ ] `flip_reports` table with full classification data

#### FR-017: Implementation-Blind Analysis with Uncertainty Focus

**Priority**: P0 | **Phase**: 4

Metric analysis that receives only traces and scores, never config content. Focuses on high-failure task categories.

**Acceptance Criteria**:
- [ ] Analysis session receives: experience traces, probe results, rubric
- [ ] Analysis session does NOT receive: config content, hook source code, rule text
- [ ] Output: diagnostics with severity (pattern type + affected metric)
- [ ] Separate repair step generates ConfigChanges from diagnostics
- [ ] Uncertainty-guided focus (TT-SI): detect task categories with highest failure rates, prioritize proposals there (+5.48% vs +1.04% for uniform adaptation)
- [ ] Cost per analysis: < $0.05 (Haiku tier)

#### FR-018: Timed Verification Deployment

**Priority**: P0 | **Phase**: 4

Staged deployment with auto-rollback at each verification window.

**Acceptance Criteria**:
- [ ] Window 1 (1h): run probe battery in production context
- [ ] Window 2 (24h): aggregate session traces, compare to 7-day baseline
- [ ] Window 3 (72h): check targeted metric improvement, no regressions > 3%
- [ ] Auto-rollback at any failing window
- [ ] Git tag on each stage: `evo-{id}-testing`, `evo-{id}-deployed`, `evo-{id}-rollback-{window}`

#### FR-019: Energy Function and Convergence

**Priority**: P1 | **Phase**: 4

Lyapunov energy tracking to guarantee convergence.

**Acceptance Criteria**:
- [ ] Energy function: weighted sum of loss metrics (regression, hook overhead, unused rules, error rate, routing waste, memory staleness)
- [ ] Each accepted modification must decrease energy by >= epsilon
- [ ] `energy_history` table tracks energy per variant
- [ ] System enters maintenance mode after convergence (monitor for drift only)
- [ ] Drift detection: energy increase > 10% triggers return to evolution mode

#### FR-020: Learning Registry

**Priority**: P1 | **Phase**: 4

Structured insights from evolution outcomes.

**Acceptance Criteria**:
- [ ] Entries (LRN-001...) with category, source proposal, applied status
- [ ] Failed proposals create learning entries with root cause
- [ ] Before proposing, engine searches registry for prior attempts
- [ ] `autonomic learn list` shows recent learnings

### 4.5 Phase 5: Cross-Project Evolution and Multi-Model -- v0.5.0

**Target**: Group evolution across projects. Automated external model verification.

#### FR-021: Cross-Project Evolution Rounds

**Priority**: P0 | **Phase**: 5

Group evolution (GEA pattern): projects as group members, shared experience pool, reflection, cross-pollination.

**Acceptance Criteria**:
- [ ] Aggregate experience traces across all projects into shared pool
- [ ] Reflection session: generate evolution directives from shared patterns
- [ ] Each project generates its own patches from shared directives
- [ ] Track ancestor breadth (how many source projects contributed to a variant)
- [ ] Framework-level improvements (hook patterns, rule structures) propagate across projects

#### FR-022: External Model Dispatch

**Priority**: P1 | **Phase**: 5

CLI dispatch to Codex and Gemini for verification.

**Acceptance Criteria**:
- [ ] Dispatch to Codex: `/opt/homebrew/bin/codex exec "<prompt>"`
- [ ] Dispatch to Gemini: `/opt/homebrew/bin/gemini -p "<prompt>"`
- [ ] Timeout enforcement (30s default)
- [ ] Graceful fallback to Opus self-review when unavailable
- [ ] Verification at plan and implementation phases (recommended, not required)

#### FR-023: Model Performance Learning

**Priority**: P1 | **Phase**: 5

Empirical model-performance matrix.

**Acceptance Criteria**:
- [ ] Record: model, task type, outcome, cost, time per session
- [ ] Build `model_performance[model][task_type]` summary view
- [ ] Override heuristic routing at 20+ samples per pair
- [ ] `autonomic models show` displays performance matrix

### 4.6 Phase 6: Governance and Advanced -- v0.6.0

**Target**: Graduated governance, A/B testing, capability gap tracking.

#### FR-024: Graduated Governance

**Priority**: P1 | **Phase**: 6

Four profiles with rubber-stamp detection.

**Acceptance Criteria**:
- [ ] Profiles: cautious, supervised, collaborative, autonomous
- [ ] `autonomic governance set <profile>`
- [ ] Rubber-stamp detection: 10+ fast approvals triggers upgrade suggestion
- [ ] Acceptance rate tracking over rolling 20-proposal window
- [ ] Two-phase operation: Evolution Mode (R&D) and Production Mode

#### FR-025: A/B Testing Framework

**Priority**: P2 | **Phase**: 6

Test alternative configurations against each other.

**Acceptance Criteria**:
- [ ] Paired worktrees: control vs experimental
- [ ] Same probe battery in both
- [ ] Statistical comparison (confidence intervals, from SICA)
- [ ] Auto-adopt winner if statistically significant

#### FR-026: Capability Gap Tracker

**Priority**: P2 | **Phase**: 6

Track what the system cannot do.

**Acceptance Criteria**:
- [ ] Gap entries (GAP-001...) with severity, category, context
- [ ] Model-capability gaps marked as `model_bound` (not evolution targets)
- [ ] Gaps linked to evolution proposals that address them
- [ ] `autonomic gaps list`

## 5. Non-Functional Requirements

### 5.1 Performance

| Metric | Target |
| --- | --- |
| Daemon startup | < 100ms |
| RSS (idle) | < 10MB |
| RSS (under load) | < 50MB |
| FTS5 query (10K entries) | < 1ms |
| Context assembly | < 50ms |
| Git commit | < 200ms |
| Session spawn to first output | < 500ms |

### 5.2 Reliability

- launchd KeepAlive auto-restart
- PostgreSQL MVCC (concurrent read/write across agent containers)
- Atomic mutations (tmp + rename + git commit)
- Graceful SIGTERM shutdown (drain sessions)
- No data loss on crash (WAL + git)

### 5.3 Observability

- Structured JSON logging (tracing + tracing-subscriber)
- Per-session output capture (`~/.autonomic/logs/sessions/`)
- `autonomic status` dashboard
- `autonomic evolve history` evolution timeline
- `autonomic budget show` rate budget utilization

## 6. Technology Stack

| Component | Crate / Tool | Purpose |
| --- | --- | --- |
| Language | Rust 2024 (1.94) | Performance, reliability, crash-safety |
| Async | tokio | Process management, HTTP, scheduling |
| HTTP | axum | CLI-daemon communication, orchestrator API |
| Database | sqlx (Postgres primary, SQLite fallback) | Compile-time checked queries, migrations, multi-backend |
| Search | PostgreSQL tsvector + pgvector | Full-text search + semantic vector search |
| Git | gix (gitoxide, pure Rust) | State management, recovery (no C dependency) |
| Config | figment | Multi-source with provenance |
| CLI | clap 4.6 | Subcommands, completions |
| Scheduling | croner | Cron expressions |
| Containers | Podman (primary) / Docker (alternative) | Agent sandboxing, resource isolation |
| Infrastructure | compose.yaml + Containerfile | Postgres container, agent base image |
| Serialization | serde, toml, serde_json | Config, state, output |
| Logging | tracing | Structured JSON |
| Errors | thiserror 2.x | Typed errors per crate |
| IDs | ulid | Sortable unique identifiers |
| Testing | proptest, insta | Property-based, snapshot |

## 7. Cross-Document Implementation Constraints (Codex Review)

These issues were identified by Codex (gpt-5.4) reviewing all docs simultaneously. They represent cross-document inconsistencies that must be resolved during implementation, not in the architecture docs.

| ID | Constraint | Resolution |
| --- | --- | --- |
| XD-001 | Scheduler MUST spawn Claude via SessionManager, not directly | Scheduler calls `session_manager.run_session()`, never `Command::new("claude")` directly |
| XD-002 | All experience traces go to PostgreSQL via SessionManager Stop handler, not ad-hoc JSONL | Hooks write to a temp buffer; SessionManager flushes to `experience_traces` table on session end |
| XD-003 | JSON/TOML mutations commit immediately; PostgreSQL writes are transactional | Document this explicitly — "every mutation" means every file mutation triggers a git commit; DB writes use standard transactions |
| XD-004 | 6 trigger types are phased: cron in Phase 3, remaining triggers in Phase 3.5 | `JobDefinition` starts with `Trigger::Cron` only; `Trigger` enum added in Phase 3.5 |
| XD-005 | stderr must be consumed concurrently with stdout to prevent deadlock | `tokio::io::BufReader` on both stdout and stderr in `select!` loop |
| XD-006 | Rate budget needs a single `RateBudget` contract shared by PRD, scheduler, and session manager | Define in `autonomic-core`; all subsystems reference the same struct |
| XD-007 | Evolution archive needs per-project active variants, not a single global one | `active_variant` is `HashMap<ProjectId, VariantId>` + `global_active: VariantId` |
| XD-008 | `rollback` must preserve gitignored files (`secrets.toml`, WAL files) | Use `git checkout` on tracked files only, never `git clean -f` |
| XD-009 | `memory.sqlite` metadata table needed for MEMORY.md diffing | Add `memory_md_sync` table: `project_id TEXT, last_hash TEXT, last_sync TEXT` |
| XD-010 | Evolution tables (`variants`, `probe_tasks`, etc.) belong in the shared PostgreSQL database | Single database with phase-gated table creation via sqlx migration system |
| XD-011 | LaunchAgent plist does not inherit shell env — secrets must load from file, not env vars | `secrets.toml` is the primary secret source; env var substitution is secondary |

## 8. Open Questions

| ID | Question | Status |
| --- | --- | --- |
| OQ-001 | A2A Agent Card for multi-agent discovery? | Deferred to Phase 6+ |
| OQ-002 | Optional sqlite-vec for semantic search alongside FTS5? | Open — measure FTS5 precision first |
| OQ-003 | Handle Claude Max rate limit changes? | Config-driven (`config.toml`) |
| OQ-004 | Agent Teams auto-trigger or explicit opt-in? | Auto with cost threshold gate (FR-003) |
| OQ-005 | Final project name ("Autonomic")? | Open |
| OQ-006 | TUI (ratatui) for real-time monitoring? | Deferred to post-v1.0 |
| OQ-007 | `autonomic-hooks` as standalone crate? | Yes — standalone-capable |
| OQ-008 | Should the evolution analysis prompts themselves be evolvable (DGM-Hyperagent pattern)? | Open — bounded by modification frontier if yes |
| OQ-009 | Optimal alpha for performance-novelty balance in variant selection? | Start 0.7, tune empirically |
| OQ-010 | Epsilon value for energy convergence bound? | Start 0.5, expect N_max ~100-200 |
| OQ-011 | Should the evolution engine synthesize entirely new hooks (SEAA tool synthesis) or only modify existing ones? | Phase 6+ consideration — SEAA shows 87.2% first-attempt success for tool synthesis |
| OQ-012 | Should capabilities follow SKILL.md format with full YAML frontmatter for cross-platform portability? | Lean yes — adopted by 20+ platforms |
| OQ-013 | Population size for variant archive? | Start 5-10 active variants (ARTEMIS + Survey recommendation), archive unlimited |

## 9. Decisions Log

| ID | Decision | Rationale | Date |
| --- | --- | --- | --- |
| DEC-001 | Rust over TypeScript/Bun | Sub-ms startup, <10MB RSS, crash-safe, no runtime deps. A daemon should be invisible. | 2026-03-25 |
| DEC-002 | ~~SQLite + FTS5~~ Postgres + tsvector + pgvector (superseded by DEC-025/026) | Originally SQLite; upgraded to Postgres for container concurrency, LISTEN/NOTIFY, pgvector. sqlx supports SQLite fallback. | 2026-03-26 |
| DEC-003 | Git-backed state | Point-in-time recovery is first-class. Git: atomic commits, tags, diffs, history, clone for backup. | 2026-03-25 |
| DEC-004 | Archive-based evolution over linear sidecar | GEA: 71% in 30 iters (archive) vs DGM: 50% in 60 iters (linear). Path-dependent improvement requires diversity. | 2026-03-25 |
| DEC-005 | Flip-centered gating over aggregate metrics | AgentDevel: 3.1% bad releases (gated) vs 14.8% (ungated). Per-scenario regression tracking is essential. | 2026-03-25 |
| DEC-006 | Lyapunov convergence bound | MARIA OS: provable convergence in N_max steps. Without bounds, self-modification can cycle or diverge. | 2026-03-25 |
| DEC-007 | Implementation-blind analysis | AgentDevel: blind critic produces better diagnostics than one that sees the implementation. Prevents rationalization. | 2026-03-25 |
| DEC-008 | Cross-project evolution as primary mechanism | GEA: group is the evolutionary unit. Best agent drew from 17 ancestors. Improvements are framework-level and transfer across projects. | 2026-03-25 |
| DEC-009 | External models recommended, not required | Codex/Gemini for verification, but graceful fallback to Opus. Must be fully functional with Claude alone. | 2026-03-25 |
| DEC-010 | CLI subprocess over Agent SDK | SDK is TS/Python. Direct CLI subprocess is simpler, language-native, same structured output. | 2026-03-25 |
| DEC-011 | launchd over systemd/tmux | macOS-native: auto-start, auto-restart, throttle protection. | 2026-03-25 |
| DEC-012 | No `--dangerously-skip-permissions` | `acceptEdits` + hooks. Safer than compensating for skipped permissions. | 2026-03-25 |
| DEC-013 | Timed verification (1h/24h/72h) over "N sessions" | MARIA OS: catches different failure classes at different timescales. | 2026-03-25 |
| DEC-014 | Self-improvement scope: scaffold only | SICA: zero gain on reasoning tasks. Don't target model-capability problems. | 2026-03-25 |
| DEC-015 | Append-only archive, never prune | DGM: failed variants are data, not waste. A variant that failed in one context may succeed in another. | 2026-03-25 |
| DEC-016 | Two-process architecture (daemon + watchdog) | Rust self-evolving agent research: prevents self-bricking when evolution deploys a broken config. Watchdog reverts to last known-good tag. | 2026-03-25 |
| DEC-017 | Filesystem-level modification frontier | Frozen files set `chmod 444`, verified by watchdog. Prompt-level "don't modify X" is insufficient — Structure > Willpower applies to the evolution engine itself. | 2026-03-25 |
| DEC-018 | System prompt re-injected on every --resume | ClaudeClaw source code confirms --append-system-prompt does NOT persist across resumes. Must re-inject context explicitly. | 2026-03-25 |
| DEC-019 | 6 trigger types, not just cron | Clawith's Aware system: cron, once, interval, poll, on_message, webhook. Richer scheduling enables more responsive evolution and monitoring. | 2026-03-25 |
| DEC-020 | SKILL.md format for capability definitions | Agent Skills Standard adopted by 20+ platforms (Claude Code, Codex, Cursor, Gemini CLI). Maximum portability. | 2026-03-25 |
| DEC-021 | Uncertainty-guided selective adaptation | TT-SI: +5.48% with focused adaptation vs +1.04% uniform. 5x more efficient to evolve where the system struggles. | 2026-03-25 |
| DEC-022 | Orchestrator-mediated coordination only | CooperBench: agents achieve ~50% lower success collaborating vs solo. All inter-agent coordination flows through the orchestrator, never peer-to-peer. | 2026-03-25 |
| DEC-023 | ARTEMIS config formalization C = (P, T, M, Theta) | Maps to ConfigSurface. Semantic GA for NL components, standard optimization for numeric. Hierarchical evaluation (cheap before expensive). | 2026-03-25 |
| DEC-024 | Podman/Docker container sandboxing for agents | SICA, MARIA OS SEAA, OpenClaw all use container isolation. Agent sessions run in ephemeral containers with project bind-mounts, network isolation, resource limits. Podman primary (rootless, daemonless), Docker as alternative via ContainerRuntime trait. | 2026-03-26 |
| DEC-025 | PostgreSQL over SQLite for shared state | Concurrent writes from multiple agent containers to SQLite is unreliable across mount boundaries. Postgres: true MVCC, LISTEN/NOTIFY for reactive evolution, tsvector FTS, pgvector for semantic search. Runs in container alongside agents. | 2026-03-26 |
| DEC-026 | sqlx over rusqlite/tokio-postgres | Compile-time checked SQL queries catch errors at build time. Multi-backend (Postgres primary + SQLite fallback). Built-in migration system. | 2026-03-26 |
| DEC-027 | K8s-like container scheduling | Orchestrator manages container pool: ephemeral per-task agents, warm pool for instant assignment, resource limits, network policies, scale based on queue depth + rate budget. | 2026-03-26 |
| DEC-028 | pgvector for semantic search | Resolves OQ-002. Available as Postgres extension. Enables hybrid search (tsvector keyword + pgvector semantic) without external embedding service. | 2026-03-26 |
| DEC-029 | gix over git2 | Pure Rust (no C/libgit2 dependency). Used in operator's commitbee project. Compatible with `#![forbid(unsafe_code)]`. | 2026-03-26 |
