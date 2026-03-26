//! Container runtime error types.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during container runtime operations.
#[derive(Debug, Error)]
pub enum ContainerError {
    /// The runtime failed to spawn the child process.
    #[error("Failed to spawn process: {0}")]
    SpawnFailed(#[source] std::io::Error),

    /// The child process stdout pipe was not captured.
    #[error("Process stdout was not captured")]
    NoStdout,

    /// The child process stderr pipe was not captured.
    #[error("Process stderr was not captured")]
    NoStderr,

    /// The child process exited with a non-zero code or was killed by a signal.
    #[error("Process exited with code {code:?}, signal {signal:?}")]
    ProcessFailed {
        code: Option<i32>,
        signal: Option<i32>,
        stderr: String,
    },

    /// The configured working directory does not exist.
    #[error("Working directory not found: {0}")]
    WorkingDirNotFound(PathBuf),

    /// The Claude binary was not found at the expected path.
    #[error("Claude binary not found at: {0}")]
    ClaudeBinaryNotFound(PathBuf),

    /// A generic I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// The container runtime binary is not installed or not reachable.
    #[error("Container runtime not available: {0}")]
    RuntimeUnavailable(String),
}
