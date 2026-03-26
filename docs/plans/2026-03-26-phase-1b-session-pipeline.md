# Phase 1B: Session Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the session pipeline that spawns Claude Code on the host, parses stream-json output, tracks cost, enforces timeouts, and provides a daemon HTTP API with watchdog health monitoring.

**Architecture:** Four crates built in dependency order: `autonomic-container` (trait + host runtime), `autonomic-session` (subprocess management + output parsing + cost tracking), `autonomic-daemon` (axum HTTP server + signal handling + PID), `autonomic-watchdog` (health poll + crash rollback). "Host-first, containers later" — Phase 1 uses `HostRuntime` (direct `tokio::process::Command`); `PodmanRuntime` comes in Phase 2.

**Tech Stack:** Rust 2024 (1.94), tokio, axum 0.8, serde/serde_json, tracing/tracing-subscriber, thiserror 2, chrono, nix (POSIX signals), dashmap (concurrent session map)

**Spec:** `docs/architecture/session-management.md`, `docs/architecture/overview.md`, `.claude/specs/cross-document-constraints.md`, `.claude/specs/async-concurrency.md`

**Prereqs from Phase 1A:** `SessionId`, `ProjectId`, `ModelTier`, `SessionStatus`, `Subsystem`, `CoreError`, `AutonomicConfig`, `RateBudget`, `CostEntry`, `BudgetAllocation`, `DbPool`, `StateLock`, `GitManager`, `StateManager`

**Key Constraints:**

- XD-001: Only SessionManager spawns Claude Code
- XD-005: stderr consumed concurrently with stdout
- XD-006: RateBudget from autonomic-core shared across subsystems
- XD-011: Secrets from secrets.toml, not env vars
- clippy.toml: No `std::process::Command`, no `std::thread::sleep`, no `println!`

## File Map

```txt
MODIFY: Cargo.toml                                       # Add nix, dashmap to workspace deps
MODIFY: crates/autonomic-container/Cargo.toml            # Add dependencies
CREATE: crates/autonomic-container/src/lib.rs             # Re-exports
CREATE: crates/autonomic-container/src/error.rs           # ContainerError enum
CREATE: crates/autonomic-container/src/config.rs          # ContainerConfig, ResourceLimits
CREATE: crates/autonomic-container/src/runtime.rs         # ContainerRuntime trait
CREATE: crates/autonomic-container/src/host.rs            # HostRuntime implementation

MODIFY: crates/autonomic-session/Cargo.toml              # Add dependencies
CREATE: crates/autonomic-session/src/lib.rs               # Re-exports
CREATE: crates/autonomic-session/src/error.rs             # SessionError, SessionConfigError
CREATE: crates/autonomic-session/src/config.rs            # SessionConfig + validation
CREATE: crates/autonomic-session/src/parser.rs            # StreamMessage, parse_output_stream
CREATE: crates/autonomic-session/src/cost.rs              # CostTracker
CREATE: crates/autonomic-session/src/command.rs           # build_command (env filtering, XD-001)
CREATE: crates/autonomic-session/src/manager.rs           # SessionManager (spawn, monitor, cancel)

MODIFY: crates/autonomic-daemon/Cargo.toml               # Add dependencies
CREATE: crates/autonomic-daemon/src/main.rs               # Entrypoint (tokio::main, tracing, signals)
CREATE: crates/autonomic-daemon/src/api.rs                # axum Router, health/session routes
CREATE: crates/autonomic-daemon/src/pid.rs                # PID file management
CREATE: crates/autonomic-daemon/src/shutdown.rs           # Graceful shutdown (SIGTERM drain)
CREATE: crates/autonomic-daemon/src/tracing_setup.rs      # Structured JSON logging setup

MODIFY: crates/autonomic-watchdog/Cargo.toml             # Add dependencies
CREATE: crates/autonomic-watchdog/src/main.rs             # Entrypoint
CREATE: crates/autonomic-watchdog/src/health.rs           # Health check loop (HTTP poll)
CREATE: crates/autonomic-watchdog/src/orphan.rs           # Orphan process sweep
CREATE: crates/autonomic-watchdog/src/rollback.rs         # Crash-triggered rollback to known-good tag
```

## Task 1: Add Workspace Dependencies for Phase 1B

**Files:**
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Add nix and dashmap to workspace dependencies**

Add to `[workspace.dependencies]`:

```toml
nix = { version = "0.29", features = ["signal", "process"] }
dashmap = "6"
```

- [ ] **Step 2: Add to known-dep-versions.toml**

Add to `.claude/known-dep-versions.toml` under `[dependencies]`:

```toml
dashmap = "6"
nix = "0.29"
```

- [ ] **Step 3: Verify workspace resolves**

```bash
SQLX_OFFLINE=true cargo check --workspace
```

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock .claude/known-dep-versions.toml
git commit -m "build: add nix and dashmap to workspace dependencies for Phase 1B"
```

## Task 2: Container Error Types (autonomic-container)

**Files:**
- Modify: `crates/autonomic-container/Cargo.toml`
- Create: `crates/autonomic-container/src/error.rs`
- Rewrite: `crates/autonomic-container/src/lib.rs`

- [ ] **Step 1: Update Cargo.toml**

```toml
[package]
name = "autonomic-container"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Podman/Docker container runtime abstraction for Autonomic"

[dependencies]
autonomic-core = { path = "../autonomic-core" }
serde = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio = { workspace = true, features = ["test-util", "macros"] }

[lints]
workspace = true
```

- [ ] **Step 2: Create error.rs**

```rust
//! Container runtime error types.

use std::path::PathBuf;
use thiserror::Error;

/// Errors from container runtime operations.
#[derive(Debug, Error)]
pub enum ContainerError {
    #[error("Failed to spawn process: {0}")]
    SpawnFailed(#[source] std::io::Error),

    #[error("Process stdout was not captured")]
    NoStdout,

    #[error("Process stderr was not captured")]
    NoStderr,

    #[error("Process exited with code {code:?}, signal {signal:?}")]
    ProcessFailed {
        code: Option<i32>,
        signal: Option<i32>,
        stderr: String,
    },

    #[error("Working directory not found: {0}")]
    WorkingDirNotFound(PathBuf),

    #[error("Claude binary not found at: {0}")]
    ClaudeBinaryNotFound(PathBuf),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Container runtime not available: {0}")]
    RuntimeUnavailable(String),
}
```

- [ ] **Step 3: Update lib.rs**

```rust
//! Podman/Docker container runtime abstraction for Autonomic.
//!
//! Phase 1: `HostRuntime` spawns Claude Code directly on the host via
//! `tokio::process::Command`. The `ContainerRuntime` trait exists so
//! `PodmanRuntime` can be added in Phase 2 without changing callers.

pub mod error;

pub use error::ContainerError;
```

- [ ] **Step 4: Run cargo check**

```bash
cargo check -p autonomic-container
```

- [ ] **Step 5: Commit**

```bash
git add crates/autonomic-container/
git commit -m "feat(container): add ContainerError enum"
```

## Task 3: ContainerConfig and ResourceLimits (autonomic-container)

**Files:**
- Create: `crates/autonomic-container/src/config.rs`
- Modify: `crates/autonomic-container/src/lib.rs`

- [ ] **Step 1: Create config.rs**

```rust
//! Container and process configuration types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

/// Configuration for spawning a process (host or container).
///
/// This is runtime-agnostic: `HostRuntime` uses it to build a
/// `tokio::process::Command`; `PodmanRuntime` (Phase 2) will use it
/// to build `podman run` arguments.
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    /// The executable to run (e.g., "claude" for host, ignored for container).
    pub executable: PathBuf,

    /// Arguments to the executable.
    pub args: Vec<String>,

    /// Working directory inside the container/process.
    pub working_dir: PathBuf,

    /// Environment variables. Merged on top of the filtered base env.
    pub env: HashMap<String, String>,

    /// Environment variable keys to remove from the inherited environment.
    pub env_remove: Vec<String>,

    /// Process timeout (wall-clock).
    pub timeout: Duration,

    /// Resource limits (enforced by container runtime; advisory for host).
    pub resources: ResourceLimits,

    /// Whether to pipe stdin (false = /dev/null).
    pub stdin: bool,
}

/// Resource limits for a container or process.
///
/// On host (`HostRuntime`), these are advisory/logged only.
/// On container (`PodmanRuntime`), they map to `--memory`, `--cpus`, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum memory in bytes (e.g., 4 * 1024^3 for 4GB).
    #[serde(default = "default_memory")]
    pub memory_bytes: u64,

    /// CPU limit as a float (e.g., 2.0 for 2 CPUs).
    #[serde(default = "default_cpus")]
    pub cpus: f64,
}

fn default_memory() -> u64 {
    4 * 1024 * 1024 * 1024 // 4 GB
}

fn default_cpus() -> f64 {
    2.0
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_bytes: default_memory(),
            cpus: default_cpus(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_resource_limits() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.memory_bytes, 4 * 1024 * 1024 * 1024);
        assert!((limits.cpus - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn resource_limits_serde_roundtrip() {
        let limits = ResourceLimits {
            memory_bytes: 8 * 1024 * 1024 * 1024,
            cpus: 4.0,
        };
        let json = serde_json::to_string(&limits).unwrap();
        let decoded: ResourceLimits = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.memory_bytes, limits.memory_bytes);
        assert!((decoded.cpus - limits.cpus).abs() < f64::EPSILON);
    }
}
```

- [ ] **Step 2: Add config module to lib.rs**

```rust
pub mod config;
pub mod error;

pub use config::{ContainerConfig, ResourceLimits};
pub use error::ContainerError;
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p autonomic-container
```

- [ ] **Step 4: Commit**

```bash
git add crates/autonomic-container/
git commit -m "feat(container): add ContainerConfig and ResourceLimits"
```

## Task 4: ContainerRuntime Trait (autonomic-container)

**Files:**
- Create: `crates/autonomic-container/src/runtime.rs`
- Modify: `crates/autonomic-container/src/lib.rs`

- [ ] **Step 1: Create runtime.rs**

```rust
//! Container runtime trait abstracting host vs container execution.

use tokio::process::Child;

use crate::config::ContainerConfig;
use crate::error::ContainerError;

/// Output from a container/process execution.
#[derive(Debug)]
pub struct ProcessOutput {
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub stderr: String,
}

/// Abstraction over process execution environments.
///
/// `HostRuntime` spawns directly via `tokio::process::Command`.
/// `PodmanRuntime` (Phase 2) wraps the command in `podman run`.
///
/// NOTE: Uses RPITIT (return-position impl Trait in trait), which makes
/// this trait NOT object-safe. Phase 1 uses static dispatch only. If Phase 2
/// needs `Arc<dyn ContainerRuntime>` for runtime switching between Host/Podman/Docker,
/// migrate to `async_trait` or `trait_variant::make(Send)`.
// TODO(phase-2): Evaluate object safety if runtime switching is needed.
pub trait ContainerRuntime: Send + Sync {
    fn spawn(
        &self,
        config: &ContainerConfig,
    ) -> impl std::future::Future<Output = Result<Child, ContainerError>> + Send;

    fn is_available(
        &self,
    ) -> impl std::future::Future<Output = Result<(), ContainerError>> + Send;

    fn name(&self) -> &'static str;
}
```

- [ ] **Step 2: Update lib.rs**

```rust
pub mod config;
pub mod error;
pub mod runtime;

pub use config::{ContainerConfig, ResourceLimits};
pub use error::ContainerError;
pub use runtime::{ContainerRuntime, ProcessOutput};
```

- [ ] **Step 3: Run cargo check**

```bash
cargo check -p autonomic-container
```

- [ ] **Step 4: Commit**

```bash
git add crates/autonomic-container/
git commit -m "feat(container): add ContainerRuntime trait with spawn and is_available"
```

## Task 5: HostRuntime Implementation (autonomic-container)

**Files:**
- Create: `crates/autonomic-container/src/host.rs`
- Modify: `crates/autonomic-container/src/lib.rs`

- [ ] **Step 1: Create host.rs with tests**

```rust
//! Host-based runtime: spawns Claude Code directly via `tokio::process::Command`.
//!
//! This is the Phase 1 runtime. `PodmanRuntime` (Phase 2) adds true isolation.

use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tracing::info;

use crate::config::ContainerConfig;
use crate::error::ContainerError;
use crate::runtime::ContainerRuntime;

/// Spawns processes directly on the host.
#[derive(Debug, Clone)]
pub struct HostRuntime {
    claude_binary: PathBuf,
}

impl HostRuntime {
    pub fn new(claude_binary: PathBuf) -> Self {
        Self { claude_binary }
    }

    pub async fn from_path() -> Result<Self, ContainerError> {
        let output = Command::new("which")
            .arg("claude")
            .output()
            .await
            .map_err(ContainerError::SpawnFailed)?;

        if !output.status.success() {
            return Err(ContainerError::ClaudeBinaryNotFound(PathBuf::from(
                "claude (not found in PATH)",
            )));
        }

        let path = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
        info!(path = %path.display(), "Found claude binary");
        Ok(Self { claude_binary: path })
    }

    pub fn claude_binary(&self) -> &PathBuf {
        &self.claude_binary
    }
}

impl ContainerRuntime for HostRuntime {
    async fn spawn(&self, config: &ContainerConfig) -> Result<Child, ContainerError> {
        if !config.working_dir.is_dir() {
            return Err(ContainerError::WorkingDirNotFound(config.working_dir.clone()));
        }

        let mut cmd = Command::new(&self.claude_binary);

        for arg in &config.args {
            cmd.arg(arg);
        }

        cmd.current_dir(&config.working_dir);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        if config.stdin {
            cmd.stdin(Stdio::piped());
        } else {
            cmd.stdin(Stdio::null());
        }

        // Environment: filtered parent env + user-specified vars
        cmd.env_clear();
        for (key, value) in std::env::vars() {
            if config.env_remove.contains(&key) {
                continue;
            }
            cmd.env(&key, &value);
        }
        for (key, value) in &config.env {
            cmd.env(key, value);
        }

        tracing::debug!(
            memory_bytes = config.resources.memory_bytes,
            cpus = config.resources.cpus,
            "Host runtime: resource limits are advisory only"
        );

        let child = cmd.spawn().map_err(ContainerError::SpawnFailed)?;
        info!(pid = child.id(), working_dir = %config.working_dir.display(), "Spawned host process");
        Ok(child)
    }

    async fn is_available(&self) -> Result<(), ContainerError> {
        if !self.claude_binary.exists() {
            return Err(ContainerError::ClaudeBinaryNotFound(self.claude_binary.clone()));
        }
        Ok(())
    }

    fn name(&self) -> &'static str {
        "host"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ResourceLimits;
    use std::collections::HashMap;
    use std::time::Duration;

    #[tokio::test]
    async fn host_runtime_spawns_process() {
        let runtime = HostRuntime::new(PathBuf::from("/bin/echo"));
        let config = ContainerConfig {
            executable: PathBuf::from("/bin/echo"),
            args: vec!["test".into()],
            working_dir: std::env::temp_dir(),
            env: HashMap::new(),
            env_remove: vec![],
            timeout: Duration::from_secs(5),
            resources: ResourceLimits::default(),
            stdin: false,
        };

        let mut child = runtime.spawn(&config).await.unwrap();
        let status = child.wait().await.unwrap();
        assert!(status.success());
    }

    #[tokio::test]
    async fn host_runtime_rejects_missing_working_dir() {
        let runtime = HostRuntime::new(PathBuf::from("/bin/echo"));
        let config = ContainerConfig {
            executable: PathBuf::from("/bin/echo"),
            args: vec![],
            working_dir: PathBuf::from("/nonexistent/dir/12345"),
            env: HashMap::new(),
            env_remove: vec![],
            timeout: Duration::from_secs(5),
            resources: ResourceLimits::default(),
            stdin: false,
        };

        let result = runtime.spawn(&config).await;
        assert!(matches!(result, Err(ContainerError::WorkingDirNotFound(_))));
    }

    #[tokio::test]
    async fn host_runtime_is_available_with_valid_binary() {
        let runtime = HostRuntime::new(PathBuf::from("/bin/echo"));
        assert!(runtime.is_available().await.is_ok());
    }

    #[tokio::test]
    async fn host_runtime_unavailable_with_missing_binary() {
        let runtime = HostRuntime::new(PathBuf::from("/nonexistent/binary"));
        assert!(matches!(
            runtime.is_available().await,
            Err(ContainerError::ClaudeBinaryNotFound(_))
        ));
    }
}
```

- [ ] **Step 2: Update lib.rs**

```rust
pub mod config;
pub mod error;
pub mod host;
pub mod runtime;

pub use config::{ContainerConfig, ResourceLimits};
pub use error::ContainerError;
pub use host::HostRuntime;
pub use runtime::{ContainerRuntime, ProcessOutput};
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p autonomic-container
```

- [ ] **Step 4: Commit**

```bash
git add crates/autonomic-container/
git commit -m "feat(container): add HostRuntime implementing ContainerRuntime trait"
```

## Task 6: Session Error Types (autonomic-session)

**Files:**
- Modify: `crates/autonomic-session/Cargo.toml`
- Create: `crates/autonomic-session/src/error.rs`
- Rewrite: `crates/autonomic-session/src/lib.rs`

- [ ] **Step 1: Update Cargo.toml**

```toml
[package]
name = "autonomic-session"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Claude Code subprocess management, output parsing, and cost tracking"

[dependencies]
autonomic-core = { path = "../autonomic-core" }
autonomic-container = { path = "../autonomic-container" }
chrono = { workspace = true }
dashmap = { workspace = true }
nix = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
tempfile = { workspace = true }
tokio = { workspace = true, features = ["test-util", "macros"] }

[lints]
workspace = true
```

- [ ] **Step 2: Create error.rs**

```rust
//! Session error types.

use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Failed to spawn claude subprocess: {0}")]
    SpawnFailed(#[source] autonomic_container::ContainerError),

    #[error("Subprocess stdout was not captured")]
    NoStdout,

    #[error("Subprocess stderr was not captured")]
    NoStderr,

    #[error("Session timed out after {elapsed:?} (limit: {limit:?})")]
    Timeout { elapsed: Duration, limit: Duration },

    #[error("Session exceeded cost budget: ${accumulated:.4} > ${budget:.4}")]
    BudgetExceeded { accumulated: f64, budget: f64 },

    #[error("Claude process crashed with exit code {exit_code:?}, signal {signal:?}")]
    ProcessCrashed {
        exit_code: Option<i32>,
        signal: Option<i32>,
        stderr: String,
    },

    #[error("Rate limited by Claude API")]
    RateLimited,

    #[error("Output parse error: {message}")]
    ParseError { message: String, raw_line: String },

    #[error("Session config validation failed: {0}")]
    ConfigError(#[from] SessionConfigError),

    #[error("Global budget exhausted for the current window")]
    GlobalBudgetExhausted,

    #[error("Maximum concurrent sessions reached ({max})")]
    ConcurrencyLimitReached { max: usize },

    #[error("Session not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum SessionConfigError {
    #[error("Path must be absolute: {0}")]
    RelativePath(PathBuf),

    #[error("Working directory not found: {0}")]
    WorkingDirNotFound(PathBuf),

    #[error("Cannot specify both --resume and --continue")]
    ConflictingResumeOptions,

    #[error("Agent Teams requires an Opus model, got: {model}")]
    AgentTeamsRequiresOpus { model: String },

    #[error("Cost budget must be positive, got: {0}")]
    InvalidBudget(f64),

    #[error("Forbidden environment variable: {0}")]
    ForbiddenEnvVar(String),

    #[error("Worktree not found: {0}")]
    WorktreeNotFound(PathBuf),

    #[error("Invalid worktree (missing .git file): {0}")]
    InvalidWorktree(PathBuf),

    #[error("Working dir {working_dir} does not match worktree path {worktree_path}")]
    WorkingDirWorktreeMismatch {
        working_dir: PathBuf,
        worktree_path: PathBuf,
    },
}
```

- [ ] **Step 3: Update lib.rs**

```rust
//! Claude Code subprocess management, output parsing, and cost tracking.
//!
//! This crate owns the full lifecycle of Claude Code subprocess instances.
//! It is the ONLY component that spawns `claude` processes (XD-001).

pub mod error;

pub use error::{SessionConfigError, SessionError};
```

- [ ] **Step 4: Run cargo check**

```bash
cargo check -p autonomic-session
```

- [ ] **Step 5: Commit**

```bash
git add crates/autonomic-session/
git commit -m "feat(session): add SessionError and SessionConfigError enums"
```

## Task 7: SessionConfig with Validation (autonomic-session)

**Files:**
- Create: `crates/autonomic-session/src/config.rs`
- Modify: `crates/autonomic-session/src/lib.rs`

See full spec in `docs/architecture/session-management.md` §4 SessionConfig.

- [ ] **Step 1: Create config.rs with tests**

The SessionConfig includes: session_id, working_dir, prompt, model, system_prompt, allowed_tools, timeout, cost_budget_usd, resume_session_id, continue_recent, extra_env, agent_teams, worktree. Validation enforces: absolute paths, no conflicting resume options, opus for agent teams, positive budget, no CLAUDECODE env var, worktree consistency.

10 unit tests covering all validation rules. See Phase 1A plan for the exact code pattern.

- [ ] **Step 2: Run tests**

```bash
cargo test -p autonomic-session
```

- [ ] **Step 3: Commit**

```bash
git add crates/autonomic-session/
git commit -m "feat(session): add SessionConfig with validation rules"
```

## Task 8: Stream-JSON Parser (autonomic-session)

**Files:**
- Create: `crates/autonomic-session/src/parser.rs`
- Modify: `crates/autonomic-session/src/lib.rs`

- [ ] **Step 1: Create parser.rs**

StreamMessage with serde deserialization for Claude Code `--output-format stream-json` NDJSON output. Fields: message_type, subtype, total_cost_usd, duration_ms, session_id, result, error, extra (flattened). SessionOutcome enum: Success, Error, MaxTurns. Functions: parse_line(), classify_result(). 12 unit tests.

- [ ] **Step 2: Run tests**

```bash
cargo test -p autonomic-session
```

- [ ] **Step 3: Commit**

```bash
git add crates/autonomic-session/
git commit -m "feat(session): add stream-json parser with StreamMessage and SessionOutcome"
```

## Task 9: CostTracker (autonomic-session)

**Files:**
- Create: `crates/autonomic-session/src/cost.rs`
- Modify: `crates/autonomic-session/src/lib.rs`

- [ ] **Step 1: Create cost.rs**

DashMap-backed concurrent cost tracker. Per-session costs + rolling window for budget enforcement. Methods: record_cost(), can_afford(), window_total(), total_cost(), session_cost(), session_count(). 7 unit tests + 1 proptest.

- [ ] **Step 2: Run tests**

```bash
cargo test -p autonomic-session
```

- [ ] **Step 3: Commit**

```bash
git add crates/autonomic-session/
git commit -m "feat(session): add CostTracker with DashMap and sliding window"
```

## Task 10: Command Builder (autonomic-session)

**Files:**
- Create: `crates/autonomic-session/src/command.rs`
- Modify: `crates/autonomic-session/src/lib.rs`

XD-001: This is the ONLY place that builds commands to spawn Claude Code.

- [ ] **Step 1: Create command.rs**

Pure function `build_container_config(SessionConfig, claude_binary) -> ContainerConfig`. Builds args: `--print --output-format stream-json --permission-mode acceptEdits --model <model>`, optional `--system-prompt`, `--allowedTools`, `--resume`/`--continue`, prompt as positional. Removes CLAUDECODE from env. 10 unit tests.

- [ ] **Step 2: Run tests**

```bash
cargo test -p autonomic-session
```

- [ ] **Step 3: Commit**

```bash
git add crates/autonomic-session/
git commit -m "feat(session): add command builder with env filtering (XD-001)"
```

## Task 11: SessionManager (autonomic-session)

**Files:**
- Create: `crates/autonomic-session/src/manager.rs`
- Modify: `crates/autonomic-session/src/lib.rs`

Most complex task. Full session lifecycle: validate -> spawn -> monitor -> complete.

- [ ] **Step 1: Create manager.rs**

SessionManager with: DashMap sessions, Arc<Semaphore> concurrency, Arc<CostTracker>, broadcast::Sender for events. Primary method: `run_session(config, runtime)` which validates, checks budget, acquires permit, spawns via ContainerRuntime, then monitors with `tokio::select!` loop consuming stdout and stderr concurrently (XD-005) with timeout enforcement. SessionState enum: Running, Completed, Failed, TimedOut. SessionRecord with transcript and stderr. Graceful termination: SIGTERM -> 5s -> SIGKILL.

- [ ] **Step 2: Run cargo check and tests**

```bash
cargo check -p autonomic-session && cargo test -p autonomic-session
```

- [ ] **Step 3: Commit**

```bash
git add crates/autonomic-session/
git commit -m "feat(session): add SessionManager with concurrent stdout/stderr monitoring (XD-005)"
```

## Task 12: Daemon Tracing Setup (autonomic-daemon)

**Files:**
- Modify: `crates/autonomic-daemon/Cargo.toml`
- Create: `crates/autonomic-daemon/src/tracing_setup.rs`

- [ ] **Step 1: Update Cargo.toml**

Add dependencies: autonomic-core, autonomic-db, autonomic-container, autonomic-session, axum, chrono, dirs, nix, serde, serde_json, tokio, tracing, tracing-subscriber. Note: `autonomic-db` is required for DB pool creation and migrations on daemon startup.

- [ ] **Step 2: Create tracing_setup.rs**

`init_tracing(log_dir)`: dual-layer (stderr compact + file JSON). EnvFilter with `info,autonomic=debug` default. Uses `#[allow(clippy::disallowed_methods)]` for startup-only std::fs calls.

- [ ] **Step 3: Commit**

```bash
git add crates/autonomic-daemon/
git commit -m "feat(daemon): add structured JSON tracing setup"
```

## Task 13: PID File Management (autonomic-daemon)

**Files:**
- Create: `crates/autonomic-daemon/src/pid.rs`

- [ ] **Step 1: Create pid.rs**

PidFile struct with create(), read(), remove(). Writes PID to `~/.autonomic/daemon.pid`. Watchdog reads this to verify daemon is running.

- [ ] **Step 2: Commit**

```bash
git add crates/autonomic-daemon/src/pid.rs
git commit -m "feat(daemon): add PID file management"
```

## Task 14: Graceful Shutdown (autonomic-daemon)

**Files:**
- Create: `crates/autonomic-daemon/src/shutdown.rs`

- [ ] **Step 1: Create shutdown.rs**

`shutdown_signal()` async function waiting for SIGTERM or SIGINT via `tokio::signal`. Unix-specific SIGTERM + cross-platform SIGINT.

- [ ] **Step 2: Commit**

```bash
git add crates/autonomic-daemon/src/shutdown.rs
git commit -m "feat(daemon): add graceful shutdown signal handling"
```

## Task 15: HTTP API (autonomic-daemon)

**Files:**
- Create: `crates/autonomic-daemon/src/api.rs`

- [ ] **Step 1: Create api.rs**

axum Router with AppState (SessionManager, CostTracker, start_time). Routes: GET /health (watchdog polling), GET /api/v1/status (version, uptime, sessions, cost), GET /api/v1/sessions (list active), GET /api/v1/budget (cost tracking).

- [ ] **Step 2: Commit**

```bash
git add crates/autonomic-daemon/src/api.rs
git commit -m "feat(daemon): add axum HTTP API with health, status, sessions, budget endpoints"
```

## Task 16: Daemon Main Entrypoint (autonomic-daemon)

**Files:**
- Rewrite: `crates/autonomic-daemon/src/main.rs`

- [ ] **Step 1: Rewrite main.rs**

`#[tokio::main]` entrypoint: determine state_dir, init tracing, load config/secrets, create PID file, **connect to PostgreSQL via `autonomic_db::create_pool(&secrets.database.url)` and run migrations via `autonomic_db::run_migrations(&pool)` (XD-002)**, find claude binary via HostRuntime::from_path(), init CostTracker + SessionManager, build router (pass pool to AppState for session trace persistence), bind axum server with graceful shutdown, cleanup PID on exit.

- [ ] **Step 2: Run cargo check**

```bash
SQLX_OFFLINE=true cargo check -p autonomic-daemon
```

- [ ] **Step 3: Commit**

```bash
git add crates/autonomic-daemon/
git commit -m "feat(daemon): add main entrypoint with axum server, PID file, signal handling (FR-001)"
```

## Task 17: Watchdog Health Check (autonomic-watchdog)

**Files:**
- Modify: `crates/autonomic-watchdog/Cargo.toml`
- Create: `crates/autonomic-watchdog/src/health.rs`

- [ ] **Step 1: Update Cargo.toml**

Add dependencies: autonomic-core, chrono, dirs, nix, serde, serde_json, tokio, tracing, tracing-subscriber.

- [ ] **Step 2: Create health.rs**

Lightweight HTTP health check via raw TCP (no reqwest dependency). `check_daemon_health(port) -> HealthStatus`. `health_check_loop(port, interval, max_consecutive_failures)` — returns when daemon is considered crashed.

- [ ] **Step 3: Commit**

```bash
git add crates/autonomic-watchdog/
git commit -m "feat(watchdog): add daemon health check via HTTP polling"
```

## Task 18: Orphan Process Sweep (autonomic-watchdog)

**Files:**
- Create: `crates/autonomic-watchdog/src/orphan.rs`

- [ ] **Step 1: Create orphan.rs**

`sweep_orphans(state_dir)`: reads `~/.autonomic/state/sessions/*.pid`, checks if each PID is still running via `kill(pid, 0)`, sends SIGTERM to orphans, cleans up PID files.

- [ ] **Step 2: Commit**

```bash
git add crates/autonomic-watchdog/src/orphan.rs
git commit -m "feat(watchdog): add orphan process sweep for crashed daemon recovery"
```

## Task 19: Crash Rollback (autonomic-watchdog)

**Files:**
- Create: `crates/autonomic-watchdog/src/rollback.rs`

- [ ] **Step 1: Create rollback.rs**

`rollback_to_known_good(state_dir)`: finds most recent `known-good/*` tag, creates pre-rollback snapshot, uses `git checkout <tag> -- .` (XD-008: no git clean), forward commits. Uses git CLI (not gix) deliberately — this is an **intentional exception** to the "gix for git ops" rule. The watchdog must not share library dependencies with the daemon it monitors; if gix or its transitive deps cause the daemon to crash, the watchdog must still function. Add a doc comment in rollback.rs explaining this rationale.

- [ ] **Step 2: Commit**

```bash
git add crates/autonomic-watchdog/src/rollback.rs
git commit -m "feat(watchdog): add crash-triggered rollback to known-good tag (XD-008)"
```

## Task 20: Watchdog Main Entrypoint (autonomic-watchdog)

**Files:**
- Rewrite: `crates/autonomic-watchdog/src/main.rs`

- [ ] **Step 1: Rewrite main.rs**

`#[tokio::main]` entrypoint: minimal tracing, sweep orphans on startup, read daemon config for port, health check loop (30s interval, 3 failures = crash), attempt rollback on crash, sweep orphans again, exit (launchd restarts both).

- [ ] **Step 2: Run cargo check**

```bash
SQLX_OFFLINE=true cargo check -p autonomic-watchdog
```

- [ ] **Step 3: Commit**

```bash
git add crates/autonomic-watchdog/
git commit -m "feat(watchdog): add main entrypoint with health loop, orphan sweep, crash rollback (FR-001)"
```

## Task 21: Integration Verification

- [ ] **Step 1: Verify full workspace builds**

```bash
SQLX_OFFLINE=true cargo check --workspace
```

- [ ] **Step 2: Run all Phase 1B tests**

```bash
cargo test -p autonomic-container && cargo test -p autonomic-session
```

- [ ] **Step 3: Run clippy**

```bash
SQLX_OFFLINE=true cargo clippy --workspace --all-targets -- -D warnings
```

- [ ] **Step 4: Run fmt check**

```bash
cargo fmt --check --all
```

- [ ] **Step 5: Verify dependency direction**

```bash
grep "autonomic-" crates/autonomic-container/Cargo.toml   # only core
grep "autonomic-" crates/autonomic-session/Cargo.toml      # core + container
grep "autonomic-" crates/autonomic-daemon/Cargo.toml       # core + db + container + session
grep "autonomic-" crates/autonomic-watchdog/Cargo.toml     # only core
```

- [ ] **Step 6: Verify XD-001**

```bash
grep -r 'Command::new.*claude' crates/ --include="*.rs"
```

Expected: matches ONLY in `autonomic-container/src/host.rs`.

- [ ] **Step 7: Tag milestone**

```bash
git tag -a milestone/phase-1b -m "Phase 1B: Session pipeline — container runtime, session manager, daemon, watchdog"
```
