# Phase 6: Governance and Advanced Features Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add graduated governance profiles, A/B testing framework, and capability gap tracking. These are the final features completing the Autonomic v0.6.0 roadmap.

**Architecture:** Governance profiles control how much human oversight the evolution engine requires. A/B testing uses paired worktrees with identical probe batteries. Capability gap tracking identifies what the system cannot do and distinguishes model-bound limitations from scaffold-improvable ones.

**Tech Stack:** Rust 2024 (1.94), sqlx, tokio, serde

**Spec:** `docs/architecture/evolution-engine.md` §Governance

**Prereqs from Phase 1-5:** All crates fully implemented. Evolution pipeline operational. Model routing empirical. Cross-project rounds working.

## File Map

```txt
CREATE: crates/autonomic-evolution/src/governance.rs     # Governance profiles + rubber-stamp detection
CREATE: crates/autonomic-evolution/src/ab_testing.rs     # A/B testing framework
CREATE: crates/autonomic-evolution/src/capability_gap.rs # Capability gap tracker

CREATE: infra/migrations/006_governance_tables.sql       # governance_config, ab_tests, capability_gaps
```

## Task 1: Migration — Governance Tables

**Files:**
- Create: `infra/migrations/006_governance_tables.sql`

```sql
-- Governance configuration per project
CREATE TABLE governance_config (
    project_id TEXT PRIMARY KEY,
    profile TEXT NOT NULL DEFAULT 'supervised',  -- cautious, supervised, collaborative, autonomous
    acceptance_window JSONB NOT NULL DEFAULT '[]',  -- Rolling 20-proposal window
    rubber_stamp_count INTEGER NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- A/B test sessions
CREATE TABLE ab_tests (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    control_variant_id TEXT NOT NULL,
    experimental_variant_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'running',  -- running, completed, aborted
    control_results JSONB,
    experimental_results JSONB,
    winner TEXT,  -- control, experimental, inconclusive
    confidence DOUBLE PRECISION,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

-- Capability gaps
CREATE TABLE capability_gaps (
    id TEXT PRIMARY KEY,
    severity TEXT NOT NULL,  -- critical, major, minor
    category TEXT NOT NULL,  -- model_bound, scaffold_improvable, unknown
    description TEXT NOT NULL,
    context TEXT,
    linked_proposal_id TEXT,  -- If an evolution proposal addresses this gap
    resolved_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

- [ ] **Step 1: Create migration**
- [ ] **Step 2: Commit**

## Task 2: Graduated Governance (FR-024)

**Files:**
- Create: `crates/autonomic-evolution/src/governance.rs`

Four profiles with escalating autonomy:

| Profile | Approval | Auto-deploy | Evolution Mode |
| --- | --- | --- | --- |
| Cautious | Every proposal manually approved | Never | Off |
| Supervised | Approve unless confidence > 90% | After 72h window | On (slow) |
| Collaborative | Auto-approve if no critical regressions | After 24h window | On (normal) |
| Autonomous | Auto-approve if gate passes | After 1h window | On (fast) |

Rubber-stamp detection: 10+ fast approvals (< 30s review time) over rolling 20-proposal window suggests the operator should upgrade governance profile.

```rust
pub struct GovernanceManager {
    pool: PgPool,
}

impl GovernanceManager {
    pub async fn get_profile(&self, project_id: &str) -> GovernanceProfile { ... }
    pub async fn set_profile(&self, project_id: &str, profile: GovernanceProfile) { ... }
    pub async fn record_approval(&self, project_id: &str, review_time: Duration) { ... }
    pub async fn check_rubber_stamp(&self, project_id: &str) -> bool { ... }
    pub fn requires_human_approval(&self, profile: GovernanceProfile, confidence: f64) -> bool { ... }
    pub fn verification_window(&self, profile: GovernanceProfile) -> Duration { ... }
}
```

- [ ] **Step 1: Create governance.rs with profile management and rubber-stamp detection**
- [ ] **Step 2: Add CLI command** `autonomic governance set <profile>`
- [ ] **Step 3: Commit**

## Task 3: A/B Testing Framework (FR-025)

**Files:**
- Create: `crates/autonomic-evolution/src/ab_testing.rs`

Test alternative configurations against each other:
1. Create paired worktrees: control (current variant) vs experimental (proposed variant)
2. Run identical probe battery in both
3. Statistical comparison (confidence intervals)
4. Auto-adopt winner if statistically significant (p < 0.05)

```rust
pub struct ABTest {
    pub id: String,
    pub project_id: String,
    pub control_variant: VariantId,
    pub experimental_variant: VariantId,
    pub status: ABTestStatus,
}

pub async fn run_ab_test(
    project_id: &ProjectId,
    control: &VariantId,
    experimental: &VariantId,
    probes: &ProbeRunner,
    pool: &PgPool,
) -> Result<ABTestResult, EvolutionError> { ... }
```

- [ ] **Step 1: Create ab_testing.rs with paired execution and comparison**
- [ ] **Step 2: Commit**

## Task 4: Capability Gap Tracker (FR-026)

**Files:**
- Create: `crates/autonomic-evolution/src/capability_gap.rs`

Track what the system cannot do:
- Gap entries (GAP-001...) with severity, category, context
- `model_bound` gaps: identified as model capability limitations (not evolution targets)
- `scaffold_improvable` gaps: addressable by evolution
- Gaps linked to evolution proposals that address them

```rust
pub struct CapabilityGap {
    pub id: String,
    pub severity: GapSeverity,
    pub category: GapCategory,
    pub description: String,
    pub context: Option<String>,
    pub linked_proposal: Option<String>,
}

pub enum GapCategory {
    ModelBound,           // Not an evolution target
    ScaffoldImprovable,   // Evolution can address this
    Unknown,              // Needs classification
}
```

Auto-detection from experience traces: sessions that fail with specific error patterns create gap entries automatically.

- [ ] **Step 1: Create capability_gap.rs with CRUD and auto-detection**
- [ ] **Step 2: Add CLI command** `autonomic gaps list`
- [ ] **Step 3: Commit**

## Task 5: Integration and Final Verification

- [ ] **Step 1: Wire governance into evolution pipeline** (check profile before auto-deploy)
- [ ] **Step 2: Add remaining CLI commands**
  - `autonomic evolve history` — evolution timeline
  - `autonomic evolve status` — current variant, energy, convergence progress
  - `autonomic governance set <profile>`
  - `autonomic gaps list`
- [ ] **Step 3: Full integration test**

```bash
SQLX_OFFLINE=true cargo check --workspace
SQLX_OFFLINE=true cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check --all
cargo test --workspace
```

- [ ] **Step 4: Tag milestones**

```bash
git tag -a milestone/phase-6 -m "Phase 6: Governance, A/B testing, capability gaps (FR-024 through FR-026)"
git tag -a v0.1.0 -m "Autonomic v0.1.0: All 26 FRs implemented across 6 phases"
```
