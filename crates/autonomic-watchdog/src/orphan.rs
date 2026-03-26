//! Orphan process detection and cleanup.
//!
//! Scans PID files left behind by crashed sessions and terminates any
//! still-running processes before cleaning up stale PID files.

use std::path::Path;

use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;

/// Sweep orphan session processes by scanning PID files in `state_dir/state/sessions/`.
///
/// For each `.pid` file found:
/// 1. Parse the PID from the file contents.
/// 2. Check whether the process is still running (signal 0).
/// 3. If running, send `SIGTERM`.
/// 4. Remove the PID file regardless of outcome.
pub async fn sweep_orphans(state_dir: &Path) {
    let sessions_dir = state_dir.join("state").join("sessions");

    let mut entries = match tokio::fs::read_dir(&sessions_dir).await {
        Ok(entries) => entries,
        Err(e) => {
            tracing::debug!(
                path = %sessions_dir.display(),
                error = %e,
                "sessions directory not readable, skipping orphan sweep"
            );
            return;
        }
    };

    let mut swept = 0u32;
    let mut errors = 0u32;

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();

        let is_pid_file = path.extension().is_some_and(|ext| ext == "pid");
        if !is_pid_file {
            continue;
        }

        match process_pid_file(&path).await {
            Ok(terminated) => {
                if terminated {
                    swept += 1;
                }
            }
            Err(reason) => {
                tracing::warn!(
                    file = %path.display(),
                    reason,
                    "failed to process PID file"
                );
                errors += 1;
            }
        }

        // Clean up the PID file regardless of whether the process was running.
        if let Err(e) = tokio::fs::remove_file(&path).await {
            tracing::warn!(
                file = %path.display(),
                error = %e,
                "failed to remove PID file"
            );
        }
    }

    if swept > 0 || errors > 0 {
        tracing::info!(terminated = swept, errors, "orphan sweep complete");
    } else {
        tracing::debug!("orphan sweep: no orphan processes found");
    }
}

/// Read a PID file, check if the process is alive, and send SIGTERM if so.
///
/// Returns `Ok(true)` if a process was terminated, `Ok(false)` if the process
/// was already dead, or `Err` if the PID file could not be parsed.
async fn process_pid_file(path: &Path) -> Result<bool, String> {
    let contents = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| format!("read error: {e}"))?;

    let raw_pid: i32 = contents
        .trim()
        .parse()
        .map_err(|e| format!("invalid PID in file: {e}"))?;

    let pid = Pid::from_raw(raw_pid);

    // Signal 0: check if process exists without actually sending a signal.
    let is_running = signal::kill(pid, None).is_ok();

    if is_running {
        tracing::info!(pid = raw_pid, "sending SIGTERM to orphan process");
        if let Err(e) = signal::kill(pid, Signal::SIGTERM) {
            tracing::warn!(
                pid = raw_pid,
                error = %e,
                "failed to send SIGTERM"
            );
        }
        Ok(true)
    } else {
        tracing::debug!(pid = raw_pid, "orphan process already dead");
        Ok(false)
    }
}
