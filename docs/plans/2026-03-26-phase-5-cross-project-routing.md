# Phase 5: Cross-Project Evolution and Model Routing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build cross-project group evolution (GEA pattern), external model dispatch (Codex/Gemini), and empirical model performance learning in the `autonomic-routing` crate.

**Architecture:** Cross-project evolution uses the GEA pattern: projects are group members, shared experience pool aggregates traces, reflection generates evolution directives, each project generates patches from shared directives. Model routing starts heuristic (day 1) and transitions to empirical (after 20 samples per model-task pair). External models are dispatched via CLI subprocess with timeout and graceful fallback.

**Tech Stack:** Rust 2024 (1.94), sqlx, tokio, serde

**Spec:** `docs/architecture/evolution-engine.md` §Cross-Project, `docs/architecture/model-routing.md`

**Prereqs from Phase 1-4:** VariantArchive, ProbeRunner, FlipGating, EnergyTracker, SessionManager, MemoryStore, model_performance table

## File Map

```txt
MODIFY: crates/autonomic-routing/Cargo.toml
CREATE: crates/autonomic-routing/src/lib.rs
CREATE: crates/autonomic-routing/src/error.rs           # RoutingError
CREATE: crates/autonomic-routing/src/classifier.rs      # Task type classification
CREATE: crates/autonomic-routing/src/heuristic.rs       # Day-1 heuristic routing rules
CREATE: crates/autonomic-routing/src/empirical.rs       # Performance matrix, learning from outcomes
CREATE: crates/autonomic-routing/src/router.rs          # Combined routing (heuristic + empirical override)
CREATE: crates/autonomic-routing/src/external.rs        # Codex/Gemini CLI dispatch

MODIFY: crates/autonomic-evolution/src/lib.rs           # Add cross-project module
CREATE: crates/autonomic-evolution/src/cross_project.rs # GEA group evolution rounds

CREATE: infra/migrations/005_routing_indexes.sql        # Indexes for model_performance queries
```

## Task 1: Task Classifier

**Files:**
- Create: `crates/autonomic-routing/src/classifier.rs`

Classify tasks into categories for routing decisions:
- `implementation` — writing new code
- `debugging` — fixing bugs, investigating failures
- `refactoring` — restructuring existing code
- `testing` — writing or running tests
- `review` — code review, security review
- `research` — exploration, documentation reading
- `planning` — architecture, design decisions

Classification uses keyword matching on the prompt + project context.

- [ ] **Step 1: Create classifier.rs with TaskType enum and classify function**
- [ ] **Step 2: Add tests for classification**
- [ ] **Step 3: Commit**

## Task 2: Heuristic Routing

**Files:**
- Create: `crates/autonomic-routing/src/heuristic.rs`

Day-1 routing rules (before empirical data):

| Task Type | Default Model |
| --- | --- |
| planning, review | opus |
| implementation, debugging | sonnet |
| testing, research | sonnet |
| refactoring | sonnet |

External model routing:
- Verification tasks → prefer Codex/Gemini if available
- Standard tasks → Claude only

- [ ] **Step 1: Create heuristic.rs with routing table**
- [ ] **Step 2: Commit**

## Task 3: Performance Matrix (FR-023)

**Files:**
- Create: `crates/autonomic-routing/src/empirical.rs`

Build `model_performance[model][task_type]` from measured outcomes:
- Record: model, task_type, outcome, cost, time per session
- Aggregate success rate, avg cost, avg duration per pair
- Override heuristic routing at 20+ samples per pair
- SQL queries against `model_performance` table

- [ ] **Step 1: Create empirical.rs with recording and aggregation**
- [ ] **Step 2: Add tests**
- [ ] **Step 3: Commit**

## Task 4: Combined Router

**Files:**
- Create: `crates/autonomic-routing/src/router.rs`

```rust
pub struct ModelRouter {
    heuristic: HeuristicRouter,
    empirical: EmpiricalRouter,
    sample_threshold: usize, // default: 20
}

impl ModelRouter {
    pub async fn route(&self, task_type: TaskType, pool: &PgPool) -> ModelTier {
        if let Some(empirical) = self.empirical.best_model(task_type, pool).await {
            if empirical.sample_count >= self.sample_threshold {
                return empirical.model;
            }
        }
        self.heuristic.route(task_type)
    }
}
```

- [ ] **Step 1: Create router.rs combining heuristic and empirical**
- [ ] **Step 2: Commit**

## Task 5: External Model Dispatch (FR-022)

**Files:**
- Create: `crates/autonomic-routing/src/external.rs`

Dispatch to external models via CLI subprocess:
- Codex: `/opt/homebrew/bin/codex exec "<prompt>"`
- Gemini: `/opt/homebrew/bin/gemini -p "<prompt>"`
- Timeout enforcement (30s default)
- Graceful fallback to Opus self-review when unavailable
- Parse output for structured results

- [ ] **Step 1: Create external.rs with CLI dispatch and timeout**
- [ ] **Step 2: Commit**

## Task 6: Cross-Project Evolution (FR-021)

**Files:**
- Create: `crates/autonomic-evolution/src/cross_project.rs`

GEA group evolution rounds:
1. Aggregate experience traces across all projects into shared pool
2. Reflection session: generate evolution directives from shared patterns
3. Each project generates its own patches from shared directives
4. Track ancestor breadth (how many source projects contributed)
5. Framework-level improvements propagate across projects

```rust
pub async fn run_cross_project_round(
    projects: &[ProjectId],
    archive: &VariantArchive,
    pool: &PgPool,
    session_manager: &SessionManager,
) -> Result<CrossProjectOutcome, EvolutionError> { ... }
```

- [ ] **Step 1: Create cross_project.rs with group evolution round**
- [ ] **Step 2: Commit**

## Task 7: Integration

- [ ] **Step 1: Update lib.rs for both crates**
- [ ] **Step 2: Wire model routing into SessionManager** (route before spawn)
- [ ] **Step 3: Add CLI commands** (`autonomic models show`, `autonomic evolve status`)
- [ ] **Step 4: Verify and tag**

```bash
SQLX_OFFLINE=true cargo check -p autonomic-routing -p autonomic-evolution
git tag -a milestone/phase-5 -m "Phase 5: Cross-project evolution, model routing, external dispatch (FR-021 through FR-023)"
```
