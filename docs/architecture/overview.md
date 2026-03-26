# Architecture Overview

**Status**: Draft
**Last updated**: 2026-03-26

## 1. What Autonomic Is

A persistent Rust daemon that replaces the human as the orchestration layer for AI-assisted software development. It manages multiple Claude Code instances across projects, routes tasks to optimal model tiers, enforces quality through structural hooks, accumulates knowledge across sessions, and evolves its own configuration based on measured outcomes.

## 2. System Diagram

```txt
HOST (macOS)
  |
  launchd
  |
  +-- autonomic-watchdog (~1MB, monitors daemon, crash rollback)
  |
  +-- autonomic daemon (Rust, axum, ~5MB RSS)
  |     |
  |     +-- podman compose (infrastructure)
  |     |     |
  |     |     +-- postgres:17 (sessions, metrics, memory, traces)
  |     |     |   Port 5432, volume: autonomic-pgdata
  |     |     |
  |     |     +-- agent containers (on-demand, per-session)
  |     |         autonomic-agent:latest
  |     |         podman run --rm -v project:/workspace:rw ...
  |     |
  |     +------------------------------------------------------------+
  |     |                     CORE SUBSYSTEMS                         |
  |     |                                                             |
  |     |  Decision Engine -----> Session Manager ----> Containers    |
  |     |  (Opus, on-demand)      (container spawn)    (podman run,  |
  |     |  Task classification    Output parsing        stream-json) |
  |     |  Model routing          Cost tracking                       |
  |     |  Delegation             Timeout enforcement                 |
  |     |                                                             |
  |     |  Evolution Engine -----> Variant Archive                    |
  |     |  (self-improvement)      (append-only, git-backed)          |
  |     |  Discovery (traces)      Selection (perf + novelty)         |
  |     |  Analysis (impl-blind)   Flip-centered gating               |
  |     |  Cross-project rounds    Timed verification (1h/24h/72h)    |
  |     |  Bounded (MARIA OS)      Sidecar deployment                 |
  |     |                                                             |
  |     |  Memory Store ----------> PostgreSQL + tsvector             |
  |     |  (context assembly)       Typed entries, decay              |
  |     |  Session injection        Usefulness tracking               |
  |     |  MEMORY.md sync           Auto-capture decisions            |
  |     |                                                             |
  |     |  Scheduler ---------------> Cron Jobs                       |
  |     |  (croner)                   Health checks (Haiku)           |
  |     |  Rate budget aware          Evolution analysis (Haiku)      |
  |     |  Priority-based             Reflection (Haiku)              |
  |     |                                                             |
  |     |  Hook Manager            State Store                        |
  |     |  (install/update/evolve)  (git-backed ~/.autonomic/)        |
  |     |  Global + project hooks   PostgreSQL + JSON + TOML          |
  |     |  Compaction recovery      Point-in-time recovery            |
  |     |  Metrics collection       Auto-commit every mutation        |
  |     +------------------------------------------------------------+
  |     |                                                             |
  |     |  External Coordinators (recommended, not required)          |
  |     |  Codex CLI (/opt/homebrew/bin/codex)                        |
  |     |  Gemini CLI (/opt/homebrew/bin/gemini)                      |
  |     |  Fallback: Opus self-review                                 |
  |     +------------------------------------------------------------+
  |
  +--- Project A (vox-scribe) ---> agent container session
  |    Own CLAUDE.md, hooks, agents, specs
  |
  +--- Project B (bind9-sdk) ----> agent container session
  |    Own CLAUDE.md, hooks, agents, specs
  |
  +--- Project C (commitbee) ----> agent container session
       Own CLAUDE.md, hooks, agents, specs
```

## 3. Crate Map

```txt
autonomic/
  Cargo.toml                     # workspace root
  crates/
    autonomic-core/              # Types, traits, config, errors
      Session, Project, MemoryEntry, ExperienceTrace,
      ConfigSurface, ConfigVariant, VariantArchive,
      EvolutionProposal, ProbeTask, FlipReport
    autonomic-db/                # PostgreSQL connection pool, migrations, sqlx queries
      DbPool, Migrations, QueryHelpers
    autonomic-container/         # Podman/Docker container lifecycle management
      ContainerManager, ContainerConfig, ImageBuilder,
      VolumeMount, ContainerHealth
    autonomic-memory/            # PostgreSQL + tsvector memory store
      MemoryStore, ContextAssembler, DecayEngine,
      MemoryMdSync, AutoCapture
    autonomic-evolution/         # Self-improvement flywheel
      EvolutionEngine, VariantArchive, VariantSelector,
      FlipCenteredGate, EnergyFunction, SidecarManager,
      WorktreeTestHarness, CrossProjectEvolver,
      GovernanceProfile, RubberStampDetector
    autonomic-session/           # Claude Code subprocess management
      SessionManager, OutputParser, CompletionDetector,
      CostTracker, WorktreeIsolator
    autonomic-scheduler/         # Cron + rate budget
      Scheduler, JobDefinition, RateBudgetManager,
      QuotaTracker, AdaptiveScheduler
    autonomic-hooks/             # Hook management (standalone-capable)
      HookManager, HookInstaller, HookTemplate,
      CompactionRecovery, SessionStartContext,
      CommandGuard, MetricsCollector
    autonomic-routing/           # Model routing
      TaskClassifier, RoutingTable, PerformanceMatrix,
      ExternalDispatcher, FallbackChain
    autonomic-state/             # Git-backed state management
      StateStore, GitManager, SnapshotManager,
      MigrationRunner
    autonomic-daemon/            # Main binary
      main.rs, axum routes, launchd integration
    autonomic-cli/               # CLI tool
      autonomic status, evolve, memory, project, schedule,
      rollback, models, budget
    autonomic-watchdog/        # Lightweight monitor: health check, crash rollback, permission verify
```

### 3.1 Dependency Graph

```txt
autonomic-daemon
  +-- autonomic-cli
  +-- autonomic-routing
  |     +-- autonomic-session
  |     +-- autonomic-container
  |     +-- autonomic-core
  +-- autonomic-evolution
  |     +-- autonomic-memory
  |     +-- autonomic-session
  |     +-- autonomic-state
  |     +-- autonomic-core
  +-- autonomic-scheduler
  |     +-- autonomic-session
  |     +-- autonomic-state
  |     +-- autonomic-core
  +-- autonomic-hooks (also standalone-usable)
  |     +-- autonomic-core
  +-- autonomic-memory
  |     +-- autonomic-db
  |     +-- autonomic-core
  +-- autonomic-container
  |     +-- autonomic-core
  +-- autonomic-db
  |     +-- autonomic-core
  +-- autonomic-state
        +-- autonomic-db
        +-- autonomic-core
```

`autonomic-hooks` is deliberately standalone — it can be used without the full orchestrator, providing just the hook infrastructure (compaction recovery, command guard, metrics collection).

## 4. Data Flow

### 4.1 Session Lifecycle

```txt
1. Task arrives (CLI command, scheduled job, or operator request)
2. Decision Engine classifies task type
3. Routing selects model tier + context size
4. Memory Store assembles relevant context (token-budgeted)
5. Session Manager spawns agent container:
   podman run --rm -v project:/workspace:rw ... autonomic-agent:latest
     claude -p "<prompt>" --output-format stream-json
     --model <tier> --permission-mode acceptEdits
     --allowedTools "Read,Edit,Write,Bash,Glob,Grep"
6. Hooks fire at lifecycle points:
   SessionStart: inject assembled memory context
   PreToolUse: command guard, safety checks
   PostToolUse: format, lint, metrics
   Stop: collect experience trace, session metrics
7. Output parsed: message stream, completion detection, cost tracking
8. Experience trace written to shared pool
9. Memory updated: auto-capture significant decisions/errors
10. Rate budget updated: track cost against allocation
```

### 4.2 Evolution Cycle

```txt
1. Experience traces accumulate from all sessions across all projects
2. Scheduled analysis (weekly, Haiku): implementation-blind diagnostics
3. Proposals generated from diagnostics (separate repair step)
4. Parent variant selected from archive (performance + novelty)
5. New variant created: parent config + proposed changes
6. Probe task battery run against new variant (flip-centered gating)
7. If gates pass: timed deployment (1h -> 24h -> 72h verification)
8. If deployed: add to archive as Active, tag in git
9. If failed at any stage: add to archive as Failed, rollback, log learning
10. Cross-project evolution round: aggregate, reflect, propagate
```

### 4.3 Memory Flow

```txt
Session starts:
  1. Hook fires (SessionStart)
  2. ContextAssembler queries PostgreSQL tsvector with prompt keywords
  3. Entries scored: ts_rank * usefulness * decay * category_boost
  4. Top entries packed within token budget (tiered rendering)
  5. Context block injected via hook stdout

Session ends:
  1. Hook fires (Stop)
  2. AutoCapture evaluates session significance
  3. New memory entries created from significant findings
  4. MEMORY.md ingestion: parse Claude's auto-memory writes
  5. Usefulness updated: helpful++ for entries in successful sessions

Between sessions:
  1. DecayEngine does NOT modify entries (decay computed at query time)
  2. RetirementScan runs weekly, retires entries with relevance < 0.01
  3. MEMORY.md regenerated for each project before next session
```

## 5. Key Design Decisions

| Decision | Choice | Rationale | Reference |
| --- | --- | --- | --- |
| Language | Rust 2024 (1.94) | Sub-ms startup, <10MB RSS, crash-safe, no runtime deps | Operator preference + daemon requirements |
| Database | PostgreSQL 17 (containerized) | Concurrent access, tsvector full-text, pgvector-ready, sqlx compile-time checks | Memory system spec, container architecture |
| State persistence | Git-backed | Point-in-time recovery, audit trail, diff, tags, clone for backup | MARIA OS (immutable audit), DGM (append-only archive) |
| Evolution model | Archive-based group evolution | GEA: 71% SWE-bench in 30 iters vs DGM 50% in 60 iters (linear) | GEA, DGM papers |
| Gating | Flip-centered (P2P/P2F/F2P/F2F) | AgentDevel: 3.1% bad releases vs 14.8% without gating | AgentDevel paper |
| Convergence | Lyapunov energy bound | MARIA OS: provable convergence in N_max = floor(V_0/epsilon) steps | MARIA OS paper |
| Analysis | Implementation-blind | AgentDevel: blind critic produces better diagnostics | AgentDevel paper |
| Memory | tsvector keyword + LLM relevance judging | <1ms queries, no embedding model. Let Claude judge relevance from candidates. pgvector available for future semantic search. | Operator's Mem0 experience |
| External models | Recommended, not required | Codex/Gemini for verification, but graceful fallback to Opus | Operator requirement |
| Permissions | acceptEdits, NOT --dangerously-skip-permissions | Built-in permissions + hook enforcement. Safer than compensating for skipped permissions. | Security principle |
| Process supervision | launchd Launch Agent | macOS-native, auto-start, auto-restart, throttle protection | Orchestrator research |
| Hook architecture | Structure > Willpower | Instar principle. Hooks are deterministic; instructions are probabilistic. | Instar analysis |
| DEC-016 | Two-process (daemon+watchdog) | Prevents self-bricking from evolution failures | Watchdog design |
| DEC-017 | Filesystem modification frontier | chmod 444 on frozen files, verified by watchdog | Security model |
| DEC-018 | System prompt re-injection on resume | --append-system-prompt doesn't persist | ClaudeClaw source analysis |
| DEC-019 | 6 trigger types | cron, once, interval, poll, on_message, webhook | Scheduler spec |
| DEC-020 | SKILL.md format | 20+ platform portability | Skills research |
| DEC-021 | Uncertainty-guided adaptation | 5x more efficient than uniform | ARTEMIS research |
| DEC-022 | Orchestrator-mediated only | CooperBench: 50% worse collaborating | MAS Design Patterns study |
| DEC-023 | ARTEMIS formalization | C=(P,T,M,Theta), semantic GA + Bayesian | ARTEMIS paper |
| DEC-024 | Container sandboxing | Agent sessions run inside Podman/Docker containers, not as host subprocesses. Filesystem isolation, resource limits, reproducible environments. | Container architecture |
| DEC-025 | PostgreSQL over SQLite | Concurrent multi-session writes, tsvector FTS, pgvector for future embeddings, rich query planner. Containerized — no host install required. | Database migration |
| DEC-026 | sqlx over sqlx | Compile-time checked SQL queries, async-native, connection pooling. Eliminates runtime SQL errors. | Database migration |
| DEC-027 | K8s-like scheduling | Container lifecycle mirrors Kubernetes pod model: create -> configure (mounts, env, limits) -> run -> capture -> destroy. | Container architecture |
| DEC-028 | pgvector for semantic search | Extension available in Postgres container. Resolves OQ-002 (semantic search path). Not active yet — tsvector keyword search is primary. | Memory system spec |
| DEC-029 | gix over git2 | Pure Rust, no libgit2 C dependency, better async compatibility, actively maintained. | State management |

## 6. Technology Stack

| Component | Crate/Tool | Version | Purpose |
| --- | --- | --- | --- |
| Async runtime | tokio | latest | Process management, HTTP server, scheduling |
| HTTP server | axum | latest | CLI-daemon communication, future API |
| Database | sqlx + PostgreSQL 17 | latest | Memory store, metrics, state (compile-time checked queries) |
| Containers | podman / docker | latest | Agent sandboxing, PostgreSQL hosting |
| Git | gix | latest | State management, point-in-time recovery |
| Config | figment | latest | Multi-source config with provenance |
| CLI | clap | 4.6 | Subcommands, shell completions |
| Scheduling | croner | latest | Cron expressions |
| Serialization | serde, toml, serde_json | latest | Config (TOML), state (JSON) |
| Logging | tracing, tracing-subscriber | latest | Structured JSON logging |
| Errors | thiserror | 2.x | Typed errors per crate |
| Process mgmt | tokio::process | latest | Container orchestration, Claude Code subprocess |
| ID generation | ulid | latest | Sortable unique IDs |
| Testing | proptest, insta | latest | Property-based + snapshot testing |

## 7. Security Model

1. **No `--dangerously-skip-permissions`** — use `acceptEdits` + hook enforcement
2. **Command guard** blocks destructive commands at the hook level (PreToolUse)
3. **External operations** gated by the operator's existing permission prompts
4. **Git-backed state** provides audit trail for all changes
5. **Modification frontier** prevents evolution engine from touching safety config
6. **Rate budget emergency reserve** (5%) always available
7. **Atomic state mutations** via git — no partial writes
8. **PostgreSQL MVCC** — no corruption from concurrent access, container-isolated

## 8. Failure Modes and Recovery

| Failure | Detection | Recovery |
| --- | --- | --- |
| Daemon crash | launchd KeepAlive | Auto-restart within 10s |
| Claude Code timeout | tokio::time::timeout | Kill subprocess, log failure, retry or skip |
| Rate limit exceeded | Budget tracker | Defer non-critical tasks, downgrade model tiers |
| Evolution regression | Flip-centered gate | Auto-rollback, log in learning registry |
| PostgreSQL corruption | Container volume + pg_dump backups | Restore from backup, rebuild container |
| Git corruption | Shouldn't happen (local only) | Clone from backup |
| External model unavailable | Timeout + exit code check | Immediate fallback to Opus self-review |
| Context compaction | SessionStart hook (compact matcher) | Inject identity + memory + task context directly |

## 9. Detailed Specifications

Each subsystem has its own implementation-grade specification:

| Document | Path | Key Content |
| --- | --- | --- |
| Evolution Engine | `evolution-engine.md` | Archive architecture, variant selection, flip-centered gating, timed deployment, cross-project rounds, energy function, governance profiles |
| Memory System | `memory-system.md` | PostgreSQL schema, tsvector queries, context assembly algorithm, decay math, MEMORY.md sync, auto-capture, usefulness tracking |
| Session Management | `session-management.md` | Subprocess spawning, output parsing, completion detection, cost tracking, worktree isolation, Agent Teams |
| Hook System | `hook-system.md` | All hooks with source code, compaction recovery, command guard, metrics collection, hook evolution |
| Model Routing | `model-routing.md` | Task classification, routing table, empirical performance matrix, external dispatch, verification pipeline |
| State Management | `state-management.md` | Git operations (gix), PostgreSQL schemas, recovery procedures, migration strategy |
| Scheduler | `scheduler.md` | Job definitions, rate budget math, adaptive scheduling |

## 10. Research Foundation

Implementation decisions are grounded in peer-reviewed research:

| Paper | What We Use | Spec Reference |
| --- | --- | --- |
| SICA | Utility function, overseer pattern, self-improvement limits | evolution-engine.md §8 |
| AgentDevel | Flip-centered gating, implementation-blind analysis, regression-aware pipeline | evolution-engine.md §4.4 |
| GEA | Archive-based group evolution, performance+novelty selection, cross-project rounds | evolution-engine.md §3, §4.6 |
| DGM | Append-only archive, exploration pressure (inverse child count) | evolution-engine.md §3.2 |
| DGM-Hyperagent | Evolvable meta-level (bounded by modification frontier) | evolution-engine.md §2.1 |
| MARIA OS | Lyapunov convergence bound, modification frontier, timed verification, energy function | evolution-engine.md §5 |
| Godel Agent | Cost-efficient self-improvement ($15 vs $300), recursive improvement loop | evolution-engine.md §10 |

Detailed paper analyses: `../research/*.md` (112KB, 2,380 lines of implementation-grade extraction)
