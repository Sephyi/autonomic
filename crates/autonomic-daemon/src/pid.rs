//! PID file management for the daemon.
//!
//! Creates a PID file on startup and removes it on shutdown to prevent
//! multiple daemon instances from running simultaneously.

use std::path::{Path, PathBuf};

/// Manages the daemon PID file lifecycle.
#[derive(Debug)]
pub struct PidFile {
    path: PathBuf,
}

impl PidFile {
    /// Create a PID file in `state_dir/autonomicd.pid` containing the current
    /// process ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the PID file cannot be written.
    pub async fn create(state_dir: &Path) -> Result<Self, std::io::Error> {
        let path = state_dir.join("autonomicd.pid");
        let pid = std::process::id().to_string();
        tokio::fs::write(&path, pid.as_bytes()).await?;
        Ok(Self { path })
    }

    /// Read the PID from an existing PID file in the given state directory.
    ///
    /// Returns `None` if the file does not exist or cannot be parsed.
    /// Used by the watchdog to verify the daemon is running.
    #[allow(dead_code)]
    pub async fn read(state_dir: &Path) -> Option<u32> {
        let path = state_dir.join("autonomicd.pid");
        let contents = tokio::fs::read_to_string(&path).await.ok()?;
        contents.trim().parse().ok()
    }

    /// Remove the PID file. Called during graceful shutdown.
    pub async fn remove(self) {
        let _ = tokio::fs::remove_file(&self.path).await;
    }
}
