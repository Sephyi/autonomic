//! Daemon health checking via raw TCP.
//!
//! Uses a plain TCP connection with a hand-crafted HTTP request to avoid
//! pulling in an HTTP client dependency. The watchdog must stay lightweight
//! and independent of the daemon's dependency tree.

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Default TCP connect and read timeout for health checks.
const HEALTH_TIMEOUT: Duration = Duration::from_secs(5);

/// Result of a single health check against the daemon.
#[derive(Debug, Clone)]
pub enum HealthStatus {
    /// Daemon responded with HTTP 200 and a parseable uptime.
    Healthy {
        /// How long the daemon reports it has been running.
        uptime_seconds: u64,
    },
    /// Daemon is unreachable or returned an unexpected response.
    Unhealthy {
        /// Human-readable reason for the failure.
        reason: String,
    },
}

/// Perform a single health check against the daemon's HTTP `/health` endpoint.
///
/// Opens a raw TCP connection to `127.0.0.1:{port}`, sends an HTTP/1.1 GET,
/// and parses the JSON response body for an `uptime_seconds` field.
pub async fn check_daemon_health(port: u16) -> HealthStatus {
    let addr = format!("127.0.0.1:{port}");

    let mut stream = match timeout(HEALTH_TIMEOUT, TcpStream::connect(&addr)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            return HealthStatus::Unhealthy {
                reason: format!("TCP connect failed: {e}"),
            };
        }
        Err(_) => {
            return HealthStatus::Unhealthy {
                reason: "TCP connect timed out after 5s".into(),
            };
        }
    };

    let request =
        format!("GET /health HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");

    if let Err(e) = stream.write_all(request.as_bytes()).await {
        return HealthStatus::Unhealthy {
            reason: format!("failed to send request: {e}"),
        };
    }

    let mut buf = vec![0u8; 4096];
    let n = match timeout(HEALTH_TIMEOUT, stream.read(&mut buf)).await {
        Ok(Ok(n)) => n,
        Ok(Err(e)) => {
            return HealthStatus::Unhealthy {
                reason: format!("read failed: {e}"),
            };
        }
        Err(_) => {
            return HealthStatus::Unhealthy {
                reason: "read timed out after 5s".into(),
            };
        }
    };

    let response = String::from_utf8_lossy(&buf[..n]);

    // Check for HTTP 200 status line.
    let Some(status_end) = response.find("\r\n") else {
        return HealthStatus::Unhealthy {
            reason: "malformed HTTP response: no status line".into(),
        };
    };

    let status_line = &response[..status_end];
    if !status_line.contains("200") {
        return HealthStatus::Unhealthy {
            reason: format!("non-200 status: {status_line}"),
        };
    }

    // Parse JSON body after the blank line separator.
    let Some(body_start) = response.find("\r\n\r\n") else {
        return HealthStatus::Unhealthy {
            reason: "malformed HTTP response: no body separator".into(),
        };
    };

    let body = &response[body_start + 4..];

    match serde_json::from_str::<serde_json::Value>(body) {
        Ok(v) => {
            if let Some(uptime) = v.get("uptime_seconds").and_then(|u| u.as_u64()) {
                HealthStatus::Healthy {
                    uptime_seconds: uptime,
                }
            } else {
                // Daemon is responding with 200 but no uptime field; still healthy.
                HealthStatus::Healthy { uptime_seconds: 0 }
            }
        }
        Err(_) => {
            // 200 response but unparseable body -- treat as healthy (daemon is alive).
            HealthStatus::Healthy { uptime_seconds: 0 }
        }
    }
}

/// Run the health check loop until the daemon is considered crashed.
///
/// Returns the failure reason string once `max_consecutive_failures`
/// consecutive unhealthy checks have been observed. Each check is
/// separated by `interval`.
pub async fn health_check_loop(
    port: u16,
    interval: Duration,
    max_consecutive_failures: u32,
) -> String {
    let mut consecutive_failures: u32 = 0;

    loop {
        let status = check_daemon_health(port).await;

        match status {
            HealthStatus::Healthy { uptime_seconds } => {
                if consecutive_failures > 0 {
                    tracing::info!(
                        uptime_seconds,
                        previous_failures = consecutive_failures,
                        "daemon recovered"
                    );
                }
                consecutive_failures = 0;
            }
            HealthStatus::Unhealthy { ref reason } => {
                consecutive_failures += 1;
                tracing::warn!(
                    consecutive_failures,
                    max = max_consecutive_failures,
                    reason = reason.as_str(),
                    "health check failed"
                );

                if consecutive_failures >= max_consecutive_failures {
                    return reason.clone();
                }
            }
        }

        tokio::time::sleep(interval).await;
    }
}
