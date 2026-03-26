//! HTTP API for the daemon.
//!
//! Provides health, status, session, and budget endpoints for the CLI
//! and external monitoring tools.

use std::sync::Arc;
use std::time::Instant;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use autonomic_session::{CostTracker, SessionManager};

/// Shared application state for all API handlers.
pub struct AppState {
    /// The global session manager (XD-001: only way to spawn Claude).
    pub session_manager: Arc<SessionManager>,
    /// Global cost tracker with sliding-window budget.
    pub cost_tracker: Arc<CostTracker>,
    /// When the daemon process started.
    pub start_time: Instant,
    /// PostgreSQL connection pool (XD-002: experience traces to Postgres).
    /// Used by POST /api/v1/sessions (Phase 1C) and trace persistence.
    #[allow(dead_code)]
    pub db_pool: Option<autonomic_db::PgPool>,
}

/// Build the axum router with all API routes.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/status", get(status))
        .route("/api/v1/sessions", get(sessions))
        .route("/api/v1/budget", get(budget))
        .with_state(state)
}

/// GET /health
#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    uptime_seconds: u64,
    pid: u32,
}

async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        uptime_seconds: state.start_time.elapsed().as_secs(),
        pid: std::process::id(),
    })
}

/// GET /api/v1/status
#[derive(Debug, Serialize)]
struct StatusResponse {
    version: &'static str,
    uptime_seconds: u64,
    active_sessions: usize,
    running_sessions: usize,
    total_cost_usd: f64,
}

async fn status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    Json(StatusResponse {
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: state.start_time.elapsed().as_secs(),
        active_sessions: state.session_manager.active_sessions().len(),
        running_sessions: state.session_manager.running_count(),
        total_cost_usd: state.cost_tracker.total_cost(),
    })
}

/// GET /api/v1/sessions
#[derive(Debug, Serialize)]
struct SessionsResponse {
    count: usize,
    session_ids: Vec<String>,
}

async fn sessions(State(state): State<Arc<AppState>>) -> Json<SessionsResponse> {
    let ids = state.session_manager.active_sessions();
    Json(SessionsResponse {
        count: ids.len(),
        session_ids: ids.into_iter().map(|id| id.to_string()).collect(),
    })
}

/// GET /api/v1/budget
#[derive(Debug, Serialize)]
struct BudgetResponse {
    total_cost_usd: f64,
    window_cost_usd: f64,
    session_count: usize,
}

async fn budget(State(state): State<Arc<AppState>>) -> Json<BudgetResponse> {
    Json(BudgetResponse {
        total_cost_usd: state.cost_tracker.total_cost(),
        window_cost_usd: state.cost_tracker.window_total(),
        session_count: state.cost_tracker.session_count(),
    })
}
