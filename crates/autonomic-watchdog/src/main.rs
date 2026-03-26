//! Lightweight watchdog monitor for the Autonomic daemon.
//!
//! Runs as a separate process (managed by launchd/systemd). On startup it
//! sweeps orphan processes, then enters a health-check loop against the
//! daemon's HTTP API. If the daemon is unresponsive for multiple consecutive
//! checks, the watchdog attempts a rollback to the last known-good state,
//! sweeps orphans again, and exits. The process supervisor restarts both the
//! daemon and the watchdog.

mod health;
mod orphan;
mod rollback;

use std::path::{Path, PathBuf};
use std::time::Duration;

use tracing_subscriber::EnvFilter;

/// Default daemon port if config cannot be loaded.
const DEFAULT_PORT: u16 = 7700;

/// Interval between health checks.
const CHECK_INTERVAL: Duration = Duration::from_secs(30);

/// Number of consecutive failures before declaring the daemon crashed.
const MAX_CONSECUTIVE_FAILURES: u32 = 3;

#[tokio::main]
async fn main() {
    // Minimal structured logging. RUST_LOG controls verbosity.
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let state_dir = resolve_state_dir();

    tracing::info!(state_dir = %state_dir.display(), "watchdog starting");

    // Phase 1: Sweep any orphan processes left from a previous crash.
    orphan::sweep_orphans(&state_dir).await;

    // Phase 2: Determine daemon port from config (best-effort).
    let port = load_daemon_port(&state_dir);
    tracing::info!(port, "monitoring daemon health");

    // Phase 3: Health check loop. Blocks until the daemon is considered crashed.
    let failure_reason =
        health::health_check_loop(port, CHECK_INTERVAL, MAX_CONSECUTIVE_FAILURES).await;

    tracing::error!(
        reason = failure_reason.as_str(),
        "daemon crash detected, initiating recovery"
    );

    // Phase 4: Attempt rollback to last known-good state.
    match rollback::rollback_to_known_good(&state_dir).await {
        Ok(()) => tracing::info!("rollback completed successfully"),
        Err(e) => tracing::error!(error = e.as_str(), "rollback failed"),
    }

    // Phase 5: Sweep orphans again after rollback.
    orphan::sweep_orphans(&state_dir).await;

    tracing::info!("watchdog exiting; process supervisor will restart");
}

/// Determine the Autonomic state directory.
///
/// Uses `dirs::home_dir()` to find `~/.autonomic`. Falls back to
/// `./.autonomic` if the home directory cannot be determined.
fn resolve_state_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".autonomic")
}

/// Best-effort attempt to read the daemon port from the config file.
///
/// Uses inline TOML parsing instead of `autonomic-core::load_config` to avoid
/// sharing library dependencies with the daemon. If the daemon crashes due to
/// a bug in a shared dependency, the watchdog must still be able to function.
#[allow(clippy::disallowed_methods)] // Startup-only blocking I/O, before health loop.
fn load_daemon_port(state_dir: &Path) -> u16 {
    let config_path = state_dir.join("config.toml");
    let contents = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return DEFAULT_PORT,
    };
    let table: toml::Table = match contents.parse() {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(error = %e, default = DEFAULT_PORT, "failed to parse config, using default port");
            return DEFAULT_PORT;
        }
    };
    table
        .get("api")
        .and_then(|v| v.get("port"))
        .and_then(|v| v.as_integer())
        .and_then(|p| u16::try_from(p).ok())
        .unwrap_or(DEFAULT_PORT)
}
