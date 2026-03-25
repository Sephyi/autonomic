# 🤖 Autonomic &emsp; ![MSRV] ![License] ![Status]

[MSRV]: https://img.shields.io/badge/MSRV-1.94-orange.svg
[License]: https://img.shields.io/badge/License-TBA-lightgrey.svg
[Status]: https://img.shields.io/badge/Status-Planning-yellow.svg

**The orchestrator that replaces you as the glue.**

> "You describe the outcome. The system builds, verifies, improves, and learns — without you watching."

> [!CAUTION]
> This project is in its very early stages. The architecture is designed, the research is done, implementation is starting. If you're interested in the concepts, feel free to explore the documentation.

A persistent, self-improving Rust daemon that orchestrates autonomous AI-assisted software development. Manages multiple Claude Code instances across projects, routes tasks to optimal model tiers, enforces quality through structural hooks, accumulates knowledge across sessions, and evolves its own configuration based on measured outcomes.

## ✨ What Sets Autonomic Apart

### 🧬 It improves itself

Archive-based group evolution with flip-centered gating. The system discovers inefficiencies, proposes configuration changes, tests them in isolated worktrees, deploys via sidecar (never overwriting originals), measures effectiveness over timed verification windows (1h/24h/72h), and rolls back failures. Formally bounded — a Lyapunov energy function guarantees convergence.

### 🏗️ Structure > Willpower

Every behavior that matters is enforced by hooks, gates, or validation ratchets — never by instructions alone. A 10-line hook is a guarantee. A 1,000-line prompt is a wish. The #1 pain point in the Claude Code ecosystem is instructions being ignored. Autonomic makes compliance structural.

### 📦 Containerized agents

Agent sessions run in ephemeral Podman/Docker containers with project directories bind-mounted, network isolated, and resources capped. The orchestrator manages lifecycle like a lightweight Kubernetes scheduler — spin up on demand, tear down on completion, scale based on queue depth and rate budget.

### 🧠 Memory that decays and assembles

PostgreSQL-backed memory with tsvector full-text search, pgvector semantic search, typed entries with per-category exponential decay, usefulness tracking, and token-budgeted context assembly. Not an ever-growing markdown file — a living knowledge base that gets smarter and leaner over time.

## 🏛️ Architecture

```txt
HOST (macOS / Linux)
  |
  +-- autonomicd (orchestrator daemon, Rust, ~5MB RSS)
  |     Decision Engine · Evolution Engine · Container Scheduler
  |     Hook Manager · External Coordinator (Codex, Gemini)
  |
  +-- autonomic-watchdog (health monitor, crash rollback)
  |
  +-- podman/docker compose
        +-- postgres:17 (shared state, tsvector, pgvector, LISTEN/NOTIFY)
        +-- agent containers (ephemeral, per-task, isolated)
```

### 📦 Crate Map

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
| `autonomic-watchdog` | Lightweight monitor |
| `autonomic-cli` | CLI interface (`autonomic`) |

## 🚀 Getting Started

### Prerequisites

- Rust 1.94+ (`rustup` will auto-install via `rust-toolchain.toml`)
- Podman or Docker (for agent containers and PostgreSQL)
- `cargo-deny` (`cargo install cargo-deny`)
- `jq` (for hook scripts: `brew install jq`)

### Build

```bash
cargo build --workspace
```

### Test

```bash
cargo test --workspace
```

### Lint

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

### Infrastructure

```bash
# Start PostgreSQL (first time: creates volume + runs migrations)
podman compose up -d    # or: docker compose up -d

# Verify
podman compose ps       # postgres should be healthy
```

## 📊 Current Status

**Phase 0: Environment Setup** — In progress

- PRD v0.5 with 26 feature requirements across 6 phases
- 8 architecture specs (226KB), 9 research papers analyzed (197KB)
- Reviewed by Gemini 3 Pro and Codex gpt-5.4
- 30 design decisions, 13 open questions, 11 cross-document constraints

**Next: Phase 1 — Foundation** (daemon, project registry, session management, git-backed state, metrics)

## 📚 Documentation

| Document | Description |
| --- | --- |
| [`PRD.md`](PRD.md) | Product requirements (v0.5, 26 FRs, 6 phases) |
| [`docs/architecture/`](docs/architecture/) | Implementation specs (8 files, 226KB) |
| [`docs/research/`](docs/research/) | Research paper analyses (9 files, 197KB) |
| [`SOURCES.md`](SOURCES.md) | All research sources with depth annotations |
| [`docs/specs/`](docs/specs/) | Design specs |
| [`docs/plans/`](docs/plans/) | Implementation plans |
| [`docs/review/`](docs/review/) | Gemini + Codex review outputs |

## 🔬 Research Foundation

Built on peer-reviewed research, not assumptions:

| Paper | Key Contribution |
| --- | --- |
| **GEA** (UCSB, 2026) | Group evolution: SWE-bench 20% → 71% in 30 iterations |
| **AgentDevel** (2026) | Flip-centered gating: bad releases 14.8% → 3.1% |
| **MARIA OS** (2026) | Lyapunov convergence bounds for self-modification |
| **SICA** (Bristol, 2025) | Single-agent self-editing: SWE-bench 17% → 53% |
| **DGM** (UBC/Meta, 2025) | Evolutionary archive: append-only, never prune |
| **Instar** (SageMindAI) | Structure > Willpower, compaction recovery hooks |

Full analyses in [`docs/research/`](docs/research/) (9 papers, 197KB of implementation-grade extraction).

## 💛 Sponsor

If you find Autonomic useful, consider [**sponsoring my work**](https://github.com/sponsors/Sephyi) — it helps keep the project going.

## 📜 License

License not yet determined. All rights reserved until announced. See [`LICENSE`](LICENSE).
