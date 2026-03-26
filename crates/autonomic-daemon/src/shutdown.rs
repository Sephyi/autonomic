//! Graceful shutdown signal handling.
//!
//! Listens for SIGINT (ctrl-c) and SIGTERM to trigger orderly daemon shutdown.

/// Wait for a shutdown signal (SIGINT or SIGTERM).
///
/// Returns once the first signal is received, logging which signal triggered
/// the shutdown.
pub async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install ctrl-c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {
            tracing::info!("received SIGINT, initiating graceful shutdown");
        }
        () = terminate => {
            tracing::info!("received SIGTERM, initiating graceful shutdown");
        }
    }
}
