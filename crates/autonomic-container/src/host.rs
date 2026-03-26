//! Host-native runtime that spawns processes directly via `tokio::process::Command`.
//!
//! This is the Phase 1 backend. It runs Claude Code as a direct child process
//! on the host, with environment filtering and resource-limit awareness.

use std::path::PathBuf;

use tokio::io::AsyncReadExt;
use tracing::debug;

use crate::config::ContainerConfig;
use crate::error::ContainerError;
use crate::runtime::{ContainerRuntime, ProcessOutput};

/// Runs processes directly on the host using `tokio::process::Command`.
#[derive(Debug, Clone)]
pub struct HostRuntime {
    /// Absolute path to the Claude Code binary.
    claude_binary: PathBuf,
}

impl HostRuntime {
    /// Create a new `HostRuntime` by searching `$PATH` for `claude`.
    ///
    /// Returns `Ok` even if the binary is not found; use [`ContainerRuntime::is_available`]
    /// to check availability.
    pub fn new() -> Self {
        let claude_binary = which_claude().unwrap_or_else(|| PathBuf::from("claude"));
        Self { claude_binary }
    }

    /// Create a `HostRuntime` with an explicit path to the Claude binary.
    pub fn from_path(path: PathBuf) -> Self {
        Self {
            claude_binary: path,
        }
    }

    /// Return the configured path to the Claude binary.
    pub fn claude_binary(&self) -> &PathBuf {
        &self.claude_binary
    }
}

impl Default for HostRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl ContainerRuntime for HostRuntime {
    async fn spawn(&self, config: &ContainerConfig) -> Result<ProcessOutput, ContainerError> {
        // Validate working directory exists.
        if !config.working_dir.exists() {
            return Err(ContainerError::WorkingDirNotFound(
                config.working_dir.clone(),
            ));
        }

        let mut cmd = tokio::process::Command::new(&config.executable);
        cmd.args(&config.args)
            .current_dir(&config.working_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Environment filtering: clear inherited env, then apply config env.
        cmd.env_clear();
        for (key, value) in &config.env {
            cmd.env(key, value);
        }
        // Remove any explicitly excluded variables (applied after env set).
        for key in &config.env_remove {
            cmd.env_remove(key);
        }

        debug!(
            executable = %config.executable.display(),
            working_dir = %config.working_dir.display(),
            args = ?config.args,
            "spawning host process"
        );

        let mut child = cmd.spawn().map_err(ContainerError::SpawnFailed)?;

        // Write stdin if provided, then drop to close the pipe.
        if let Some(ref input) = config.stdin {
            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                // Best-effort write; if it fails the process may still produce output.
                let _ = stdin.write_all(input.as_bytes()).await;
                let _ = stdin.flush().await;
            }
        } else {
            // Drop stdin immediately so the child doesn't wait for input.
            drop(child.stdin.take());
        }

        // Consume stdout and stderr concurrently to avoid pipe deadlock (XD-005).
        let mut stdout_handle = child.stdout.take().ok_or(ContainerError::NoStdout)?;
        let mut stderr_handle = child.stderr.take().ok_or(ContainerError::NoStderr)?;

        let mut stdout_buf = String::new();
        let mut stderr_buf = String::new();

        let (stdout_result, stderr_result, status) = tokio::join!(
            stdout_handle.read_to_string(&mut stdout_buf),
            stderr_handle.read_to_string(&mut stderr_buf),
            child.wait(),
        );

        stdout_result?;
        stderr_result?;
        let status = status?;

        let exit_code = status.code();

        Ok(ProcessOutput {
            exit_code,
            stdout: stdout_buf,
            stderr: stderr_buf,
        })
    }

    async fn is_available(&self) -> bool {
        self.claude_binary.exists()
            || tokio::process::Command::new(&self.claude_binary)
                .arg("--version")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .await
                .is_ok_and(|s| s.success())
    }

    fn name(&self) -> &str {
        "host"
    }
}

/// Search `$PATH` for the `claude` binary using a simple lookup.
fn which_claude() -> Option<PathBuf> {
    let path_var = std::env::var("PATH").ok()?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join("claude");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ContainerConfig;
    use std::collections::HashMap;

    #[tokio::test]
    async fn spawns_process() {
        let runtime = HostRuntime::from_path(PathBuf::from("/bin/echo"));

        let config = ContainerConfig {
            executable: PathBuf::from("/bin/echo"),
            args: vec!["hello".to_string(), "world".to_string()],
            working_dir: std::env::temp_dir(),
            env: HashMap::new(),
            env_remove: Vec::new(),
            timeout: std::time::Duration::from_secs(10),
            resources: Default::default(),
            stdin: None,
        };

        let output = runtime.spawn(&config).await.expect("spawn should succeed");
        assert_eq!(output.exit_code, Some(0));
        assert_eq!(output.stdout.trim(), "hello world");
    }

    #[tokio::test]
    async fn rejects_missing_working_dir() {
        let runtime = HostRuntime::from_path(PathBuf::from("/bin/echo"));

        let config = ContainerConfig {
            executable: PathBuf::from("/bin/echo"),
            args: Vec::new(),
            working_dir: PathBuf::from("/nonexistent/directory/that/should/not/exist"),
            env: HashMap::new(),
            env_remove: Vec::new(),
            timeout: std::time::Duration::from_secs(10),
            resources: Default::default(),
            stdin: None,
        };

        let err = runtime.spawn(&config).await.unwrap_err();
        assert!(
            matches!(err, ContainerError::WorkingDirNotFound(_)),
            "expected WorkingDirNotFound, got: {err:?}"
        );
    }

    #[tokio::test]
    async fn is_available_with_valid_binary() {
        // /bin/echo always exists on macOS/Linux.
        let runtime = HostRuntime::from_path(PathBuf::from("/bin/echo"));
        assert!(runtime.is_available().await);
    }

    #[tokio::test]
    async fn unavailable_with_missing_binary() {
        let runtime =
            HostRuntime::from_path(PathBuf::from("/nonexistent/binary/that/should/not/exist"));
        assert!(!runtime.is_available().await);
    }
}
