# Phase 1C: User Interface Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the CLI binary and daemon API endpoints that complete Phase 1: `autonomic run`, `autonomic status`, project registry, metrics display, snapshot/rollback commands, launchd integration, and frozen-file enforcement.

**Architecture:** `autonomic-cli` uses clap 4.6 with derive for subcommands. It communicates with the daemon via HTTP (localhost). The daemon gets a `POST /api/v1/sessions` endpoint that accepts a session request and spawns via `SessionManager`. Project registry is stored in PostgreSQL (`projects` table, already in migration). Experience traces are persisted to PostgreSQL on session completion. launchd plists are static XML files installed to `~/Library/LaunchAgents/`.

**Tech Stack:** Rust 2024 (1.94), clap 4.6 (derive), reqwest (blocking client for CLI), serde/serde_json, tokio, axum, sqlx

**Spec:** `docs/architecture/session-management.md`, `docs/architecture/overview.md`, `.claude/specs/cross-document-constraints.md`

**Prereqs from Phase 1A/1B:** `SessionId`, `ProjectId`, `ModelTier`, `SessionConfig`, `SessionManager`, `CostTracker`, `AppState`, `AutonomicConfig`, `SecretsConfig`, `load_config`, `load_secrets`, `create_pool`, `create_readonly_pool`, `GitManager`, `StateManager`

**Key Constraints:**
- XD-001: Only SessionManager spawns Claude Code
- XD-002: Experience traces to PostgreSQL via SessionManager
- XD-011: Secrets from secrets.toml, not env vars
- clippy.toml: No `std::process::Command`, no `println!`, no `tracing::info!`
- CLI output via `tracing` with bare formatter (no timestamps/levels). CSV export uses `#[allow(clippy::disallowed_macros)]` for stdout.

## File Map

```txt
MODIFY: crates/autonomic-daemon/src/api.rs              # Add POST /sessions, POST /projects, GET /projects
MODIFY: crates/autonomic-daemon/src/main.rs             # Wire db_pool into SessionManager for trace persistence
CREATE: crates/autonomic-daemon/src/traces.rs           # Experience trace persistence to PostgreSQL (XD-002)
CREATE: crates/autonomic-daemon/src/project_registry.rs # Project CRUD against PostgreSQL

MODIFY: crates/autonomic-cli/Cargo.toml                 # Add dependencies
REWRITE: crates/autonomic-cli/src/main.rs               # clap subcommands entrypoint
CREATE: crates/autonomic-cli/src/client.rs              # HTTP client to daemon API
CREATE: crates/autonomic-cli/src/commands/mod.rs        # Subcommand modules
CREATE: crates/autonomic-cli/src/commands/run.rs        # `autonomic run <prompt>`
CREATE: crates/autonomic-cli/src/commands/status.rs     # `autonomic status`
CREATE: crates/autonomic-cli/src/commands/project.rs    # `autonomic project add/list`
CREATE: crates/autonomic-cli/src/commands/metrics.rs    # `autonomic metrics show/export`
CREATE: crates/autonomic-cli/src/commands/snapshot.rs   # `autonomic snapshot <label>`
CREATE: crates/autonomic-cli/src/commands/rollback.rs   # `autonomic rollback --to <tag>`
CREATE: crates/autonomic-cli/src/commands/budget.rs     # `autonomic budget show`

CREATE: infra/launchd/io.sephy.autonomicd.plist         # Daemon Launch Agent
CREATE: infra/launchd/io.sephy.autonomic-watchdog.plist # Watchdog Launch Agent
CREATE: infra/migrations/002_phase1c_indexes.sql        # Additional indexes for project queries

MODIFY: crates/autonomic-watchdog/src/main.rs           # Add frozen-file verification on startup
CREATE: crates/autonomic-watchdog/src/frozen.rs         # Frozen-file permission enforcement
```

## Task 1: Daemon Trace Persistence (XD-002)

**Files:**
- Create: `crates/autonomic-daemon/src/traces.rs`

- [ ] **Step 1: Create traces.rs**

```rust
//! Experience trace persistence to PostgreSQL (XD-002).
//!
//! On session completion, the daemon writes a trace record capturing
//! the session outcome, cost, duration, and raw transcript metadata.

use autonomic_core::{ProjectId, SessionId};
use chrono::Utc;
use sqlx::PgPool;
use ulid::Ulid;

/// Persist an experience trace for a completed session.
pub async fn record_trace(
    pool: &PgPool,
    session_id: &SessionId,
    project_id: &ProjectId,
    model_used: &str,
    outcome: &str,
    duration_ms: i64,
    cost_usd: f64,
    compaction_count: i32,
    trace_json: &serde_json::Value,
) -> Result<(), sqlx::Error> {
    let trace_id = Ulid::new().to_string();
    let session_str = session_id.to_string();
    let project_str = project_id.to_string();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO experience_traces
            (id, session_id, project_id, model_used, outcome, duration_ms, cost_usd, compaction_count, trace_json, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
    )
    .bind(&trace_id)
    .bind(&session_str)
    .bind(&project_str)
    .bind(model_used)
    .bind(outcome)
    .bind(duration_ms)
    .bind(cost_usd)
    .bind(compaction_count)
    .bind(trace_json)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

/// Persist a session record (insert or update).
pub async fn upsert_session(
    pool: &PgPool,
    session_id: &SessionId,
    project_id: &ProjectId,
    model: &str,
    status: &str,
    cost_usd: Option<f64>,
    duration_ms: Option<i64>,
) -> Result<(), sqlx::Error> {
    let session_str = session_id.to_string();
    let project_str = project_id.to_string();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO sessions (id, project_id, model, status, cost_usd, duration_ms, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (id) DO UPDATE SET
            status = EXCLUDED.status,
            cost_usd = EXCLUDED.cost_usd,
            duration_ms = EXCLUDED.duration_ms,
            completed_at = CASE WHEN EXCLUDED.status != 'running' THEN NOW() ELSE sessions.completed_at END
        "#,
    )
    .bind(&session_str)
    .bind(&project_str)
    .bind(model)
    .bind(status)
    .bind(cost_usd)
    .bind(duration_ms)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

/// Fetch recent traces for metrics display.
pub async fn recent_traces(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<TraceRow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, TraceRow>(
        r#"
        SELECT id, session_id, project_id, model_used, outcome,
               duration_ms, cost_usd, compaction_count, created_at
        FROM experience_traces
        ORDER BY created_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// A row from the experience_traces table for display.
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct TraceRow {
    pub id: String,
    pub session_id: String,
    pub project_id: String,
    pub model_used: String,
    pub outcome: String,
    pub duration_ms: Option<i64>,
    pub cost_usd: Option<f64>,
    pub compaction_count: Option<i32>,
    pub created_at: chrono::DateTime<Utc>,
}
```

- [ ] **Step 2: Add missing deps to daemon Cargo.toml**

Add these dependencies (needed for traces.rs, project_registry.rs, and expanded api.rs):
- `chrono = { workspace = true }` — trace timestamps
- `serde_json = { workspace = true }` — JSON trace data
- `sqlx = { workspace = true }` — database queries
- `ulid = { workspace = true }` — trace ID generation

- [ ] **Step 3: Update daemon lib exports**

Add `mod traces;` and `mod project_registry;` to `main.rs` module declarations.

- [ ] **Step 4: Commit**

```bash
git add crates/autonomic-daemon/src/traces.rs crates/autonomic-daemon/Cargo.toml
git commit -m "feat(daemon): add experience trace persistence to PostgreSQL (XD-002)"
```

## Task 2: Project Registry (FR-002 backend)

**Files:**
- Create: `crates/autonomic-daemon/src/project_registry.rs`

- [ ] **Step 1: Create project_registry.rs**

```rust
//! Project registry backed by PostgreSQL (FR-002).
//!
//! Projects are registered by path and tracked with metadata about
//! language, activity, cost, and hook inventory.

use autonomic_core::ProjectId;
use chrono::Utc;
use sqlx::PgPool;

/// Register a new project or update if it already exists.
pub async fn register_project(
    pool: &PgPool,
    path: &str,
    name: &str,
    language: Option<&str>,
) -> Result<String, sqlx::Error> {
    let id = ProjectId::from_path(std::path::Path::new(path));
    let id_str = id.to_string();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO projects (id, path, name, language, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $5)
        ON CONFLICT (path) DO UPDATE SET
            name = EXCLUDED.name,
            language = COALESCE(EXCLUDED.language, projects.language),
            updated_at = $5
        "#,
    )
    .bind(&id_str)
    .bind(path)
    .bind(name)
    .bind(language)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(id_str)
}

/// List all registered projects.
pub async fn list_projects(pool: &PgPool) -> Result<Vec<ProjectRow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, ProjectRow>(
        r#"
        SELECT id, path, name, language, last_activity, last_session_cost_usd,
               hook_count, created_at
        FROM projects
        ORDER BY last_activity DESC NULLS LAST, name
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

/// Update a project's activity metadata after a session completes.
pub async fn update_project_activity(
    pool: &PgPool,
    project_path: &str,
    cost_usd: f64,
) -> Result<(), sqlx::Error> {
    let now = Utc::now();

    sqlx::query(
        r#"
        UPDATE projects
        SET last_activity = $1, last_session_cost_usd = $2, updated_at = $1
        WHERE path = $3
        "#,
    )
    .bind(now)
    .bind(cost_usd)
    .bind(project_path)
    .execute(pool)
    .await?;

    Ok(())
}

/// Auto-discover projects by scanning for `.claude/` directories.
pub async fn discover_projects(root: &std::path::Path) -> Vec<DiscoveredProject> {
    let mut found = Vec::new();
    discover_recursive(root, &mut found, 0, 4).await;
    found
}

/// Discovered project before registration.
#[derive(Debug)]
pub struct DiscoveredProject {
    pub path: std::path::PathBuf,
    pub name: String,
}

async fn discover_recursive(
    dir: &std::path::Path,
    found: &mut Vec<DiscoveredProject>,
    depth: usize,
    max_depth: usize,
) {
    if depth > max_depth {
        return;
    }

    let claude_dir = dir.join(".claude");
    if claude_dir.is_dir() {
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        found.push(DiscoveredProject {
            path: dir.to_path_buf(),
            name,
        });
        return; // Don't recurse into projects
    }

    let Ok(mut entries) = tokio::fs::read_dir(dir).await else {
        return;
    };

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            // Skip hidden dirs, node_modules, target, etc.
            if name.starts_with('.') || name == "node_modules" || name == "target" || name == "vendor" {
                continue;
            }
            Box::pin(discover_recursive(&path, found, depth + 1, max_depth)).await;
        }
    }
}

/// A row from the projects table for display.
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct ProjectRow {
    pub id: String,
    pub path: String,
    pub name: String,
    pub language: Option<String>,
    pub last_activity: Option<chrono::DateTime<Utc>>,
    pub last_session_cost_usd: Option<f64>,
    pub hook_count: i32,
    pub created_at: chrono::DateTime<Utc>,
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/autonomic-daemon/src/project_registry.rs
git commit -m "feat(daemon): add project registry with auto-discovery (FR-002)"
```

## Task 3: Daemon API Expansion (POST /sessions, project endpoints)

**Files:**
- Modify: `crates/autonomic-daemon/src/api.rs`

- [ ] **Step 1: Add new route handlers and request/response types**

Add to `api.rs`:

```rust
use axum::http::StatusCode;
use axum::routing::post;
use autonomic_core::SessionId;
use autonomic_session::SessionConfig;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
```

New request/response types:

```rust
/// POST /api/v1/sessions request body.
#[derive(Debug, serde::Deserialize)]
pub struct RunSessionRequest {
    pub prompt: String,
    pub working_dir: String,
    #[serde(default = "default_model")]
    pub model: String,
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_cost_budget")]
    pub cost_budget_usd: f64,
}

fn default_model() -> String { "sonnet".to_string() }
fn default_timeout() -> u64 { 1800 }
fn default_cost_budget() -> f64 { 5.0 }

#[derive(Debug, Serialize)]
pub struct RunSessionResponse {
    pub session_id: String,
    pub status: String,
    pub cost_usd: f64,
    pub duration_ms: u64,
    pub outcome: Option<String>,
    pub error: Option<String>,
}

/// POST /api/v1/projects request body.
#[derive(Debug, serde::Deserialize)]
pub struct AddProjectRequest {
    pub path: String,
    pub name: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AddProjectResponse {
    pub id: String,
    pub path: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectListResponse {
    pub count: usize,
    pub projects: Vec<crate::project_registry::ProjectRow>,
}

/// GET /api/v1/metrics response.
#[derive(Debug, Serialize)]
pub struct MetricsResponse {
    pub count: usize,
    pub traces: Vec<crate::traces::TraceRow>,
}
```

New handlers:

```rust
async fn run_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RunSessionRequest>,
) -> Result<Json<RunSessionResponse>, (StatusCode, String)> {
    let session_id = SessionId::new();
    let working_dir = PathBuf::from(&req.working_dir);

    if !working_dir.is_absolute() || !working_dir.is_dir() {
        return Err((StatusCode::BAD_REQUEST, format!("Invalid working_dir: {}", req.working_dir)));
    }

    let config = SessionConfig {
        session_id: session_id.clone(),
        working_dir,
        prompt: req.prompt,
        model: req.model,
        system_prompt: req.system_prompt,
        allowed_tools: req.allowed_tools,
        timeout: Duration::from_secs(req.timeout_seconds),
        cost_budget_usd: req.cost_budget_usd,
        resume_session_id: None,
        continue_recent: false,
        extra_env: HashMap::new(),
        agent_teams: false,
        worktree: None,
    };

    let started = std::time::Instant::now();
    let record = state.session_manager.run_session(&config).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let duration_ms = started.elapsed().as_millis() as u64;

    // Persist trace to PostgreSQL if pool is available (XD-002).
    if let Some(ref pool) = state.db_pool {
        let project_id = autonomic_core::ProjectId::from_path(&PathBuf::from(&config.working_dir));
        let (outcome_str, cost) = match &record.state {
            autonomic_session::SessionState::Completed { outcome, .. } => {
                let (o, c) = match outcome {
                    autonomic_session::SessionOutcome::Success { cost_usd, .. } => ("success", *cost_usd),
                    autonomic_session::SessionOutcome::Error { cost_usd, .. } => ("error", *cost_usd),
                    autonomic_session::SessionOutcome::MaxTurns { cost_usd, .. } => ("max_turns", *cost_usd),
                };
                (o, c)
            }
            autonomic_session::SessionState::Failed { error, .. } => ("failed", 0.0),
            autonomic_session::SessionState::TimedOut { accumulated_cost_usd, .. } => ("timed_out", *accumulated_cost_usd),
            _ => ("unknown", 0.0),
        };

        // Best-effort persistence — don't fail the response if DB write fails.
        let trace_json = serde_json::json!({
            "transcript_lines": record.transcript.len(),
            "stderr_lines": record.stderr_lines.len(),
        });
        if let Err(e) = crate::traces::upsert_session(
            pool, &session_id, &project_id, &config.model, outcome_str, Some(cost), Some(duration_ms as i64),
        ).await {
            tracing::warn!(error = %e, "failed to persist session record");
        }
        if let Err(e) = crate::traces::record_trace(
            pool, &session_id, &project_id, &config.model, outcome_str,
            duration_ms as i64, cost, 0, &trace_json,
        ).await {
            tracing::warn!(error = %e, "failed to persist experience trace");
        }
    }

    let (status_str, outcome_str, error_str, cost) = match &record.state {
        autonomic_session::SessionState::Completed { outcome, .. } => match outcome {
            autonomic_session::SessionOutcome::Success { cost_usd, .. } =>
                ("completed", Some("success".to_string()), None, *cost_usd),
            autonomic_session::SessionOutcome::Error { cost_usd, error, .. } =>
                ("completed", Some("error".to_string()), Some(error.clone()), *cost_usd),
            autonomic_session::SessionOutcome::MaxTurns { cost_usd, .. } =>
                ("completed", Some("max_turns".to_string()), None, *cost_usd),
        },
        autonomic_session::SessionState::Failed { error, .. } =>
            ("failed", None, Some(error.clone()), 0.0),
        autonomic_session::SessionState::TimedOut { accumulated_cost_usd, .. } =>
            ("timed_out", None, Some("timeout exceeded".to_string()), *accumulated_cost_usd),
        _ => ("unknown", None, None, 0.0),
    };

    Ok(Json(RunSessionResponse {
        session_id: session_id.to_string(),
        status: status_str.to_string(),
        cost_usd: cost,
        duration_ms,
        outcome: outcome_str,
        error: error_str,
    }))
}

async fn add_project(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddProjectRequest>,
) -> Result<Json<AddProjectResponse>, (StatusCode, String)> {
    let pool = state.db_pool.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "Database not available".to_string(),
    ))?;

    let path = std::path::Path::new(&req.path);
    if !path.is_absolute() || !path.is_dir() {
        return Err((StatusCode::BAD_REQUEST, format!("Invalid path: {}", req.path)));
    }

    let name = req.name.unwrap_or_else(|| {
        path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    });

    let id = crate::project_registry::register_project(pool, &req.path, &name, req.language.as_deref())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AddProjectResponse {
        id,
        path: req.path,
        name,
    }))
}

async fn list_projects(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ProjectListResponse>, (StatusCode, String)> {
    let pool = state.db_pool.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "Database not available".to_string(),
    ))?;

    let projects = crate::project_registry::list_projects(pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(ProjectListResponse {
        count: projects.len(),
        projects,
    }))
}

async fn metrics(
    State(state): State<Arc<AppState>>,
) -> Result<Json<MetricsResponse>, (StatusCode, String)> {
    let pool = state.db_pool.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "Database not available".to_string(),
    ))?;

    let traces = crate::traces::recent_traces(pool, 50)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(MetricsResponse {
        count: traces.len(),
        traces,
    }))
}
```

- [ ] **Step 2: Update build_router to include new routes**

```rust
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/status", get(status))
        .route("/api/v1/sessions", get(sessions).post(run_session))
        .route("/api/v1/budget", get(budget))
        .route("/api/v1/projects", get(list_projects).post(add_project))
        .route("/api/v1/metrics", get(metrics))
        .with_state(state)
}
```

- [ ] **Step 3: Remove `#[allow(dead_code)]` from db_pool in AppState** (it's used now)

- [ ] **Step 4: Run cargo check**

```bash
SQLX_OFFLINE=true cargo check -p autonomic-daemon
```

- [ ] **Step 5: Commit**

```bash
git add crates/autonomic-daemon/
git commit -m "feat(daemon): add POST /sessions, project registry, and metrics API endpoints"
```

## Task 4: CLI Crate Setup + Client

**Files:**
- Modify: `crates/autonomic-cli/Cargo.toml`
- Rewrite: `crates/autonomic-cli/src/main.rs`
- Create: `crates/autonomic-cli/src/client.rs`
- Create: `crates/autonomic-cli/src/commands/mod.rs`

- [ ] **Step 1: Update Cargo.toml**

```toml
[package]
name = "autonomic-cli"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "CLI interface for the Autonomic orchestrator"

[[bin]]
name = "autonomic"
path = "src/main.rs"

[dependencies]
autonomic-core = { path = "../autonomic-core" }
clap = { workspace = true }
dirs = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }

[lints]
workspace = true
```

- [ ] **Step 2: Create client.rs**

```rust
//! HTTP client for communicating with the autonomicd daemon.

use std::time::Duration;

/// A lightweight HTTP client that talks to the daemon API.
///
/// Uses raw TCP + hand-crafted HTTP/1.1 to avoid pulling in reqwest
/// (same pattern as the watchdog health check).
pub struct DaemonClient {
    base_url: String,
}

impl DaemonClient {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            base_url: format!("http://{host}:{port}"),
        }
    }

    /// Send a GET request and return the response body as a string.
    pub async fn get(&self, path: &str) -> Result<String, ClientError> {
        let url = format!("{}{}", self.base_url, path);
        let response = http_request("GET", &url, None).await?;
        Ok(response)
    }

    /// Send a POST request with a JSON body and return the response body.
    pub async fn post(&self, path: &str, body: &str) -> Result<String, ClientError> {
        let url = format!("{}{}", self.base_url, path);
        let response = http_request("POST", &url, Some(body)).await?;
        Ok(response)
    }
}

#[derive(Debug)]
pub enum ClientError {
    Connection(String),
    Http(u16, String),
    Parse(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connection(e) => write!(f, "cannot connect to daemon: {e}"),
            Self::Http(code, body) => write!(f, "HTTP {code}: {body}"),
            Self::Parse(e) => write!(f, "parse error: {e}"),
        }
    }
}

/// Minimal async HTTP/1.1 client using raw TCP.
async fn http_request(method: &str, url: &str, body: Option<&str>) -> Result<String, ClientError> {
    // Parse URL to extract host, port, path.
    let url = url.strip_prefix("http://").unwrap_or(url);
    let (host_port, path) = url.split_once('/').unwrap_or((url, ""));
    let path = format!("/{path}");
    let (host, port_str) = host_port.rsplit_once(':').unwrap_or((host_port, "80"));
    let port: u16 = port_str.parse().unwrap_or(80);

    let addr = format!("{host}:{port}");
    let stream = tokio::time::timeout(
        Duration::from_secs(5),
        tokio::net::TcpStream::connect(&addr),
    )
    .await
    .map_err(|_| ClientError::Connection("timeout connecting to daemon".to_string()))?
    .map_err(|e| ClientError::Connection(e.to_string()))?;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut stream = stream;

    let content_length = body.map(|b| b.len()).unwrap_or(0);
    let request = if let Some(body) = body {
        format!(
            "{method} {path} HTTP/1.1\r\nHost: {host_port}\r\nContent-Type: application/json\r\nContent-Length: {content_length}\r\nConnection: close\r\n\r\n{body}"
        )
    } else {
        format!(
            "{method} {path} HTTP/1.1\r\nHost: {host_port}\r\nConnection: close\r\n\r\n"
        )
    };

    stream.write_all(request.as_bytes()).await
        .map_err(|e| ClientError::Connection(e.to_string()))?;

    let mut response = String::new();
    stream.read_to_string(&mut response).await
        .map_err(|e| ClientError::Connection(e.to_string()))?;

    // Parse HTTP response: status line + headers + body.
    let (headers, body) = response.split_once("\r\n\r\n").unwrap_or(("", &response));
    let status_line = headers.lines().next().unwrap_or("");
    let status_code: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if status_code >= 400 {
        return Err(ClientError::Http(status_code, body.to_string()));
    }

    Ok(body.to_string())
}
```

- [ ] **Step 3: Create commands/mod.rs**

```rust
pub mod budget;
pub mod metrics;
pub mod project;
pub mod rollback;
pub mod run;
pub mod snapshot;
pub mod status;
```

- [ ] **Step 4: Rewrite main.rs with clap**

```rust
//! CLI interface for the Autonomic orchestrator (`autonomic`).

mod client;
mod commands;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "autonomic", version, about = "Autonomic orchestrator CLI")]
struct Cli {
    /// Daemon host address.
    #[arg(long, default_value = "127.0.0.1", global = true)]
    host: String,

    /// Daemon port.
    #[arg(long, default_value = "7700", global = true)]
    port: u16,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a Claude Code session via the daemon.
    Run(commands::run::RunArgs),
    /// Show daemon and session status.
    Status,
    /// Manage registered projects.
    Project {
        #[command(subcommand)]
        command: commands::project::ProjectCommand,
    },
    /// Display session metrics and traces.
    Metrics(commands::metrics::MetricsArgs),
    /// Create a named state snapshot (git tag).
    Snapshot(commands::snapshot::SnapshotArgs),
    /// Rollback state to a previous snapshot.
    Rollback(commands::rollback::RollbackArgs),
    /// Show rate budget utilization.
    Budget,
}

#[tokio::main]
async fn main() {
    // Bare formatter for CLI: no timestamps, no levels, no targets — just the message.
    // This lets us use tracing::info!() for user output without log noise.
    // RUST_LOG override still works for debugging.
    tracing_subscriber::fmt()
        .with_target(false)
        .without_time()
        .with_level(false)
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let cli = Cli::parse();
    let client = client::DaemonClient::new(&cli.host, cli.port);

    let result = match cli.command {
        Commands::Run(args) => commands::run::execute(&client, args).await,
        Commands::Status => commands::status::execute(&client).await,
        Commands::Project { command } => commands::project::execute(&client, command).await,
        Commands::Metrics(args) => commands::metrics::execute(&client, args).await,
        Commands::Snapshot(args) => commands::snapshot::execute(args).await,
        Commands::Rollback(args) => commands::rollback::execute(args).await,
        Commands::Budget => commands::budget::execute(&client).await,
    };

    if let Err(e) = result {
        tracing::error!("{e}");
        std::process::exit(1);
    }
}
```

- [ ] **Step 5: Commit**

```bash
git add crates/autonomic-cli/
git commit -m "feat(cli): add clap skeleton with subcommands and daemon HTTP client"
```

## Task 5: CLI Subcommands — run, status, budget

**Files:**
- Create: `crates/autonomic-cli/src/commands/run.rs`
- Create: `crates/autonomic-cli/src/commands/status.rs`
- Create: `crates/autonomic-cli/src/commands/budget.rs`

- [ ] **Step 1: Create run.rs**

```rust
//! `autonomic run <prompt>` — run a Claude session via the daemon.

use crate::client::DaemonClient;
use clap::Args;

#[derive(Args)]
pub struct RunArgs {
    /// The prompt to send to Claude Code.
    pub prompt: String,

    /// Working directory for the session.
    #[arg(short = 'd', long, default_value = ".")]
    pub working_dir: String,

    /// Model to use (haiku, sonnet, opus).
    #[arg(short, long, default_value = "sonnet")]
    pub model: String,

    /// Timeout in seconds.
    #[arg(short, long, default_value = "1800")]
    pub timeout: u64,

    /// Cost budget in USD.
    #[arg(short, long, default_value = "5.0")]
    pub budget: f64,

    /// System prompt override.
    #[arg(long)]
    pub system_prompt: Option<String>,
}

pub async fn execute(client: &DaemonClient, args: RunArgs) -> Result<(), String> {
    let working_dir = if args.working_dir == "." {
        std::env::current_dir()
            .map_err(|e| format!("cannot determine working directory: {e}"))?
            .to_string_lossy()
            .to_string()
    } else {
        args.working_dir
    };

    let body = serde_json::json!({
        "prompt": args.prompt,
        "working_dir": working_dir,
        "model": args.model,
        "timeout_seconds": args.timeout,
        "cost_budget_usd": args.budget,
        "system_prompt": args.system_prompt,
    });

    tracing::info!("Starting session (model={}, timeout={}s, budget=${:.2})...", args.model, args.timeout, args.budget);

    let response = client
        .post("/api/v1/sessions", &body.to_string())
        .await
        .map_err(|e| e.to_string())?;

    let result: serde_json::Value = serde_json::from_str(&response)
        .map_err(|e| format!("invalid response: {e}"))?;

    let status = result["status"].as_str().unwrap_or("unknown");
    let session_id = result["session_id"].as_str().unwrap_or("?");
    let cost = result["cost_usd"].as_f64().unwrap_or(0.0);
    let duration = result["duration_ms"].as_u64().unwrap_or(0);

    tracing::info!("\nSession {session_id}");
    tracing::info!("  Status:   {status}");
    tracing::info!("  Cost:     ${cost:.4}");
    tracing::info!("  Duration: {:.1}s", duration as f64 / 1000.0);

    if let Some(outcome) = result["outcome"].as_str() {
        tracing::info!("  Outcome:  {outcome}");
    }
    if let Some(error) = result["error"].as_str() {
        tracing::info!("  Error:    {error}");
    }

    if status == "failed" || status == "timed_out" {
        Err(format!("session {status}"))
    } else {
        Ok(())
    }
}
```

- [ ] **Step 2: Create status.rs**

```rust
//! `autonomic status` — show daemon and session status.

use crate::client::DaemonClient;

pub async fn execute(client: &DaemonClient) -> Result<(), String> {
    let response = client.get("/api/v1/status").await.map_err(|e| e.to_string())?;

    let status: serde_json::Value = serde_json::from_str(&response)
        .map_err(|e| format!("invalid response: {e}"))?;

    let version = status["version"].as_str().unwrap_or("?");
    let uptime = status["uptime_seconds"].as_u64().unwrap_or(0);
    let active = status["active_sessions"].as_u64().unwrap_or(0);
    let running = status["running_sessions"].as_u64().unwrap_or(0);
    let cost = status["total_cost_usd"].as_f64().unwrap_or(0.0);

    tracing::info!("autonomicd v{version}");
    tracing::info!("  Uptime:           {}m {}s", uptime / 60, uptime % 60);
    tracing::info!("  Active sessions:  {active}");
    tracing::info!("  Running sessions: {running}");
    tracing::info!("  Total cost:       ${cost:.4}");

    Ok(())
}
```

- [ ] **Step 3: Create budget.rs**

```rust
//! `autonomic budget` — show rate budget utilization.

use crate::client::DaemonClient;

pub async fn execute(client: &DaemonClient) -> Result<(), String> {
    let response = client.get("/api/v1/budget").await.map_err(|e| e.to_string())?;

    let budget: serde_json::Value = serde_json::from_str(&response)
        .map_err(|e| format!("invalid response: {e}"))?;

    let total = budget["total_cost_usd"].as_f64().unwrap_or(0.0);
    let window = budget["window_cost_usd"].as_f64().unwrap_or(0.0);
    let sessions = budget["session_count"].as_u64().unwrap_or(0);

    tracing::info!("Rate Budget (5-hour sliding window)");
    tracing::info!("  Window cost:    ${window:.4}");
    tracing::info!("  Total cost:     ${total:.4}");
    tracing::info!("  Session count:  {sessions}");

    Ok(())
}
```

- [ ] **Step 4: Commit**

```bash
git add crates/autonomic-cli/src/commands/run.rs crates/autonomic-cli/src/commands/status.rs crates/autonomic-cli/src/commands/budget.rs
git commit -m "feat(cli): add run, status, and budget commands"
```

## Task 6: CLI Subcommands — project, metrics

**Files:**
- Create: `crates/autonomic-cli/src/commands/project.rs`
- Create: `crates/autonomic-cli/src/commands/metrics.rs`

- [ ] **Step 1: Create project.rs**

```rust
//! `autonomic project add/list` — manage registered projects (FR-002).

use crate::client::DaemonClient;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum ProjectCommand {
    /// Register a project by path.
    Add {
        /// Path to the project directory.
        path: String,
        /// Optional project name (defaults to directory name).
        #[arg(short, long)]
        name: Option<String>,
        /// Primary language.
        #[arg(short, long)]
        language: Option<String>,
    },
    /// List all registered projects.
    List,
}

pub async fn execute(client: &DaemonClient, command: ProjectCommand) -> Result<(), String> {
    match command {
        ProjectCommand::Add { path, name, language } => {
            let abs_path = if std::path::Path::new(&path).is_absolute() {
                path
            } else {
                std::env::current_dir()
                    .map_err(|e| e.to_string())?
                    .join(&path)
                    .to_string_lossy()
                    .to_string()
            };

            let body = serde_json::json!({
                "path": abs_path,
                "name": name,
                "language": language,
            });

            let response = client
                .post("/api/v1/projects", &body.to_string())
                .await
                .map_err(|e| e.to_string())?;

            let result: serde_json::Value = serde_json::from_str(&response)
                .map_err(|e| format!("invalid response: {e}"))?;

            let id = result["id"].as_str().unwrap_or("?");
            let name = result["name"].as_str().unwrap_or("?");
            tracing::info!("Registered project '{name}' (id: {id})");
        }
        ProjectCommand::List => {
            let response = client.get("/api/v1/projects").await.map_err(|e| e.to_string())?;

            let result: serde_json::Value = serde_json::from_str(&response)
                .map_err(|e| format!("invalid response: {e}"))?;

            let count = result["count"].as_u64().unwrap_or(0);
            tracing::info!("{count} registered project(s)\n");

            if let Some(projects) = result["projects"].as_array() {
                for p in projects {
                    let name = p["name"].as_str().unwrap_or("?");
                    let path = p["path"].as_str().unwrap_or("?");
                    let lang = p["language"].as_str().unwrap_or("-");
                    let cost = p["last_session_cost_usd"].as_f64();

                    tracing::info!("  {name}");
                    tracing::info!("    Path:     {path}");
                    tracing::info!("    Language: {lang}");
                    if let Some(c) = cost {
                        tracing::info!("    Last cost: ${c:.4}");
                    }
                    tracing::info!();
                }
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Create metrics.rs**

```rust
//! `autonomic metrics show/export` — display session metrics (FR-005).

use crate::client::DaemonClient;
use clap::Args;

#[derive(Args)]
pub struct MetricsArgs {
    /// Export format: "table" (default) or "csv".
    #[arg(long, default_value = "table")]
    pub format: String,

    /// Maximum number of traces to show.
    #[arg(short, long, default_value = "20")]
    pub limit: usize,
}

pub async fn execute(client: &DaemonClient, args: MetricsArgs) -> Result<(), String> {
    let response = client.get("/api/v1/metrics").await.map_err(|e| e.to_string())?;

    let result: serde_json::Value = serde_json::from_str(&response)
        .map_err(|e| format!("invalid response: {e}"))?;

    let traces = result["traces"].as_array().ok_or("no traces in response")?;

    if args.format == "csv" {
        // CSV export must go to stdout for piping. Allow println! here only.
        #[allow(clippy::disallowed_macros)]
        {
        println!("session_id,project_id,model,outcome,duration_ms,cost_usd,created_at");
        for t in traces.iter().take(args.limit) {
            println!(
                "{},{},{},{},{},{},{}",
                t["session_id"].as_str().unwrap_or(""),
                t["project_id"].as_str().unwrap_or(""),
                t["model_used"].as_str().unwrap_or(""),
                t["outcome"].as_str().unwrap_or(""),
                t["duration_ms"].as_i64().unwrap_or(0),
                t["cost_usd"].as_f64().unwrap_or(0.0),
                t["created_at"].as_str().unwrap_or(""),
            );
        }
    } else {
        let count = traces.len().min(args.limit);
        tracing::info!("Recent session traces ({count} shown)\n");

        for t in traces.iter().take(args.limit) {
            let session = t["session_id"].as_str().unwrap_or("?");
            let model = t["model_used"].as_str().unwrap_or("?");
            let outcome = t["outcome"].as_str().unwrap_or("?");
            let duration = t["duration_ms"].as_i64().unwrap_or(0);
            let cost = t["cost_usd"].as_f64().unwrap_or(0.0);
            let created = t["created_at"].as_str().unwrap_or("?");

            tracing::info!("  {session}  {model}  {outcome}  {:.1}s  ${cost:.4}  {created}", duration as f64 / 1000.0);
        }
    }

    Ok(())
}
```

- [ ] **Step 3: Commit**

```bash
git add crates/autonomic-cli/src/commands/project.rs crates/autonomic-cli/src/commands/metrics.rs
git commit -m "feat(cli): add project and metrics commands (FR-002, FR-005)"
```

## Task 7: CLI Subcommands — snapshot, rollback (FR-004)

**Files:**
- Create: `crates/autonomic-cli/src/commands/snapshot.rs`
- Create: `crates/autonomic-cli/src/commands/rollback.rs`

These run locally (not via daemon) because they operate on `~/.autonomic/` git state.

- [ ] **Step 1: Create snapshot.rs**

```rust
//! `autonomic snapshot <label>` — create a named state snapshot (FR-004).

use clap::Args;
use tokio::process::Command;

#[derive(Args)]
pub struct SnapshotArgs {
    /// Label for the snapshot (used as git tag name).
    pub label: String,
}

#[allow(clippy::disallowed_methods)] // Startup utility, not daemon runtime.
pub async fn execute(args: SnapshotArgs) -> Result<(), String> {
    let state_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".autonomic");

    if !state_dir.join(".git").exists() {
        return Err("State directory is not initialized. Start the daemon first.".to_string());
    }

    let tag_name = format!("snapshot/{}", args.label);

    let output = Command::new("git")
        .args(["tag", "-a", &tag_name, "-m", &format!("Manual snapshot: {}", args.label)])
        .current_dir(&state_dir)
        .output()
        .await
        .map_err(|e| format!("failed to create tag: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git tag failed: {stderr}"));
    }

    tracing::info!("Snapshot created: {tag_name}");
    Ok(())
}
```

- [ ] **Step 2: Create rollback.rs**

```rust
//! `autonomic rollback --to <tag>` — rollback state to a previous snapshot (FR-004).

use clap::Args;
use tokio::process::Command;

#[derive(Args)]
pub struct RollbackArgs {
    /// Tag or commit to rollback to.
    #[arg(long)]
    pub to: String,
}

#[allow(clippy::disallowed_methods)] // Startup utility, not daemon runtime.
pub async fn execute(args: RollbackArgs) -> Result<(), String> {
    let state_dir = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".autonomic");

    if !state_dir.join(".git").exists() {
        return Err("State directory is not initialized. Start the daemon first.".to_string());
    }

    // Verify the target exists.
    let check = Command::new("git")
        .args(["rev-parse", "--verify", &args.to])
        .current_dir(&state_dir)
        .output()
        .await
        .map_err(|e| format!("failed to verify target: {e}"))?;

    if !check.status.success() {
        return Err(format!("Target '{}' not found. Use `git -C ~/.autonomic tag -l` to see available tags.", args.to));
    }

    // Create pre-rollback snapshot.
    let _ = Command::new("git")
        .args(["tag", "-a", "pre-rollback/cli", "-m", "Pre-rollback snapshot (CLI)"])
        .current_dir(&state_dir)
        .output()
        .await;

    // Rollback tracked files only (XD-008: preserve gitignored files).
    let output = Command::new("git")
        .args(["checkout", &args.to, "--", "."])
        .current_dir(&state_dir)
        .output()
        .await
        .map_err(|e| format!("rollback failed: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git checkout failed: {stderr}"));
    }

    // Commit the rollback.
    // XD-008: Only stage tracked files — never `git add .` which would stage untracked/gitignored files.
    let _ = Command::new("git")
        .args(["add", "-u"])
        .current_dir(&state_dir)
        .output()
        .await;

    let _ = Command::new("git")
        .args(["commit", "-m", &format!("rollback: restore to {} (CLI)", args.to)])
        .current_dir(&state_dir)
        .output()
        .await;

    tracing::info!("Rolled back to '{}'. Pre-rollback state tagged as 'pre-rollback/cli'.", args.to);
    Ok(())
}
```

- [ ] **Step 3: Commit**

```bash
git add crates/autonomic-cli/src/commands/snapshot.rs crates/autonomic-cli/src/commands/rollback.rs
git commit -m "feat(cli): add snapshot and rollback commands (FR-004, XD-008)"
```

## Task 8: launchd Plists (FR-001)

**Files:**
- Create: `infra/launchd/io.sephy.autonomicd.plist`
- Create: `infra/launchd/io.sephy.autonomic-watchdog.plist`

**Note:** These plists use the developer's home directory (`/Users/sephyi`). On installation, paths must be adjusted to match the target user's `$HOME` and `$CARGO_HOME`. A future `autonomic install` command could generate these dynamically.

- [ ] **Step 1: Create daemon plist**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>io.sephy.autonomicd</string>

    <key>ProgramArguments</key>
    <array>
        <string>/Users/sephyi/.cargo/bin/autonomicd</string>
    </array>

    <key>RunAtLoad</key>
    <true/>

    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>

    <key>ThrottleInterval</key>
    <integer>10</integer>

    <key>StandardOutPath</key>
    <string>/Users/sephyi/.autonomic/logs/autonomicd.stdout.log</string>

    <key>StandardErrorPath</key>
    <string>/Users/sephyi/.autonomic/logs/autonomicd.stderr.log</string>

    <key>WorkingDirectory</key>
    <string>/Users/sephyi/.autonomic</string>

    <key>EnvironmentVariables</key>
    <dict>
        <key>HOME</key>
        <string>/Users/sephyi</string>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:/Users/sephyi/.cargo/bin</string>
    </dict>
</dict>
</plist>
```

- [ ] **Step 2: Create watchdog plist**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>io.sephy.autonomic-watchdog</string>

    <key>ProgramArguments</key>
    <array>
        <string>/Users/sephyi/.cargo/bin/autonomic-watchdog</string>
    </array>

    <key>RunAtLoad</key>
    <true/>

    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>

    <key>ThrottleInterval</key>
    <integer>10</integer>

    <key>StandardOutPath</key>
    <string>/Users/sephyi/.autonomic/logs/watchdog.stdout.log</string>

    <key>StandardErrorPath</key>
    <string>/Users/sephyi/.autonomic/logs/watchdog.stderr.log</string>

    <key>WorkingDirectory</key>
    <string>/Users/sephyi/.autonomic</string>

    <key>EnvironmentVariables</key>
    <dict>
        <key>HOME</key>
        <string>/Users/sephyi</string>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:/Users/sephyi/.cargo/bin</string>
    </dict>
</dict>
</plist>
```

- [ ] **Step 3: Commit**

```bash
git add infra/launchd/
git commit -m "feat: add launchd plists for daemon and watchdog (FR-001)"
```

## Task 9: Frozen-File Enforcement (FR-001)

**Files:**
- Create: `crates/autonomic-watchdog/src/frozen.rs`
- Modify: `crates/autonomic-watchdog/src/main.rs`

- [ ] **Step 1: Create frozen.rs**

```rust
//! Frozen-file permission enforcement.
//!
//! The modification frontier is enforced at filesystem level: frozen files
//! are set read-only (chmod 444). The watchdog verifies permissions on
//! startup and restores them if tampered with.
//!
//! Frozen files include the evolution engine's core logic, safety predicates,
//! and energy function — anything outside the modification frontier.

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// List of files that must be frozen (read-only, mode 0o444).
///
/// These are relative to the state directory (~/.autonomic/).
/// In Phase 4, the evolution engine will manage this list dynamically.
/// For now, a static list is sufficient.
const FROZEN_PATTERNS: &[&str] = &[
    // Evolution engine core (Phase 4 — these won't exist yet but the check is harmless)
    "evolution/energy.toml",
    "evolution/frontier.toml",
    "evolution/safety_predicates.toml",
];

/// Verify that all frozen files have correct permissions (0o444).
/// Returns a list of files that were repaired.
pub async fn verify_frozen_files(state_dir: &Path) -> Vec<PathBuf> {
    let mut repaired = Vec::new();

    for pattern in FROZEN_PATTERNS {
        let path = state_dir.join(pattern);
        if !path.exists() {
            continue;
        }

        match tokio::fs::metadata(&path).await {
            Ok(meta) => {
                let mode = meta.permissions().mode() & 0o777;
                if mode != 0o444 {
                    tracing::warn!(
                        path = %path.display(),
                        current_mode = format!("{mode:o}"),
                        "frozen file has wrong permissions, restoring to 444"
                    );
                    let perms = std::fs::Permissions::from_mode(0o444);
                    if let Err(e) = tokio::fs::set_permissions(&path, perms).await {
                        tracing::error!(
                            path = %path.display(),
                            error = %e,
                            "failed to restore frozen file permissions"
                        );
                    } else {
                        repaired.push(path);
                    }
                } else {
                    tracing::debug!(path = %path.display(), "frozen file permissions OK");
                }
            }
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "cannot read frozen file metadata"
                );
            }
        }
    }

    repaired
}
```

- [ ] **Step 2: Add frozen module and call in main.rs**

Add `mod frozen;` to main.rs, and call `frozen::verify_frozen_files` after orphan sweep on startup:

```rust
// Phase 1.5: Verify frozen file permissions.
let repaired = frozen::verify_frozen_files(&state_dir).await;
if !repaired.is_empty() {
    tracing::warn!(count = repaired.len(), "repaired frozen file permissions");
}
```

- [ ] **Step 3: Commit**

```bash
git add crates/autonomic-watchdog/
git commit -m "feat(watchdog): add frozen-file permission enforcement on startup (FR-001)"
```

## Task 10: Integration Verification and Milestone Tag

- [ ] **Step 1: Verify full workspace builds**

```bash
SQLX_OFFLINE=true cargo check --workspace
```

- [ ] **Step 2: Run all tests**

```bash
cargo test -p autonomic-container && cargo test -p autonomic-session && cargo test -p autonomic-core
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
grep "autonomic-" crates/autonomic-cli/Cargo.toml   # only core
```

- [ ] **Step 6: Verify end-to-end flow** (manual, requires running daemon + Postgres)

```bash
# Terminal 1: Start infrastructure
podman compose -f infra/compose.yaml up -d

# Terminal 2: Start daemon
cargo run --bin autonomicd

# Terminal 3: Test CLI
cargo run --bin autonomic -- status
cargo run --bin autonomic -- project add .
cargo run --bin autonomic -- project list
cargo run --bin autonomic -- budget
cargo run --bin autonomic -- run "say hello"
cargo run --bin autonomic -- metrics --limit 5
cargo run --bin autonomic -- snapshot test-1
```

- [ ] **Step 7: Tag milestone**

```bash
git tag -a milestone/phase-1c -m "Phase 1C: CLI, project registry, metrics, launchd, frozen files"
git tag -a milestone/phase-1 -m "Phase 1 complete: Foundation (FR-001 through FR-005)"
```
