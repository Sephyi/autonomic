//! Tracing initialization for the daemon.
//!
//! Sets up dual-layer logging: compact format to stderr for human consumption,
//! JSON format to a rotating log file for machine parsing.

use std::path::Path;

use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

/// Initialize the tracing subscriber with dual output layers.
///
/// - **stderr**: compact format, colored when a terminal is attached.
/// - **file**: JSON format written to `<log_dir>/autonomicd.log`.
///
/// The default filter is `info,autonomic=debug`, overridable via the
/// `RUST_LOG` environment variable.
///
/// # Errors
///
/// Returns an error if the log directory cannot be created or the log file
/// cannot be opened.
#[allow(clippy::disallowed_methods)]
pub fn init_tracing(log_dir: &Path) -> Result<(), std::io::Error> {
    // Startup-only blocking I/O — acceptable before the async runtime is active.
    std::fs::create_dir_all(log_dir)?;

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_dir.join("autonomicd.log"))?;

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,autonomic=debug"));

    let stderr_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_target(true)
        .with_span_events(FmtSpan::CLOSE)
        .with_filter(env_filter);

    let file_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,autonomic=debug"));

    let file_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_writer(std::sync::Mutex::new(log_file))
        .with_target(true)
        .with_span_events(FmtSpan::CLOSE)
        .with_filter(file_filter);

    tracing_subscriber::registry()
        .with(stderr_layer)
        .with(file_layer)
        .init();

    Ok(())
}
