//! Autonomic orchestrator daemon (`autonomicd`).
//!
//! Manages Claude Code sessions, tracks costs, and exposes an HTTP API
//! for the CLI and monitoring tools.

mod api;
mod pid;
mod shutdown;
mod tracing_setup;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use autonomic_container::HostRuntime;
use autonomic_core::{load_config, load_secrets};
use autonomic_session::{CostTracker, SessionManager};
use tokio::net::TcpListener;

use crate::api::AppState;

#[tokio::main]
async fn main() {
    let start_time = Instant::now();

    // 1. Determine state directory.
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let state_dir = home.join(".autonomic");

    // 2. Initialize tracing (pre-async, blocking I/O acceptable).
    let log_dir = state_dir.join("logs");
    if let Err(e) = tracing_setup::init_tracing(&log_dir) {
        // Fall back to minimal stderr logging if tracing setup fails.
        let fallback_filter = tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,autonomic=debug"));
        tracing_subscriber::fmt()
            .compact()
            .with_env_filter(fallback_filter)
            .init();
        tracing::warn!(error = %e, "failed to initialize file logging, using stderr only");
    }

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        state_dir = %state_dir.display(),
        "autonomicd starting"
    );

    // 3. Load config and secrets.
    let config_path = state_dir.join("config.toml");
    let config = match load_config(&config_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "failed to load config, using defaults");
            load_config(std::path::Path::new("/nonexistent")).expect("default config infallible")
        }
    };

    let secrets_path = state_dir.join("secrets.toml");
    let secrets = match load_secrets(&secrets_path) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "failed to load secrets, using defaults");
            load_secrets(std::path::Path::new("/nonexistent")).expect("default secrets infallible")
        }
    };

    // 4. Ensure state directory exists.
    if let Err(e) = tokio::fs::create_dir_all(&state_dir).await {
        tracing::error!(error = %e, "failed to create state directory");
        return;
    }

    // 5. Create PID file.
    let pid_file = match pid::PidFile::create(&state_dir).await {
        Ok(pf) => {
            tracing::info!(pid = std::process::id(), "PID file created");
            Some(pf)
        }
        Err(e) => {
            tracing::warn!(error = %e, "failed to create PID file, continuing without it");
            None
        }
    };

    // 6. Connect to PostgreSQL and run migrations (warn on failure, don't exit).
    let db_pool = match autonomic_db::create_pool(&secrets.database.url).await {
        Ok(pool) => {
            if let Err(e) = autonomic_db::run_migrations(&pool).await {
                tracing::warn!(error = %e, "database migrations failed, some features may be unavailable");
            }
            Some(pool)
        }
        Err(e) => {
            tracing::warn!(error = %e, "database connection failed, operating without persistence");
            None
        }
    };

    // 7. Find the claude binary (configurable via config.toml, falls back to $PATH).
    let runtime = match config.session.claude_binary {
        Some(ref path) => HostRuntime::from_path(path.clone()),
        None => HostRuntime::new(),
    };
    let claude_binary = runtime.claude_binary().clone();
    if !claude_binary.exists() {
        tracing::warn!(
            path = %claude_binary.display(),
            "claude binary not found, sessions will fail until it is installed"
        );
    } else {
        tracing::info!(path = %claude_binary.display(), "claude binary found");
    }

    // 8. Initialize cost tracker and session manager.
    let budget_window = Duration::from_secs(5 * 3600); // 5-hour sliding window
    let cost_tracker = Arc::new(CostTracker::new(
        config.budget.global_budget_usd,
        budget_window,
    ));
    let session_manager = Arc::new(SessionManager::new(
        config.session.max_concurrent,
        Arc::clone(&cost_tracker),
        claude_binary,
    ));

    tracing::info!(
        max_concurrent = config.session.max_concurrent,
        global_budget_usd = config.budget.global_budget_usd,
        "session manager initialized"
    );

    // 9. Build HTTP API.
    let app_state = Arc::new(AppState {
        session_manager,
        cost_tracker,
        start_time,
        db_pool,
    });
    let router = api::build_router(app_state);

    // 10. Bind and serve with graceful shutdown.
    let addr = SocketAddr::new(
        config
            .api
            .bind
            .parse()
            .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        config.api.port,
    );

    let listener = match TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(error = %e, %addr, "failed to bind HTTP listener");
            if let Some(pf) = pid_file {
                pf.remove().await;
            }
            return;
        }
    };

    tracing::info!(%addr, "HTTP API listening");

    let server = axum::serve(listener, router).with_graceful_shutdown(shutdown::shutdown_signal());

    if let Err(e) = server.await {
        tracing::error!(error = %e, "HTTP server error");
    }

    // 11. Cleanup on exit.
    tracing::info!("autonomicd shutting down");
    if let Some(pf) = pid_file {
        pf.remove().await;
    }
}
