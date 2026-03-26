# Phase 4: Evolution Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `autonomic-evolution` crate: append-only variant archive, probe task battery, flip-centered gating, implementation-blind analysis, timed verification deployment, energy function with convergence bounds, and learning registry.

**Architecture:** The evolution engine operates as a multi-stage pipeline (DISCOVER → ANALYZE → PROPOSE → TEST → DEPLOY → CROSS-PROJECT). Each stage is a separate module. The variant archive is PostgreSQL-backed (append-only). Probes are standardized test scenarios stored per-project. Flip-centered gating classifies each probe outcome as P2P/P2F/F2P/F2F. Analysis is implementation-blind (receives only traces and scores, never config content). Deployment uses timed verification windows (1h → 24h → 72h) with auto-rollback.

**Tech Stack:** Rust 2024 (1.94), sqlx (Postgres), serde, tokio, chrono

**Spec:** `docs/architecture/evolution-engine.md` (25KB, 586 lines)

**Prereqs from Phase 1-3:** SessionManager, MemoryStore, Scheduler, RateBudget, GitManager, experience_traces table, all hooks

**Key Constraints:**
- Archive is append-only — variants never deleted or pruned (DEC-015)
- Modification frontier enforced at filesystem level (DEC-017)
- Analysis is implementation-blind (DEC-007)
- Energy must strictly decrease per accepted modification (DEC-006)
- 10% budget allocation for evolution (BudgetAllocation)

## File Map

```txt
MODIFY: crates/autonomic-evolution/Cargo.toml
CREATE: crates/autonomic-evolution/src/lib.rs
CREATE: crates/autonomic-evolution/src/error.rs          # EvolutionError
CREATE: crates/autonomic-evolution/src/config_surface.rs  # ConfigSurface (C = P, T, M, Theta)
CREATE: crates/autonomic-evolution/src/archive.rs         # Variant archive (append-only, PostgreSQL)
CREATE: crates/autonomic-evolution/src/probe.rs           # Probe task battery
CREATE: crates/autonomic-evolution/src/gating.rs          # Flip-centered gating (P2P/P2F/F2P/F2F)
CREATE: crates/autonomic-evolution/src/analysis.rs        # Implementation-blind analysis
CREATE: crates/autonomic-evolution/src/proposal.rs        # ConfigChange proposals
CREATE: crates/autonomic-evolution/src/deployment.rs      # Timed verification (1h/24h/72h)
CREATE: crates/autonomic-evolution/src/energy.rs          # Lyapunov energy function
CREATE: crates/autonomic-evolution/src/learning.rs        # Learning registry
CREATE: crates/autonomic-evolution/src/pipeline.rs        # Full evolution cycle orchestration

CREATE: infra/migrations/004_evolution_tables.sql         # variants, probe_tasks, probe_results, flip_reports, energy_history, learnings
```

## Task 1: Migration — Evolution Tables (FR-014, FR-015, FR-016, FR-019, FR-020)

**Files:**
- Create: `infra/migrations/004_evolution_tables.sql`

Tables needed:
- `variants` (id, parent_id, ancestors JSONB, config_surface JSONB, status, energy, created_at)
- `probe_tasks` (id, project_id, name, command, expected_outcome, is_critical, created_at, retired_at)
- `probe_results` (id, variant_id, probe_id, outcome, duration_ms, output TEXT, created_at)
- `flip_reports` (id, variant_id, parent_id, p2p_count, p2f_count, f2p_count, f2f_count, rho_p2f, gate_result, created_at)
- `energy_history` (id, variant_id, energy DOUBLE, components JSONB, created_at)
- `learnings` (id, category, source_proposal, content, applied BOOL, created_at)

- [ ] **Step 1: Create migration SQL**
- [ ] **Step 2: Commit**

## Task 2: ConfigSurface (ARTEMIS C = P, T, M, Theta)

**Files:**
- Create: `crates/autonomic-evolution/src/config_surface.rs`

The configuration surface formalizes what the evolution engine can modify:
- **P** (Prompts): CLAUDE.md rules, system prompts
- **T** (Tools): hook scripts, hook registrations
- **M** (Memory): decay rates, assembly token budget, category boosts
- **Theta** (Numeric): timeout thresholds, cost budgets, concurrency limits

```rust
pub struct ConfigSurface {
    pub prompts: Vec<PromptConfig>,      // CLAUDE.md sections, system prompts
    pub tools: Vec<ToolConfig>,          // Hook scripts + registrations
    pub memory: MemoryConfig,            // Decay rates, budgets, boosts
    pub theta: HashMap<String, f64>,     // Numeric parameters
}
```

- [ ] **Step 1: Create config_surface.rs with types and serialization**
- [ ] **Step 2: Create error.rs**
- [ ] **Step 3: Commit**

## Task 3: Variant Archive (FR-014)

**Files:**
- Create: `crates/autonomic-evolution/src/archive.rs`

Append-only archive of configuration variants backed by PostgreSQL.

Key operations:
- `create_variant(parent_id, surface) -> VariantId`: Insert new variant
- `get_variant(id) -> Variant`: Fetch variant with full surface
- `select_parent(project_id) -> VariantId`: Softmax selection over `alpha * performance + (1-alpha) * novelty + 0.1 * exploration_bonus`
- `active_variant(project_id) -> VariantId`: Get currently deployed variant
- `set_active(project_id, variant_id)`: Deploy a variant
- `seed_from_current(project_id)`: Create seed variant from operator's existing config

Selection algorithm uses DGM's inverse child count for exploration pressure.

- [ ] **Step 1: Create archive.rs with variant CRUD and parent selection**
- [ ] **Step 2: Add tests for selection algorithm**
- [ ] **Step 3: Commit**

## Task 4: Probe Task Battery (FR-015)

**Files:**
- Create: `crates/autonomic-evolution/src/probe.rs`

Standardized test scenarios per project:
- `register_probe(project_id, name, command, expected, is_critical)`: Add a probe
- `run_probes(project_id, variant_id) -> Vec<ProbeResult>`: Execute all probes
- `add_failure_probe(error_trace)`: Auto-create probe from new failure mode
- Probes are never deleted, only retired

10-30 probes per project: compilation, tests, hook timing, past failure scenarios.

- [ ] **Step 1: Create probe.rs with probe registration and execution**
- [ ] **Step 2: Commit**

## Task 5: Flip-Centered Gating (FR-016)

**Files:**
- Create: `crates/autonomic-evolution/src/gating.rs`

Per-scenario regression detection:
- Classify each probe: P2P (stable pass), P2F (REGRESSION), F2P (improvement), F2F (stable fail)
- Gate formula: `rho_P2F = |P2F| / (|passes| + epsilon)` must be < threshold (default 2%)
- Intent alignment: F2P improvements must correspond to proposal rationale
- No critical probes in P2F set (zero-regression on critical probes)
- Energy check: `V(proposed) < V(current) - epsilon`

```rust
pub struct FlipReport {
    pub p2p: Vec<ProbeId>,    // Stable pass
    pub p2f: Vec<ProbeId>,    // REGRESSION
    pub f2p: Vec<ProbeId>,    // Improvement
    pub f2f: Vec<ProbeId>,    // Stable fail
    pub rho_p2f: f64,
    pub gate_passed: bool,
    pub critical_regression: bool,
}
```

- [ ] **Step 1: Create gating.rs with flip classification and gate evaluation**
- [ ] **Step 2: Add proptest for gate invariants**
- [ ] **Step 3: Commit**

## Task 6: Implementation-Blind Analysis (FR-017)

**Files:**
- Create: `crates/autonomic-evolution/src/analysis.rs`

The analysis session receives ONLY:
- Experience traces (outcomes, costs, durations)
- Probe results (pass/fail per scenario)
- Scoring rubric

It does NOT receive: config content, hook source code, rule text.

Output: diagnostics with severity (pattern type + affected metric).

Uses uncertainty-guided selective adaptation (TT-SI): detect task categories with highest failure rates, focus proposals there.

- [ ] **Step 1: Create analysis.rs with blind analysis protocol**
- [ ] **Step 2: Create proposal.rs with ConfigChange generation from diagnostics**
- [ ] **Step 3: Commit**

## Task 7: Timed Verification Deployment (FR-018)

**Files:**
- Create: `crates/autonomic-evolution/src/deployment.rs`

Three verification windows with auto-rollback:
1. **Window 1 (1h)**: Run probe battery in production context
2. **Window 2 (24h)**: Aggregate session traces, compare to 7-day baseline
3. **Window 3 (72h)**: Check targeted metric improvement, no regressions > 3%

Git tags at each stage: `evo-{id}-testing`, `evo-{id}-deployed`, `evo-{id}-rollback-{window}`

- [ ] **Step 1: Create deployment.rs with staged verification and auto-rollback**
- [ ] **Step 2: Commit**

## Task 8: Energy Function (FR-019)

**Files:**
- Create: `crates/autonomic-evolution/src/energy.rs`

Lyapunov energy function:
```
V(M) = w_regression * L_regression + w_hook_overhead * L_hook_overhead
     + w_unused_rules * L_unused_rules + w_error_rate * L_error_rate
     + w_routing_waste * L_routing_waste + w_memory_staleness * L_memory_staleness
```

Each accepted modification: `V(M_{t+1}) < V(M_t) - epsilon`
Upper bound: `N_max = floor(V(M_0) / epsilon)` ~50-200 modifications to convergence

System enters maintenance mode after convergence. Drift detection: energy increase > 10% triggers return to evolution mode.

- [ ] **Step 1: Create energy.rs with energy computation and convergence tracking**
- [ ] **Step 2: Add proptest for energy monotonic decrease**
- [ ] **Step 3: Commit**

## Task 9: Learning Registry (FR-020)

**Files:**
- Create: `crates/autonomic-evolution/src/learning.rs`

Structured insights from evolution outcomes:
- `record_learning(category, source_proposal, content, applied)`: Create entry
- `search_learnings(query) -> Vec<Learning>`: Search before proposing
- Failed proposals create learning entries with root cause

- [ ] **Step 1: Create learning.rs with CRUD**
- [ ] **Step 2: Commit**

## Task 10: Pipeline Orchestration

**Files:**
- Create: `crates/autonomic-evolution/src/pipeline.rs`

Full evolution cycle: DISCOVER → ANALYZE → PROPOSE → TEST → DEPLOY → CROSS-PROJECT

```rust
pub async fn run_evolution_cycle(
    archive: &VariantArchive,
    probes: &ProbeRunner,
    energy: &EnergyTracker,
    session_manager: &SessionManager,
    project_id: &ProjectId,
) -> Result<EvolutionOutcome, EvolutionError> { ... }
```

- [ ] **Step 1: Create pipeline.rs orchestrating all stages**
- [ ] **Step 2: Update lib.rs with all exports**
- [ ] **Step 3: Verify and tag**

```bash
SQLX_OFFLINE=true cargo check -p autonomic-evolution
git tag -a milestone/phase-4 -m "Phase 4: Evolution Engine (FR-014 through FR-020)"
```
