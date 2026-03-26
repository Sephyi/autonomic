# Phase 1A: Foundation Layer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the foundation types, database layer, and git-backed state store that all other Phase 1 crates depend on.

**Architecture:** Three crates built bottom-up: `autonomic-core` (zero dependencies on siblings, defines shared types), `autonomic-db` (depends on core, owns PostgreSQL pool and migrations), `autonomic-state` (depends on core + db, owns git-backed config state and file locking). TDD throughout — failing test first, then implementation.

**Tech Stack:** Rust 2024 (1.94), tokio, sqlx (Postgres), gix, figment, serde, thiserror, chrono, ulid, fs4, tempfile, proptest, insta

**Spec:** `docs/architecture/overview.md`, `docs/architecture/state-management.md`, `.claude/specs/cross-document-constraints.md`

**Prereqs already applied:** `[workspace.lints]` in root `Cargo.toml` enables `unsafe_code = "forbid"`, `clippy::disallowed_methods`, `clippy::disallowed_macros`, `clippy::await_holding_invalid_type`, `clippy::large_futures`, `clippy::large_stack_frames` across all crates. Each crate's `Cargo.toml` inherits via `[lints] workspace = true`. No per-file `#![forbid]` or `#![warn]` attributes needed.

## File Map

```txt
CREATE: crates/autonomic-core/src/lib.rs          # Re-exports
CREATE: crates/autonomic-core/src/error.rs         # CoreError enum
CREATE: crates/autonomic-core/src/config.rs        # AutonomicConfig, figment loading
CREATE: crates/autonomic-core/src/types.rs         # SessionId, ProjectId, ModelTier, etc.
CREATE: crates/autonomic-core/src/rate_budget.rs   # RateBudget (XD-006: shared by all subsystems)
CREATE: crates/autonomic-core/Cargo.toml           # MODIFY: add dependencies

CREATE: crates/autonomic-db/src/lib.rs             # Re-exports
CREATE: crates/autonomic-db/src/pool.rs            # create_pool, create_readonly_pool
CREATE: crates/autonomic-db/src/error.rs           # DbError enum
MODIFY: crates/autonomic-db/Cargo.toml             # Add dependencies
MODIFY: infra/migrations/001_initial.sql           # Expand with full Phase 1 schema

CREATE: crates/autonomic-state/src/lib.rs          # Re-exports
CREATE: crates/autonomic-state/src/error.rs        # StateError enum
CREATE: crates/autonomic-state/src/git.rs          # GitManager: init, commit, tag, rollback
CREATE: crates/autonomic-state/src/lock.rs         # StateLock: exclusive file lock
CREATE: crates/autonomic-state/src/atomic.rs       # atomic_write, atomic_toml_update
CREATE: crates/autonomic-state/src/manager.rs      # StateManager: unified entry point
MODIFY: crates/autonomic-state/Cargo.toml          # Add dependencies
```

## Task 1: Core Types and IDs (autonomic-core)

**Files:**
- Modify: `crates/autonomic-core/Cargo.toml`
- Rewrite: `crates/autonomic-core/src/lib.rs`
- Create: `crates/autonomic-core/src/types.rs`

- [ ] **Step 1: Add dependencies to autonomic-core/Cargo.toml**

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
chrono = { workspace = true }
figment = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
sha2 = { workspace = true }
thiserror = { workspace = true }
toml = { workspace = true }
ulid = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
insta = { workspace = true }
tempfile = { workspace = true }
```

- [ ] **Step 2: Create types.rs with core IDs and enums**

```rust
//! Core types shared across all Autonomic crates.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique session identifier. Wraps a ULID for time-sortable uniqueness.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub ulid::Ulid);

impl SessionId {
    pub fn new() -> Self {
        Self(ulid::Ulid::new())
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique project identifier. Stable hash of the canonical project path.
///
/// Uses SHA-256 (via the `sha2` crate) to produce a deterministic, version-stable
/// identifier. This is critical because ProjectIds are persisted in PostgreSQL —
/// `DefaultHasher` is NOT stable across Rust versions and must never be used here.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(pub String);

impl ProjectId {
    /// Create a ProjectId from a project path by SHA-256 hashing it.
    /// The result is a 16-character hex prefix of the full hash.
    pub fn from_path(path: &std::path::Path) -> Self {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(path.to_string_lossy().as_bytes());
        let hash = hasher.finalize();
        // First 8 bytes (16 hex chars) — collision probability negligible for <1M projects
        Self(format!(
            "{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7]
        ))
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Model tier for routing and budget tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelTier {
    Haiku,
    Sonnet,
    Opus,
}

impl fmt::Display for ModelTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Haiku => write!(f, "haiku"),
            Self::Sonnet => write!(f, "sonnet"),
            Self::Opus => write!(f, "opus"),
        }
    }
}

/// Session status for database storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Budget allocation subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Subsystem {
    Interactive,
    Scheduled,
    Evolution,
    Monitoring,
}

impl fmt::Display for Subsystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Interactive => write!(f, "interactive"),
            Self::Scheduled => write!(f, "scheduled"),
            Self::Evolution => write!(f, "evolution"),
            Self::Monitoring => write!(f, "monitoring"),
        }
    }
}
```

- [ ] **Step 3: Update lib.rs to re-export types**

```rust
//! Core types, traits, and configuration for Autonomic.

pub mod types;

pub use types::{ModelTier, ProjectId, SessionId, SessionStatus, Subsystem};
```

- [ ] **Step 4: Run cargo check to verify compilation**

```bash
cargo check -p autonomic-core
```

Expected: clean check.

- [ ] **Step 5: Commit**

```bash
git add crates/autonomic-core/
git commit -m "feat(core): add foundation types (SessionId, ProjectId, ModelTier, SessionStatus)"
```

## Task 2: Core Error Types (autonomic-core)

**Files:**
- Create: `crates/autonomic-core/src/error.rs`
- Modify: `crates/autonomic-core/src/lib.rs`

- [ ] **Step 1: Create error.rs**

```rust
//! Core error types for Autonomic.
//!
//! Each downstream crate defines its own error enum with `#[from]` conversions
//! for CoreError where needed. This crate provides only the shared error cases.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can originate from core operations.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Configuration file not found: {0}")]
    ConfigNotFound(PathBuf),

    #[error("Invalid path: {0}")]
    InvalidPath(PathBuf),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
```

- [ ] **Step 2: Add error module to lib.rs**

Add `pub mod error;` and `pub use error::CoreError;` to `lib.rs`.

- [ ] **Step 3: Run cargo check**

```bash
cargo check -p autonomic-core
```

- [ ] **Step 4: Commit**

```bash
git add crates/autonomic-core/
git commit -m "feat(core): add CoreError enum with thiserror"
```

## Task 3: RateBudget Contract (autonomic-core)

**Files:**
- Create: `crates/autonomic-core/src/rate_budget.rs`
- Modify: `crates/autonomic-core/src/lib.rs`

Per XD-006: Single `RateBudget` contract defined in `autonomic-core`, shared by all subsystems.

- [ ] **Step 1: Write failing tests for RateBudget**

Create `crates/autonomic-core/src/rate_budget.rs` with tests first:

```rust
//! Rate budget contract shared across all Autonomic subsystems (XD-006).
//!
//! The budget tracks cost in USD over a sliding time window. Subsystems
//! check `can_afford()` before starting work that consumes budget.

use crate::types::{ModelTier, Subsystem};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// A single cost entry in the sliding window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEntry {
    pub timestamp: DateTime<Utc>,
    pub cost_usd: f64,
    pub subsystem: Subsystem,
    pub model_tier: ModelTier,
    pub session_id: Option<String>,
}

/// Budget allocation percentages per subsystem.
/// Must sum to 100. Validated at construction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetAllocation {
    /// Percentage for interactive/on-demand sessions (default: 55%)
    pub interactive: f64,
    /// Percentage for scheduled jobs (default: 15%)
    pub scheduled: f64,
    /// Percentage for evolution engine (default: 10%)
    pub evolution: f64,
    /// Percentage for monitoring/health checks (default: 15%)
    pub monitoring: f64,
    /// Emergency reserve, not allocatable (default: 5%)
    pub emergency_reserve: f64,
}

impl Default for BudgetAllocation {
    fn default() -> Self {
        Self {
            interactive: 55.0,
            scheduled: 15.0,
            evolution: 10.0,
            monitoring: 15.0,
            emergency_reserve: 5.0,
        }
    }
}

impl BudgetAllocation {
    /// Validate that allocations sum to 100%.
    pub fn validate(&self) -> Result<(), String> {
        let total =
            self.interactive + self.scheduled + self.evolution + self.monitoring + self.emergency_reserve;
        if (total - 100.0).abs() > 0.01 {
            return Err(format!("Budget allocations sum to {total:.2}%, expected 100%"));
        }
        Ok(())
    }

    /// Get the allocation percentage for a subsystem.
    pub fn percentage_for(&self, subsystem: Subsystem) -> f64 {
        match subsystem {
            Subsystem::Interactive => self.interactive,
            Subsystem::Scheduled => self.scheduled,
            Subsystem::Evolution => self.evolution,
            Subsystem::Monitoring => self.monitoring,
        }
    }
}

/// Rate budget tracker with sliding window.
///
/// Thread-safety: This struct is NOT thread-safe. The caller (typically the daemon)
/// must wrap it in a `tokio::sync::Mutex` or similar.
#[derive(Debug)]
pub struct RateBudget {
    /// Maximum budget in USD for the sliding window.
    global_budget_usd: f64,
    /// Duration of the sliding window.
    window_duration: Duration,
    /// Cost entries within the current window.
    entries: VecDeque<CostEntry>,
    /// Budget allocation percentages.
    allocation: BudgetAllocation,
}

impl RateBudget {
    /// Create a new RateBudget with the given ceiling and 5-hour window.
    pub fn new(global_budget_usd: f64, allocation: BudgetAllocation) -> Self {
        Self {
            global_budget_usd,
            window_duration: Duration::hours(5),
            entries: VecDeque::new(),
            allocation,
        }
    }

    /// Create with a custom window duration (for testing).
    pub fn with_window(global_budget_usd: f64, window: Duration, allocation: BudgetAllocation) -> Self {
        Self {
            global_budget_usd,
            window_duration: window,
            entries: VecDeque::new(),
            allocation,
        }
    }

    /// Record a cost entry.
    pub fn record(&mut self, entry: CostEntry) {
        self.entries.push_back(entry);
        self.prune();
    }

    /// Check whether a subsystem can afford the estimated cost.
    pub fn can_afford(&mut self, subsystem: Subsystem, estimated_cost_usd: f64) -> bool {
        self.prune();
        let subsystem_budget = self.global_budget_usd * (self.allocation.percentage_for(subsystem) / 100.0);
        let subsystem_used: f64 = self
            .entries
            .iter()
            .filter(|e| e.subsystem == subsystem)
            .map(|e| e.cost_usd)
            .sum();
        subsystem_used + estimated_cost_usd <= subsystem_budget
    }

    /// Total cost in the current window across all subsystems.
    pub fn window_total(&mut self) -> f64 {
        self.prune();
        self.entries.iter().map(|e| e.cost_usd).sum()
    }

    /// Remaining budget in the current window.
    pub fn remaining(&mut self) -> f64 {
        self.global_budget_usd - self.window_total()
    }

    /// Usage as a percentage (0.0 to 100.0).
    pub fn usage_percent(&mut self) -> f64 {
        if self.global_budget_usd <= 0.0 {
            return 100.0;
        }
        (self.window_total() / self.global_budget_usd) * 100.0
    }

    /// Remove entries older than the window.
    fn prune(&mut self) {
        let cutoff = Utc::now() - self.window_duration;
        while self
            .entries
            .front()
            .is_some_and(|e| e.timestamp < cutoff)
        {
            self.entries.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(subsystem: Subsystem, cost: f64, age_minutes: i64) -> CostEntry {
        CostEntry {
            timestamp: Utc::now() - Duration::minutes(age_minutes),
            cost_usd: cost,
            subsystem,
            model_tier: ModelTier::Sonnet,
            session_id: None,
        }
    }

    #[test]
    fn default_allocation_sums_to_100() {
        let alloc = BudgetAllocation::default();
        alloc.validate().expect("default allocation should be valid");
    }

    #[test]
    fn invalid_allocation_rejected() {
        let alloc = BudgetAllocation {
            interactive: 50.0,
            scheduled: 50.0,
            evolution: 50.0,
            monitoring: 50.0,
            emergency_reserve: 50.0,
        };
        assert!(alloc.validate().is_err());
    }

    #[test]
    fn empty_budget_can_afford() {
        let mut budget = RateBudget::new(10.0, BudgetAllocation::default());
        // Interactive gets 55% of $10 = $5.50
        assert!(budget.can_afford(Subsystem::Interactive, 5.0));
    }

    #[test]
    fn exhausted_subsystem_cannot_afford() {
        let mut budget = RateBudget::new(10.0, BudgetAllocation::default());
        // Interactive gets 55% of $10 = $5.50
        budget.record(make_entry(Subsystem::Interactive, 5.50, 1));
        assert!(!budget.can_afford(Subsystem::Interactive, 0.01));
    }

    #[test]
    fn other_subsystem_unaffected() {
        let mut budget = RateBudget::new(10.0, BudgetAllocation::default());
        budget.record(make_entry(Subsystem::Interactive, 5.50, 1));
        // Scheduled gets 15% of $10 = $1.50
        assert!(budget.can_afford(Subsystem::Scheduled, 1.0));
    }

    #[test]
    fn old_entries_pruned() {
        let mut budget = RateBudget::with_window(
            10.0,
            Duration::hours(1),
            BudgetAllocation::default(),
        );
        // Entry from 2 hours ago should be pruned
        budget.record(make_entry(Subsystem::Interactive, 5.50, 120));
        assert!(budget.can_afford(Subsystem::Interactive, 5.0));
    }

    #[test]
    fn usage_percent_correct() {
        let mut budget = RateBudget::new(100.0, BudgetAllocation::default());
        budget.record(make_entry(Subsystem::Interactive, 25.0, 1));
        budget.record(make_entry(Subsystem::Scheduled, 10.0, 1));
        assert!((budget.usage_percent() - 35.0).abs() < 0.01);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    prop_compose! {
        fn arb_cost()(cost in 0.001f64..100.0) -> f64 {
            cost
        }
    }

    prop_compose! {
        fn arb_budget()(budget in 1.0f64..1000.0) -> f64 {
            budget
        }
    }

    proptest! {
        #[test]
        fn usage_percent_bounded(budget_usd in 1.0f64..1000.0, cost in 0.001f64..500.0) {
            let mut budget = RateBudget::new(budget_usd, BudgetAllocation::default());
            let entry = CostEntry {
                timestamp: Utc::now(),
                cost_usd: cost,
                subsystem: Subsystem::Interactive,
                model_tier: ModelTier::Sonnet,
                session_id: None,
            };
            budget.record(entry);
            let pct = budget.usage_percent();
            prop_assert!(pct >= 0.0, "usage_percent must be >= 0, got {}", pct);
            // Can exceed 100% if cost > budget — that's valid (overspend detection)
        }

        #[test]
        fn can_afford_monotonically_decreasing(
            budget_usd in 10.0f64..100.0,
            costs in prop::collection::vec(0.1f64..5.0, 1..10),
        ) {
            let mut budget = RateBudget::new(budget_usd, BudgetAllocation::default());
            let mut prev_remaining = budget.remaining();

            for cost in costs {
                let entry = CostEntry {
                    timestamp: Utc::now(),
                    cost_usd: cost,
                    subsystem: Subsystem::Interactive,
                    model_tier: ModelTier::Sonnet,
                    session_id: None,
                };
                budget.record(entry);
                let current_remaining = budget.remaining();
                prop_assert!(
                    current_remaining <= prev_remaining,
                    "remaining budget must not increase: {} -> {}",
                    prev_remaining,
                    current_remaining,
                );
                prev_remaining = current_remaining;
            }
        }

        #[test]
        fn allocation_percentages_consistent(
            interactive in 10.0f64..40.0,
            scheduled in 5.0f64..20.0,
            evolution in 5.0f64..20.0,
        ) {
            // Force sum to 100 by computing monitoring and reserve from remainder
            let reserve = 5.0;
            let monitoring = 100.0 - interactive - scheduled - evolution - reserve;
            prop_assume!(monitoring > 0.0);

            let alloc = BudgetAllocation {
                interactive,
                scheduled,
                evolution,
                monitoring,
                emergency_reserve: reserve,
            };
            prop_assert!(alloc.validate().is_ok());
            // Each subsystem's percentage_for matches its field
            prop_assert!((alloc.percentage_for(Subsystem::Interactive) - interactive).abs() < f64::EPSILON);
            prop_assert!((alloc.percentage_for(Subsystem::Scheduled) - scheduled).abs() < f64::EPSILON);
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

```bash
cargo test -p autonomic-core
```

Expected: all tests pass (the implementation is in the same file).

- [ ] **Step 3: Add rate_budget module to lib.rs**

Add `pub mod rate_budget;` and `pub use rate_budget::{BudgetAllocation, CostEntry, RateBudget};` to `lib.rs`.

- [ ] **Step 4: Commit**

```bash
git add crates/autonomic-core/
git commit -m "feat(core): add RateBudget with sliding window and subsystem allocations (XD-006)"
```

## Task 4: Configuration Loading (autonomic-core)

**Files:**
- Create: `crates/autonomic-core/src/config.rs`
- Modify: `crates/autonomic-core/src/lib.rs`

The daemon reads config from `~/.autonomic/config.toml` with figment (TOML + env overrides). Secrets come from `~/.autonomic/secrets.toml` per XD-011.

- [ ] **Step 1: Create config.rs**

```rust
//! Configuration types and loading for Autonomic.
//!
//! Config is loaded via figment: TOML file + environment variable overrides.
//! Secrets (API keys, database URL) are in a separate secrets.toml (XD-011).

use figment::{providers::Toml, Figment};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::CoreError;
use crate::rate_budget::BudgetAllocation;

/// Top-level Autonomic configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomicConfig {
    /// Base directory for Autonomic state. Default: ~/.autonomic
    #[serde(default = "default_state_dir")]
    pub state_dir: PathBuf,

    /// HTTP API configuration.
    #[serde(default)]
    pub api: ApiConfig,

    /// Rate budget configuration.
    #[serde(default)]
    pub budget: BudgetConfig,

    /// Session defaults.
    #[serde(default)]
    pub session: SessionDefaults,
}

fn default_state_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".autonomic")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    /// Localhost port for the daemon HTTP API.
    #[serde(default = "default_port")]
    pub port: u16,

    /// Bind address.
    #[serde(default = "default_bind")]
    pub bind: String,
}

fn default_port() -> u16 {
    7700
}

fn default_bind() -> String {
    "127.0.0.1".into()
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            port: default_port(),
            bind: default_bind(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetConfig {
    /// Maximum budget in USD for the 5-hour sliding window.
    #[serde(default = "default_global_budget")]
    pub global_budget_usd: f64,

    /// Budget allocation percentages.
    #[serde(default)]
    pub allocation: BudgetAllocation,
}

fn default_global_budget() -> f64 {
    50.0
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            global_budget_usd: default_global_budget(),
            allocation: BudgetAllocation::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDefaults {
    /// Default timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,

    /// Default cost budget per session in USD.
    #[serde(default = "default_cost_budget")]
    pub cost_budget_usd: f64,

    /// Maximum concurrent sessions.
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent: usize,
}

fn default_timeout() -> u64 {
    1800
}

fn default_cost_budget() -> f64 {
    5.0
}

fn default_max_concurrent() -> usize {
    5
}

impl Default for SessionDefaults {
    fn default() -> Self {
        Self {
            timeout_seconds: default_timeout(),
            cost_budget_usd: default_cost_budget(),
            max_concurrent: default_max_concurrent(),
        }
    }
}

/// Secrets loaded from secrets.toml (gitignored, per XD-011).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsConfig {
    /// Database connection details.
    #[serde(default)]
    pub database: DatabaseSecrets,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSecrets {
    /// PostgreSQL connection URL.
    #[serde(default = "default_database_url")]
    pub url: String,
}

fn default_database_url() -> String {
    "postgresql://autonomic:autonomic_dev_password@localhost:5432/autonomic".into()
}

impl Default for DatabaseSecrets {
    fn default() -> Self {
        Self {
            url: default_database_url(),
        }
    }
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self {
            database: DatabaseSecrets::default(),
        }
    }
}

/// Load the main config from a TOML file.
pub fn load_config(config_path: &Path) -> Result<AutonomicConfig, CoreError> {
    if !config_path.exists() {
        return Ok(AutonomicConfig {
            state_dir: config_path
                .parent()
                .unwrap_or(Path::new("."))
                .to_path_buf(),
            api: ApiConfig::default(),
            budget: BudgetConfig::default(),
            session: SessionDefaults::default(),
        });
    }

    let config: AutonomicConfig = Figment::new()
        .merge(Toml::file(config_path))
        .extract()
        .map_err(|e| CoreError::Config(e.to_string()))?;

    Ok(config)
}

/// Load secrets from secrets.toml (per XD-011: secrets from file, not env).
pub fn load_secrets(secrets_path: &Path) -> Result<SecretsConfig, CoreError> {
    if !secrets_path.exists() {
        return Ok(SecretsConfig::default());
    }

    let secrets: SecretsConfig = Figment::new()
        .merge(Toml::file(secrets_path))
        .extract()
        .map_err(|e| CoreError::Config(e.to_string()))?;

    Ok(secrets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn default_config_when_file_missing() {
        let config = load_config(Path::new("/nonexistent/config.toml")).unwrap();
        assert_eq!(config.api.port, 7700);
        assert_eq!(config.session.timeout_seconds, 1800);
    }

    #[test]
    fn load_config_from_toml() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(
            f,
            r#"
[api]
port = 8800

[session]
timeout_seconds = 900
"#
        )
        .unwrap();

        let config = load_config(f.path()).unwrap();
        assert_eq!(config.api.port, 8800);
        assert_eq!(config.session.timeout_seconds, 900);
    }

    #[test]
    fn default_secrets_when_file_missing() {
        let secrets = load_secrets(Path::new("/nonexistent/secrets.toml")).unwrap();
        assert!(secrets.database.url.contains("localhost:5432"));
    }

    #[test]
    fn default_config_snapshot() {
        let config = load_config(Path::new("/nonexistent/config.toml")).unwrap();
        // Snapshot the serialized TOML to catch unintended default changes
        let serialized = toml::to_string_pretty(&config).unwrap();
        insta::assert_snapshot!("default_config", serialized);
    }
}
```

**Note:** The insta snapshot test will create a `snapshots/` directory on first run with the expected output. Subsequent runs verify the defaults haven't changed unintentionally. Run `cargo insta review` to approve new snapshots.

- [ ] **Step 2: Add `dirs` and `sha2` to workspace dependencies**

In root `Cargo.toml`, add to `[workspace.dependencies]`:

```toml
dirs = "6"
sha2 = "0.10"
```

In `crates/autonomic-core/Cargo.toml`, add to `[dependencies]`:

```toml
dirs = { workspace = true }
```

- [ ] **Step 3: Add config module to lib.rs**

Add `pub mod config;` and `pub use config::{AutonomicConfig, SecretsConfig, load_config, load_secrets};` to `lib.rs`.

- [ ] **Step 4: Run tests**

```bash
cargo test -p autonomic-core
```

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/autonomic-core/
git commit -m "feat(core): add config loading via figment with secrets support (XD-011)"
```

## Task 5: Database Pool and Migrations (autonomic-db)

**Files:**
- Modify: `crates/autonomic-db/Cargo.toml`
- Rewrite: `crates/autonomic-db/src/lib.rs`
- Create: `crates/autonomic-db/src/pool.rs`
- Create: `crates/autonomic-db/src/error.rs`
- Modify: `infra/migrations/001_initial.sql`

- [ ] **Step 1: Update autonomic-db/Cargo.toml**

```toml
[package]
name = "autonomic-db"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Database migrations, schemas, and compile-time checked queries for Autonomic"

[dependencies]
autonomic-core = { path = "../autonomic-core" }
sqlx = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
```

- [ ] **Step 2: Create error.rs**

```rust
//! Database error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Database connection failed: {0}")]
    Connection(#[from] sqlx::Error),

    #[error("Migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("Database not available — is Postgres running? (infra/compose.yaml)")]
    Unavailable,
}
```

- [ ] **Step 3: Create pool.rs**

```rust
//! PostgreSQL connection pool management.

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::info;

use crate::error::DbError;

/// Create a connection pool for the daemon (read-write, higher connection limit).
pub async fn create_pool(database_url: &str) -> Result<PgPool, DbError> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .map_err(|e| {
            if e.to_string().contains("Connection refused") {
                return DbError::Unavailable;
            }
            DbError::Connection(e)
        })?;

    info!("Database pool created (max_connections=10)");
    Ok(pool)
}

/// Create a read-only pool for the CLI (lower connection limit).
pub async fn create_readonly_pool(database_url: &str) -> Result<PgPool, DbError> {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(database_url)
        .await
        .map_err(|e| {
            if e.to_string().contains("Connection refused") {
                return DbError::Unavailable;
            }
            DbError::Connection(e)
        })?;

    Ok(pool)
}

/// Run all pending migrations. Call on daemon startup before any other DB access.
pub async fn run_migrations(pool: &PgPool) -> Result<(), DbError> {
    sqlx::migrate!("../../infra/migrations")
        .run(pool)
        .await?;

    info!("Database migrations applied successfully");
    Ok(())
}
```

**Note:** The `sqlx::migrate!` macro uses a compile-time path relative to the crate root. The path `../../infra/migrations` points from `crates/autonomic-db/` to the project root's `infra/migrations/`.

- [ ] **Step 4: Update lib.rs**

```rust
//! Database migrations, schemas, and compile-time checked queries for Autonomic.

pub mod error;
pub mod pool;

pub use error::DbError;
pub use pool::{create_pool, create_readonly_pool, run_migrations};

// Re-export sqlx::PgPool for downstream crates
pub use sqlx::PgPool;
```

- [ ] **Step 5: Expand the initial migration**

Update `infra/migrations/001_initial.sql` to include the full Phase 1 schema from the state-management architecture doc. Add the `sessions`, `metrics`, `rate_budget`, `budget_allocations`, `experience_traces`, `memory_entries`, and `model_performance` tables. Also add the `projects` table for FR-002 and the `memory_md_sync` table per XD-009.

The migration file should contain all tables from the existing `infra/migrations/001_initial.sql` plus these additions:

```sql
-- Projects registry (FR-002)
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    language TEXT,
    last_activity TIMESTAMPTZ,
    last_session_cost_usd DOUBLE PRECISION,
    hook_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Metrics time-series (FR-005)
CREATE TABLE metrics (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL,
    session_id TEXT REFERENCES sessions(id),
    metric_name TEXT NOT NULL,
    metric_value DOUBLE PRECISION NOT NULL,
    labels JSONB,
    UNIQUE(timestamp, session_id, metric_name)
);

CREATE INDEX idx_metrics_name_time ON metrics(metric_name, timestamp);
CREATE INDEX idx_metrics_session ON metrics(session_id);

-- Rate budget tracking
CREATE TABLE rate_budget (
    id BIGSERIAL PRIMARY KEY,
    window_start TIMESTAMPTZ NOT NULL,
    window_end TIMESTAMPTZ NOT NULL,
    model_tier TEXT NOT NULL,
    tokens_used BIGINT NOT NULL DEFAULT 0,
    tokens_limit BIGINT NOT NULL,
    requests_used BIGINT NOT NULL DEFAULT 0,
    requests_limit BIGINT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_rate_budget_window ON rate_budget(model_tier, window_start);

-- Memory sync metadata (XD-009)
CREATE TABLE memory_md_sync (
    project_id TEXT PRIMARY KEY,
    last_hash TEXT NOT NULL,
    last_sync TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

Append these to the existing `001_initial.sql` file (which already has `memory_entries`, `sessions`, `experience_traces`, and `model_performance`).

- [ ] **Step 6: Run cargo check (note: sqlx migrations need DATABASE_URL for compile-time checks)**

For development without a running database, set `SQLX_OFFLINE=true`:

```bash
SQLX_OFFLINE=true cargo check -p autonomic-db
```

If you have Postgres running (`mise run infra:up`), you can run with the real database:

```bash
DATABASE_URL="postgresql://autonomic:autonomic_dev_password@localhost:5432/autonomic" cargo check -p autonomic-db
```

- [ ] **Step 7: Commit**

```bash
git add crates/autonomic-db/ infra/migrations/
git commit -m "feat(db): add PostgreSQL pool, migrations, and error types"
```

## Task 6: State Lock (autonomic-state)

**Files:**
- Modify: `crates/autonomic-state/Cargo.toml`
- Create: `crates/autonomic-state/src/error.rs`
- Create: `crates/autonomic-state/src/lock.rs`

- [ ] **Step 1: Update autonomic-state/Cargo.toml**

```toml
[package]
name = "autonomic-state"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Git-backed state management with point-in-time recovery"

[dependencies]
autonomic-core = { path = "../autonomic-core" }
autonomic-db = { path = "../autonomic-db" }
chrono = { workspace = true }
fs4 = { workspace = true }
gix = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
sqlx = { workspace = true }
tempfile = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
toml = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
tempfile = { workspace = true }
tokio = { workspace = true, features = ["test-util", "macros"] }
```

- [ ] **Step 2: Create error.rs**

```rust
//! State management error types.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("Another Autonomic instance is already running")]
    AlreadyRunning,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Git error: {0}")]
    Git(String),

    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid path: {0}")]
    InvalidPath(PathBuf),

    #[error("State directory not found: {0}")]
    StateDirNotFound(PathBuf),

    #[error("Invalid rollback target: {0}")]
    InvalidTarget(String),

    #[error("Temp file persist error: {0}")]
    Persist(#[from] tempfile::PersistError),

    #[error("Path strip prefix error: {0}")]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}
```

- [ ] **Step 3: Create lock.rs with tests**

```rust
//! Exclusive file lock for single-writer enforcement.
//!
//! The Autonomic daemon is the sole writer to ~/.autonomic/.
//! This lock prevents multiple daemon instances from corrupting state.

use fs4::fs_std::FileExt;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use crate::error::StateError;

/// Exclusive file lock on ~/.autonomic/.lock.
/// Held for the lifetime of the daemon process.
/// Dropped automatically when the struct goes out of scope.
pub struct StateLock {
    _file: std::fs::File,
}

impl StateLock {
    /// Acquire an exclusive lock on the state directory.
    /// Returns `StateError::AlreadyRunning` if another instance holds the lock.
    pub fn acquire(state_dir: &Path) -> Result<Self, StateError> {
        let lock_path = state_dir.join(".lock");
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&lock_path)?;

        file.try_lock_exclusive()
            .map_err(|_| StateError::AlreadyRunning)?;

        // Write PID for diagnostics
        writeln!(&file, "{}", std::process::id())?;

        Ok(StateLock { _file: file })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn acquire_lock_succeeds() {
        let dir = TempDir::new().unwrap();
        let _lock = StateLock::acquire(dir.path()).expect("should acquire lock");
    }

    #[test]
    fn second_lock_fails() {
        let dir = TempDir::new().unwrap();
        let _lock1 = StateLock::acquire(dir.path()).expect("first lock should succeed");
        let result = StateLock::acquire(dir.path());
        assert!(
            matches!(result, Err(StateError::AlreadyRunning)),
            "second lock should fail with AlreadyRunning"
        );
    }

    #[test]
    fn lock_released_on_drop() {
        let dir = TempDir::new().unwrap();
        {
            let _lock = StateLock::acquire(dir.path()).unwrap();
        }
        // Lock should be released now
        let _lock2 = StateLock::acquire(dir.path()).expect("should acquire after drop");
    }
}
```

- [ ] **Step 4: Update lib.rs**

```rust
//! Git-backed state management with point-in-time recovery.

pub mod error;
pub mod lock;

pub use error::StateError;
pub use lock::StateLock;
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p autonomic-state
```

- [ ] **Step 6: Commit**

```bash
git add crates/autonomic-state/
git commit -m "feat(state): add StateLock with exclusive file locking"
```

## Task 7: Git Manager (autonomic-state)

**Files:**
- Create: `crates/autonomic-state/src/git.rs`
- Modify: `crates/autonomic-state/src/lib.rs`

This is the most complex task in 1A. The GitManager wraps gix for init, commit, tag, and rollback operations per the state-management architecture spec.

**Important:** gix has a complex API. The implementation should use `gix::open()` and `gix::init()` for repository operations. For commit operations, we use gix's index manipulation and commit APIs. Read the gix docs via Context7 if needed.

- [ ] **Step 1: Create git.rs with the GitManager struct and init**

```rust
//! Git-backed state management using gix (pure Rust).
//!
//! Every config file mutation is auto-committed. Tags provide named
//! recovery points. Rollback restores tracked files without touching
//! gitignored content (XD-008).

use gix::bstr::BString;
use std::path::{Path, PathBuf};
use tracing::info;

use crate::error::StateError;

/// Commit categories embedded in commit messages for filtering.
#[derive(Debug, Clone, Copy)]
pub enum CommitKind {
    Config,
    Evolution,
    Schedule,
    Snapshot,
    Migration,
    Rollback,
}

impl CommitKind {
    fn prefix(self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Evolution => "evolution",
            Self::Schedule => "schedule",
            Self::Snapshot => "snapshot",
            Self::Migration => "migration",
            Self::Rollback => "rollback",
        }
    }
}

/// Manages the git repository at ~/.autonomic/.
pub struct GitManager {
    /// Path to the state directory (working directory of the git repo).
    root: PathBuf,
}

impl GitManager {
    /// Open or initialize the git repository at the given path.
    /// Creates the directory and initializes git if it doesn't exist.
    pub fn open_or_init(state_dir: &Path) -> Result<Self, StateError> {
        if !state_dir.exists() {
            tokio::fs::create_dir_all(state_dir);
            // Note: this is called during sync init, use std::fs
            std::fs::create_dir_all(state_dir)?;
        }

        let git_dir = state_dir.join(".git");
        if !git_dir.exists() {
            // Initialize a new repository
            let _repo = gix::init(state_dir).map_err(|e| StateError::Git(e.to_string()))?;
            info!(path = %state_dir.display(), "Initialized new state repository");

            // Create .gitignore
            let gitignore_content = "logs/\nbackups/\n*.tmp\nsecrets.toml\ndaemon-heartbeat\nstate/sessions/*.pid\n.lock\n";
            std::fs::write(state_dir.join(".gitignore"), gitignore_content)?;

            // Create initial commit
            let manager = Self {
                root: state_dir.to_path_buf(),
            };
            manager.commit_all(CommitKind::Config, "initialize state repository")?;

            Ok(manager)
        } else {
            Ok(Self {
                root: state_dir.to_path_buf(),
            })
        }
    }

    /// Stage all changes and create a commit.
    pub fn commit_all(&self, kind: CommitKind, message: &str) -> Result<String, StateError> {
        let repo = gix::open(&self.root).map_err(|e| StateError::Git(e.to_string()))?;
        let full_message = format!("{}: {}", kind.prefix(), message);

        // Use gix to stage and commit
        // For simplicity in Phase 1, we shell out to git for the commit operation.
        // gix's index/commit API is complex and we want correctness first.
        // TODO: Replace with pure gix calls in Phase 2.
        let output = std::process::Command::new("git")
            .args(["add", "-A"])
            .current_dir(&self.root)
            .output()?;

        if !output.status.success() {
            return Err(StateError::Git(format!(
                "git add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Check if there's anything to commit
        let status_output = std::process::Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.root)
            .output()?;

        if status_output.stdout.is_empty() {
            // Nothing to commit
            return Ok(String::new());
        }

        let output = std::process::Command::new("git")
            .args([
                "commit",
                "-m",
                &full_message,
                "--author",
                "autonomic <autonomic@localhost>",
            ])
            .current_dir(&self.root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // "nothing to commit" is not an error
            if stderr.contains("nothing to commit") {
                return Ok(String::new());
            }
            return Err(StateError::Git(format!("git commit failed: {stderr}")));
        }

        // Get the commit hash
        let hash_output = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.root)
            .output()?;

        let hash = String::from_utf8_lossy(&hash_output.stdout).trim().to_string();
        Ok(hash)
    }

    /// Create an annotated tag at HEAD.
    pub fn create_tag(&self, tag_name: &str, message: &str) -> Result<(), StateError> {
        let output = std::process::Command::new("git")
            .args(["tag", "-a", tag_name, "-m", message])
            .current_dir(&self.root)
            .output()?;

        if !output.status.success() {
            return Err(StateError::Git(format!(
                "git tag failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        info!(tag = tag_name, "Created tag");
        Ok(())
    }

    /// List all tags matching a pattern.
    pub fn list_tags(&self, pattern: Option<&str>) -> Result<Vec<String>, StateError> {
        let mut args = vec!["tag", "-l"];
        if let Some(p) = pattern {
            args.push(p);
        }

        let output = std::process::Command::new("git")
            .args(&args)
            .current_dir(&self.root)
            .output()?;

        let tags: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect();

        Ok(tags)
    }

    /// Rollback to a specific tag or commit.
    /// Creates a pre-rollback snapshot tag first.
    /// Uses `git checkout` on tracked files only (XD-008: preserves gitignored files).
    pub fn rollback(&self, target: &str) -> Result<(), StateError> {
        // Verify target exists
        let verify = std::process::Command::new("git")
            .args(["rev-parse", "--verify", target])
            .current_dir(&self.root)
            .output()?;

        if !verify.status.success() {
            return Err(StateError::InvalidTarget(target.to_string()));
        }

        // Create pre-rollback safety snapshot
        let now = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
        self.create_tag(
            &format!("pre-rollback/{now}"),
            &format!("State before rollback to {target}"),
        )?;

        // Checkout tracked files from target (XD-008: no git clean)
        let output = std::process::Command::new("git")
            .args(["checkout", target, "--", "."])
            .current_dir(&self.root)
            .output()?;

        if !output.status.success() {
            return Err(StateError::Git(format!(
                "git checkout failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Forward commit recording the rollback
        self.commit_all(CommitKind::Rollback, &format!("restored state to {target}"))?;

        info!(target = target, "Rolled back state");
        Ok(())
    }

    /// Create a daily snapshot tag.
    pub fn create_daily_snapshot(&self) -> Result<String, StateError> {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let tag_name = format!("daily/{today}");

        self.commit_all(CommitKind::Snapshot, &format!("daily snapshot {today}"))?;
        self.create_tag(&tag_name, &format!("Daily snapshot {today}"))?;

        Ok(tag_name)
    }

    /// Get the repository root path.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, GitManager) {
        let dir = TempDir::new().unwrap();
        let manager = GitManager::open_or_init(dir.path()).unwrap();
        (dir, manager)
    }

    #[test]
    fn init_creates_git_repo() {
        let (dir, _manager) = setup();
        assert!(dir.path().join(".git").exists());
        assert!(dir.path().join(".gitignore").exists());
    }

    #[test]
    fn open_existing_repo() {
        let (dir, _manager1) = setup();
        let _manager2 = GitManager::open_or_init(dir.path()).unwrap();
    }

    #[test]
    fn commit_and_tag() {
        let (dir, manager) = setup();

        // Write a file
        std::fs::write(dir.path().join("test.toml"), "[test]\nkey = \"value\"").unwrap();

        // Commit
        let hash = manager.commit_all(CommitKind::Config, "add test config").unwrap();
        assert!(!hash.is_empty());

        // Tag
        manager.create_tag("test/v1", "Test tag").unwrap();
        let tags = manager.list_tags(Some("test/*")).unwrap();
        assert!(tags.contains(&"test/v1".to_string()));
    }

    #[test]
    fn rollback_preserves_gitignored() {
        let (dir, manager) = setup();

        // Create a tracked file and commit
        std::fs::write(dir.path().join("config.toml"), "v1").unwrap();
        manager.commit_all(CommitKind::Config, "v1 config").unwrap();
        manager.create_tag("v1", "Version 1").unwrap();

        // Modify tracked file and create gitignored file
        std::fs::write(dir.path().join("config.toml"), "v2").unwrap();
        std::fs::write(dir.path().join("secrets.toml"), "secret data").unwrap();
        manager.commit_all(CommitKind::Config, "v2 config").unwrap();

        // Rollback to v1
        manager.rollback("v1").unwrap();

        // Tracked file should be restored
        let content = std::fs::read_to_string(dir.path().join("config.toml")).unwrap();
        assert_eq!(content, "v1");

        // Gitignored file should survive (XD-008)
        assert!(dir.path().join("secrets.toml").exists());
    }

    #[test]
    fn rollback_invalid_target() {
        let (_dir, manager) = setup();
        let result = manager.rollback("nonexistent-tag");
        assert!(matches!(result, Err(StateError::InvalidTarget(_))));
    }
}
```

**Note on gix vs git CLI**: The initial implementation uses `std::process::Command` for git operations because gix's index/commit API is complex and we prioritize correctness. This is explicitly marked with a TODO to migrate to pure gix in a later phase. The clippy.toml disallows `std::process::Command::new` — add a `#[allow(clippy::disallowed_methods)]` with a `// SAFETY:` comment explaining this is intentional for the git subprocess (the clippy rule targets `tokio::process::Command` preference, but git operations here are synchronous by design since they operate on the state dir, not on agent sessions).

**Alternative**: If clippy blocks this, wrap the git calls in `tokio::task::spawn_blocking` and use `tokio::process::Command` instead.

- [ ] **Step 2: Add git module to lib.rs**

```rust
pub mod git;
pub use git::{CommitKind, GitManager};
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p autonomic-state
```

- [ ] **Step 4: Commit**

```bash
git add crates/autonomic-state/
git commit -m "feat(state): add GitManager with init, commit, tag, rollback (XD-008)"
```

## Task 8: Atomic File Operations (autonomic-state)

**Files:**
- Create: `crates/autonomic-state/src/atomic.rs`
- Modify: `crates/autonomic-state/src/lib.rs`

- [ ] **Step 1: Create atomic.rs**

```rust
//! Atomic file write operations.
//!
//! Every state mutation follows: write tmp → fsync → rename → git commit.
//! This prevents partial writes on crash.

use serde::{de::DeserializeOwned, Serialize};
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

use crate::error::StateError;
use crate::git::{CommitKind, GitManager};

/// Atomically write content to a file, then commit to git.
pub fn atomic_write_and_commit(
    git: &GitManager,
    target: &Path,
    content: &[u8],
    kind: CommitKind,
    message: &str,
) -> Result<String, StateError> {
    // Ensure parent directory exists
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write to temp file in the same directory (ensures same filesystem for atomic rename)
    let parent = target.parent().ok_or_else(|| StateError::InvalidPath(target.to_path_buf()))?;
    let mut tmp = NamedTempFile::new_in(parent)?;
    tmp.write_all(content)?;

    // fsync to ensure durability
    tmp.as_file().sync_all()?;

    // Atomic rename
    tmp.persist(target)?;

    // Git commit
    git.commit_all(kind, message)
}

/// Atomically read-modify-write a TOML config file, then commit.
pub fn atomic_toml_update<T: Serialize + DeserializeOwned>(
    git: &GitManager,
    path: &Path,
    kind: CommitKind,
    message: &str,
    mutate: impl FnOnce(&mut T) -> Result<(), StateError>,
) -> Result<String, StateError> {
    let content = std::fs::read_to_string(path)?;
    let mut value: T = toml::from_str(&content)?;
    mutate(&mut value)?;
    let new_content = toml::to_string_pretty(&value)?;
    atomic_write_and_commit(git, path, new_content.as_bytes(), kind, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::GitManager;
    use serde::{Deserialize, Serialize};
    use tempfile::TempDir;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestConfig {
        name: String,
        value: i32,
    }

    fn setup() -> (TempDir, GitManager) {
        let dir = TempDir::new().unwrap();
        let git = GitManager::open_or_init(dir.path()).unwrap();
        (dir, git)
    }

    #[test]
    fn atomic_write_creates_file_and_commits() {
        let (dir, git) = setup();
        let target = dir.path().join("test.txt");

        let hash = atomic_write_and_commit(
            &git,
            &target,
            b"hello world",
            CommitKind::Config,
            "add test file",
        )
        .unwrap();

        assert!(target.exists());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "hello world");
        assert!(!hash.is_empty());
    }

    #[test]
    fn atomic_toml_update_modifies_and_commits() {
        let (dir, git) = setup();
        let target = dir.path().join("config.toml");

        // Create initial file
        let initial = TestConfig {
            name: "test".into(),
            value: 1,
        };
        std::fs::write(&target, toml::to_string_pretty(&initial).unwrap()).unwrap();
        git.commit_all(CommitKind::Config, "initial").unwrap();

        // Update atomically
        atomic_toml_update::<TestConfig>(
            &git,
            &target,
            CommitKind::Config,
            "update value",
            |config| {
                config.value = 42;
                Ok(())
            },
        )
        .unwrap();

        // Verify
        let content = std::fs::read_to_string(&target).unwrap();
        let updated: TestConfig = toml::from_str(&content).unwrap();
        assert_eq!(updated.value, 42);
    }

    #[test]
    fn atomic_write_creates_parent_dirs() {
        let (dir, git) = setup();
        let target = dir.path().join("nested/deep/file.toml");

        atomic_write_and_commit(
            &git,
            &target,
            b"content",
            CommitKind::Config,
            "nested file",
        )
        .unwrap();

        assert!(target.exists());
    }
}
```

- [ ] **Step 2: Add atomic module to lib.rs**

```rust
pub mod atomic;
pub use atomic::{atomic_toml_update, atomic_write_and_commit};
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p autonomic-state
```

- [ ] **Step 4: Commit**

```bash
git add crates/autonomic-state/
git commit -m "feat(state): add atomic file write with tmp+fsync+rename+commit"
```

## Task 9: State Manager (autonomic-state)

**Files:**
- Create: `crates/autonomic-state/src/manager.rs`
- Modify: `crates/autonomic-state/src/lib.rs`

The StateManager is the unified entry point combining git, lock, db pool, and atomic ops.

- [ ] **Step 1: Create manager.rs**

```rust
//! Unified state manager combining git, lock, and database pool.

use sqlx::PgPool;
use std::path::{Path, PathBuf};
use tracing::info;

use crate::error::StateError;
use crate::git::{CommitKind, GitManager};
use crate::lock::StateLock;

/// The single entry point for all state operations.
/// Owns the exclusive lock, git repo, and database pool.
pub struct StateManager {
    /// Root directory (~/.autonomic/).
    root: PathBuf,
    /// Git repository manager.
    git: GitManager,
    /// Exclusive file lock (held for daemon lifetime).
    _lock: StateLock,
    /// PostgreSQL connection pool.
    pool: PgPool,
}

impl StateManager {
    /// Open (or initialize) the state store.
    /// Acquires the exclusive lock, initializes git if needed,
    /// and connects to PostgreSQL.
    pub async fn open(state_dir: &Path, database_url: &str) -> Result<Self, StateError> {
        // Ensure state directory exists
        tokio::fs::create_dir_all(state_dir).await?;

        // Acquire exclusive lock
        let lock = StateLock::acquire(state_dir)?;
        info!(path = %state_dir.display(), "State lock acquired");

        // Initialize git repo
        let git = GitManager::open_or_init(state_dir)?;

        // Connect to PostgreSQL
        let pool = autonomic_db::create_pool(database_url)
            .await
            .map_err(|e| StateError::Git(e.to_string()))?;

        // Run migrations
        autonomic_db::run_migrations(&pool)
            .await
            .map_err(|e| StateError::Git(e.to_string()))?;

        Ok(StateManager {
            root: state_dir.to_path_buf(),
            git,
            _lock: lock,
            pool,
        })
    }

    /// Get the git manager for direct git operations.
    pub fn git(&self) -> &GitManager {
        &self.git
    }

    /// Get the database pool for SQL queries.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Get the state directory root path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Path to config.toml.
    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    /// Path to secrets.toml.
    pub fn secrets_path(&self) -> PathBuf {
        self.root.join("secrets.toml")
    }

    /// Create a named snapshot (tag).
    pub fn snapshot(&self, tag_name: &str, message: &str) -> Result<(), StateError> {
        self.git.commit_all(CommitKind::Snapshot, &format!("snapshot for {tag_name}"))?;
        self.git.create_tag(tag_name, message)
    }

    /// Rollback to a tag or commit. Preserves gitignored files (XD-008).
    pub fn rollback(&self, target: &str) -> Result<(), StateError> {
        self.git.rollback(target)
    }
}
```

- [ ] **Step 2: Add manager module to lib.rs and update exports**

Final `lib.rs`:

```rust
//! Git-backed state management with point-in-time recovery.

pub mod atomic;
pub mod error;
pub mod git;
pub mod lock;
pub mod manager;

pub use atomic::{atomic_toml_update, atomic_write_and_commit};
pub use error::StateError;
pub use git::{CommitKind, GitManager};
pub use lock::StateLock;
pub use manager::StateManager;
```

- [ ] **Step 3: Run cargo check (DB-dependent, so offline mode)**

```bash
SQLX_OFFLINE=true cargo check -p autonomic-state
```

- [ ] **Step 4: Commit**

```bash
git add crates/autonomic-state/
git commit -m "feat(state): add StateManager unifying git, lock, and database pool"
```

## Task 10: Integration Verification

- [ ] **Step 1: Verify full workspace builds**

```bash
SQLX_OFFLINE=true cargo check --workspace
```

Expected: clean check, all 13 crates resolve.

- [ ] **Step 2: Run all tests**

```bash
cargo test -p autonomic-core && cargo test -p autonomic-state
```

Expected: all tests pass.

- [ ] **Step 3: Run clippy**

```bash
SQLX_OFFLINE=true cargo clippy --workspace --all-targets -- -D warnings
```

Expected: clean. Fix any warnings (likely around the `std::process::Command` usage in git.rs — add the `#[allow]` attribute with comment).

- [ ] **Step 4: Run fmt check**

```bash
cargo fmt --check --all
```

Expected: clean.

- [ ] **Step 5: Verify dependency direction**

```bash
# autonomic-core should have NO sibling dependencies
grep -E "autonomic-" crates/autonomic-core/Cargo.toml
# Expected: no matches

# autonomic-db should depend only on autonomic-core
grep "autonomic-" crates/autonomic-db/Cargo.toml
# Expected: only autonomic-core

# autonomic-state should depend on autonomic-core and autonomic-db
grep "autonomic-" crates/autonomic-state/Cargo.toml
# Expected: autonomic-core and autonomic-db
```

- [ ] **Step 6: Tag milestone**

```bash
git tag -a milestone/phase-1a -m "Phase 1A: Foundation layer — core types, DB pool, git-backed state"
```

## Integration Test (requires Postgres)

If Postgres is running (`mise run infra:up`), run the full integration test:

- [ ] **Step 1: Start infrastructure**

```bash
mise run infra:up
```

- [ ] **Step 2: Run with real database**

```bash
DATABASE_URL="postgresql://autonomic:autonomic_dev_password@localhost:5432/autonomic" cargo test --workspace
```

- [ ] **Step 3: Verify migrations applied**

```bash
mise run infra:psql
# Then in psql:
\dt
# Expected: sessions, experience_traces, memory_entries, model_performance, metrics, projects, rate_budget, memory_md_sync
```
