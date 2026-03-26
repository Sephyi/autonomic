# Phase 3: Scheduling Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `autonomic-scheduler` crate: TOML-defined jobs with 6 trigger types, priority-based execution, rate budget enforcement, hot-reload, and built-in jobs (daily health, reflection, snapshot, weekly analysis).

**Architecture:** The scheduler runs as a tokio task inside the daemon. It manages a `JobRegistry` loaded from TOML files in `~/.autonomic/jobs/`. Each job has a trigger (cron/once/interval/poll/on_message/webhook), model tier, priority, and prompt template. The scheduler's main loop checks triggers, respects rate budget allocation (15% scheduled), and spawns sessions via `SessionManager`. Hot-reload watches the jobs directory for changes.

**Tech Stack:** Rust 2024 (1.94), croner (cron expressions), tokio, serde/toml, chrono

**Spec:** `docs/architecture/scheduler.md` (38KB, 1,291 lines)

**Prereqs from Phase 1-2:** `SessionManager`, `CostTracker`, `RateBudget`, `BudgetAllocation`, `MemoryStore`, managed hooks

**Key Constraints:**
- XD-001: Scheduler spawns via SessionManager, never directly
- XD-006: RateBudget from autonomic-core, shared with all subsystems
- XD-004: Start with cron trigger, add remaining 5 triggers incrementally

## File Map

```txt
MODIFY: crates/autonomic-scheduler/Cargo.toml         # Add dependencies
CREATE: crates/autonomic-scheduler/src/lib.rs          # Re-exports
CREATE: crates/autonomic-scheduler/src/error.rs        # SchedulerError
CREATE: crates/autonomic-scheduler/src/job.rs          # JobDefinition, Trigger, Priority, ModelTier
CREATE: crates/autonomic-scheduler/src/registry.rs     # JobRegistry: load from TOML, hot-reload
CREATE: crates/autonomic-scheduler/src/executor.rs     # Job execution loop, trigger evaluation
CREATE: crates/autonomic-scheduler/src/budget.rs       # Rate budget enforcement for scheduled jobs
CREATE: crates/autonomic-scheduler/src/builtin.rs      # Built-in job definitions (FR-013)

CREATE: infra/jobs/daily-health.toml                   # Example: daily health check
CREATE: infra/jobs/daily-snapshot.toml                 # Example: daily git snapshot
CREATE: infra/jobs/daily-reflection.toml               # Example: daily experience reflection
CREATE: infra/jobs/weekly-analysis.toml                # Example: weekly evolution analysis
```

## Task 1: Types and Error

**Files:**
- Create: `crates/autonomic-scheduler/src/error.rs`
- Create: `crates/autonomic-scheduler/src/job.rs`

- [ ] **Step 1: Update Cargo.toml**

```toml
[package]
name = "autonomic-scheduler"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Cron-based job scheduling with rate budget awareness"

[dependencies]
autonomic-core = { path = "../autonomic-core" }
chrono = { workspace = true }
croner = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
toml = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }

[lints]
workspace = true
```

- [ ] **Step 2: Create error.rs**

```rust
//! Scheduler error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("invalid job definition: {0}")]
    InvalidJob(String),

    #[error("invalid cron expression: {expr}: {reason}")]
    InvalidCron { expr: String, reason: String },

    #[error("job not found: {0}")]
    NotFound(String),

    #[error("rate budget exhausted for scheduled jobs")]
    BudgetExhausted,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("session error: {0}")]
    Session(String),
}
```

- [ ] **Step 3: Create job.rs with all 6 trigger types**

```rust
//! Job definitions with 6 trigger types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A scheduled job definition loaded from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDefinition {
    /// Unique job name (derived from filename).
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// The trigger that activates this job.
    pub trigger: Trigger,
    /// Claude model tier to use.
    #[serde(default = "default_model")]
    pub model: String,
    /// Job priority.
    #[serde(default)]
    pub priority: Priority,
    /// Prompt template for the session.
    pub prompt: String,
    /// Working directory (project path).
    pub working_dir: Option<String>,
    /// Whether the job is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Maximum cost for a single execution.
    #[serde(default = "default_job_budget")]
    pub max_cost_usd: f64,
}

fn default_model() -> String { "haiku".to_string() }
fn default_true() -> bool { true }
fn default_job_budget() -> f64 { 1.0 }

/// The 6 trigger types (XD-004: cron first, rest in Phase 3.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Trigger {
    /// Recurring schedule via cron expression.
    Cron { expression: String },
    /// One-time execution at a specific timestamp.
    Once { at: DateTime<Utc> },
    /// Execute every N seconds/minutes/hours.
    Interval { seconds: u64 },
    /// Check a condition periodically, execute if true.
    Poll {
        /// Shell command that returns exit 0 if condition is met.
        condition: String,
        /// Check interval in seconds.
        check_interval_seconds: u64,
    },
    /// React to an incoming message/request.
    OnMessage {
        /// Channel to listen on (e.g., "api", "webhook").
        channel: String,
    },
    /// External HTTP webhook trigger.
    Webhook {
        /// Path to register (e.g., "/hooks/deploy").
        path: String,
    },
}

/// Job priority levels with budget thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    /// Always runs regardless of budget utilization.
    Critical,
    /// Defers at 80% budget utilization.
    #[default]
    Normal,
    /// Defers at 60% budget utilization.
    Low,
}

impl Priority {
    /// Budget utilization threshold above which this priority defers.
    pub fn defer_threshold(self) -> f64 {
        match self {
            Self::Critical => 1.0, // Never defers.
            Self::Normal => 0.80,
            Self::Low => 0.60,
        }
    }
}

/// Runtime state for a job.
#[derive(Debug, Clone)]
pub struct JobState {
    pub definition: JobDefinition,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub run_count: u64,
    pub last_error: Option<String>,
}
```

- [ ] **Step 4: Commit**

```bash
git add crates/autonomic-scheduler/
git commit -m "feat(scheduler): add job types with 6 trigger types and priority levels"
```

## Task 2: Job Registry (TOML loading + hot-reload)

**Files:**
- Create: `crates/autonomic-scheduler/src/registry.rs`

- [ ] **Step 1: Create registry.rs**

Load all `.toml` files from `~/.autonomic/jobs/`, parse into `JobDefinition`, watch for changes.

Key methods: `load_all(jobs_dir)`, `reload()`, `get(name)`, `list()`, `add_builtin(jobs)`.

The registry uses `tokio::fs::read_dir` and `notify` (or polling interval) for hot-reload.

- [ ] **Step 2: Commit**

```bash
git add crates/autonomic-scheduler/src/registry.rs
git commit -m "feat(scheduler): add JobRegistry with TOML loading and hot-reload"
```

## Task 3: Executor (main scheduling loop)

**Files:**
- Create: `crates/autonomic-scheduler/src/executor.rs`

- [ ] **Step 1: Create executor.rs**

The scheduling loop:
1. Every 10 seconds, check all registered jobs
2. For each job, evaluate its trigger (cron: check next_run, interval: check elapsed, poll: run condition)
3. Check rate budget allocation (15% for scheduled jobs)
4. Check priority vs budget utilization (defer if over threshold)
5. Spawn session via SessionManager (XD-001)
6. Update job state (last_run, next_run, run_count)
7. On error, log and update last_error

Key: `async fn run_scheduler_loop(registry, session_manager, cost_tracker, shutdown)` — runs until shutdown signal.

- [ ] **Step 2: Commit**

```bash
git add crates/autonomic-scheduler/src/executor.rs
git commit -m "feat(scheduler): add executor loop with trigger evaluation and budget checks"
```

## Task 4: Budget Enforcement

**Files:**
- Create: `crates/autonomic-scheduler/src/budget.rs`

- [ ] **Step 1: Create budget.rs**

Rate budget enforcement for the 15% scheduled allocation:
- `can_schedule(cost_tracker, priority) -> bool`: checks window utilization vs priority threshold
- `scheduled_budget_remaining(cost_tracker, global_budget) -> f64`: 15% of global minus window cost for scheduled subsystem
- Integration with `RateBudget` from autonomic-core (XD-006)

- [ ] **Step 2: Commit**

```bash
git add crates/autonomic-scheduler/src/budget.rs
git commit -m "feat(scheduler): add rate budget enforcement for scheduled jobs (XD-006)"
```

## Task 5: Built-in Jobs (FR-013)

**Files:**
- Create: `crates/autonomic-scheduler/src/builtin.rs`
- Create: `infra/jobs/daily-health.toml`
- Create: `infra/jobs/daily-snapshot.toml`
- Create: `infra/jobs/daily-reflection.toml`
- Create: `infra/jobs/weekly-analysis.toml`

- [ ] **Step 1: Create builtin.rs with 4 default jobs**

```rust
//! Built-in scheduled jobs (FR-013).

use crate::job::{JobDefinition, Priority, Trigger};

pub fn builtin_jobs() -> Vec<JobDefinition> {
    vec![
        JobDefinition {
            name: "daily-health".to_string(),
            description: "Check all projects compile and tests pass".to_string(),
            trigger: Trigger::Cron { expression: "0 6 * * *".to_string() }, // 6 AM daily
            model: "haiku".to_string(),
            priority: Priority::Normal,
            prompt: "Run `cargo check --workspace` and `cargo test --workspace`. Report any failures.".to_string(),
            working_dir: None,
            enabled: true,
            max_cost_usd: 0.50,
        },
        JobDefinition {
            name: "daily-snapshot".to_string(),
            description: "Create daily git snapshot of state directory".to_string(),
            trigger: Trigger::Cron { expression: "0 0 * * *".to_string() }, // Midnight daily
            model: "haiku".to_string(),
            priority: Priority::Low,
            prompt: "Create a daily snapshot tag in the state repository.".to_string(),
            working_dir: None,
            enabled: true,
            max_cost_usd: 0.10,
        },
        JobDefinition {
            name: "daily-reflection".to_string(),
            description: "Review experience traces and capture learnings".to_string(),
            trigger: Trigger::Cron { expression: "0 22 * * *".to_string() }, // 10 PM daily
            model: "haiku".to_string(),
            priority: Priority::Low,
            prompt: "Review today's experience traces. Identify patterns, errors, and learnings. Create memory entries for significant findings.".to_string(),
            working_dir: None,
            enabled: true,
            max_cost_usd: 0.50,
        },
        JobDefinition {
            name: "weekly-analysis".to_string(),
            description: "Analyze weekly metric patterns for evolution".to_string(),
            trigger: Trigger::Cron { expression: "0 8 * * 0".to_string() }, // Sunday 8 AM
            model: "haiku".to_string(),
            priority: Priority::Low,
            prompt: "Analyze this week's session metrics. Identify: most common errors, costliest sessions, model routing effectiveness, and evolution opportunities.".to_string(),
            working_dir: None,
            enabled: true,
            max_cost_usd: 1.00,
        },
    ]
}
```

- [ ] **Step 2: Create TOML job files** (one per built-in job, same content as Rust structs)

- [ ] **Step 3: Commit**

```bash
git add crates/autonomic-scheduler/src/builtin.rs infra/jobs/
git commit -m "feat(scheduler): add built-in jobs (health, snapshot, reflection, analysis) (FR-013)"
```

## Task 6: Integration and lib.rs

- [ ] **Step 1: Update lib.rs**

```rust
//! Cron-based job scheduling with rate budget awareness.

pub mod budget;
pub mod builtin;
pub mod error;
pub mod executor;
pub mod job;
pub mod registry;

pub use error::SchedulerError;
pub use executor::run_scheduler_loop;
pub use job::{JobDefinition, JobState, Priority, Trigger};
pub use registry::JobRegistry;
```

- [ ] **Step 2: Wire scheduler into daemon main.rs** (add as tokio task alongside HTTP server)

- [ ] **Step 3: Verify**

```bash
SQLX_OFFLINE=true cargo check -p autonomic-scheduler
SQLX_OFFLINE=true cargo clippy -p autonomic-scheduler --all-targets -- -D warnings
```

- [ ] **Step 4: Tag milestone**

```bash
git tag -a milestone/phase-3 -m "Phase 3: Scheduling — 6 triggers, budget enforcement, built-in jobs (FR-011 through FR-013)"
```
