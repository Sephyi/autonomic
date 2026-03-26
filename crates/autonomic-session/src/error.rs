//! Error types for session management.

use std::path::PathBuf;
use std::time::Duration;

use autonomic_container::ContainerError;
use thiserror::Error;

/// Errors that occur during session lifecycle management.
#[derive(Debug, Error)]
pub enum SessionError {
    /// The container runtime failed to spawn the subprocess.
    #[error("Failed to spawn session process: {0}")]
    SpawnFailed(#[source] ContainerError),

    /// The subprocess stdout pipe was not captured.
    #[error("Process stdout was not captured")]
    NoStdout,

    /// The subprocess stderr pipe was not captured.
    #[error("Process stderr was not captured")]
    NoStderr,

    /// The session exceeded its wall-clock timeout.
    #[error("Session timed out after {elapsed:?} (limit: {limit:?})")]
    Timeout {
        /// How long the session actually ran.
        elapsed: Duration,
        /// The configured timeout limit.
        limit: Duration,
    },

    /// The session exceeded its per-session cost budget.
    #[error("Session cost ${accumulated:.4} exceeds budget ${budget:.4}")]
    BudgetExceeded {
        /// The accumulated cost so far.
        accumulated: f64,
        /// The configured budget limit.
        budget: f64,
    },

    /// The Claude Code process crashed or was killed.
    #[error("Process crashed (exit_code: {exit_code:?}, signal: {signal:?})")]
    ProcessCrashed {
        /// The exit code, if the process exited normally.
        exit_code: Option<i32>,
        /// The signal number, if the process was killed by a signal.
        signal: Option<i32>,
        /// Captured stderr output.
        stderr: String,
    },

    /// The API rate limit was hit and the session cannot proceed.
    #[error("API rate limit reached")]
    RateLimited,

    /// Failed to parse a stream-json line from Claude Code output.
    #[error("Parse error: {message} (raw: {raw_line:?})")]
    ParseError {
        /// Description of the parse failure.
        message: String,
        /// The raw line that failed to parse.
        raw_line: String,
    },

    /// Invalid session configuration.
    #[error(transparent)]
    ConfigError(#[from] SessionConfigError),

    /// The global cost budget across all sessions is exhausted.
    #[error("Global cost budget exhausted")]
    GlobalBudgetExhausted,

    /// The maximum number of concurrent sessions has been reached.
    #[error("Concurrency limit reached (max: {max})")]
    ConcurrencyLimitReached {
        /// The configured maximum concurrent sessions.
        max: usize,
    },

    /// The referenced session was not found.
    #[error("Session not found: {0}")]
    NotFound(String),

    /// A generic I/O error during session operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Errors that occur during session configuration validation.
#[derive(Debug, Error)]
pub enum SessionConfigError {
    /// The working directory path is not absolute.
    #[error("Working directory must be an absolute path, got: {0}")]
    RelativePath(PathBuf),

    /// The working directory does not exist on disk.
    #[error("Working directory not found: {0}")]
    WorkingDirNotFound(PathBuf),

    /// Both `resume_session_id` and `continue_recent` were specified.
    #[error("Cannot specify both resume_session_id and continue_recent")]
    ConflictingResumeOptions,

    /// Agent teams require an Opus-class model.
    #[error("Agent teams require an Opus model, got: {model}")]
    AgentTeamsRequiresOpus {
        /// The model string that was provided.
        model: String,
    },

    /// The cost budget must be a positive value.
    #[error("Invalid cost budget: ${0:.4} (must be > 0)")]
    InvalidBudget(f64),

    /// An environment variable name is forbidden because it conflicts with
    /// internal session management.
    #[error("Forbidden environment variable: {0}")]
    ForbiddenEnvVar(String),

    /// The specified worktree path does not exist.
    #[error("Worktree not found: {0}")]
    WorktreeNotFound(PathBuf),

    /// The specified worktree path is not a valid git worktree.
    #[error("Invalid worktree (missing .git file): {0}")]
    InvalidWorktree(PathBuf),

    /// The working directory does not match the worktree path.
    #[error("Working directory {working_dir} does not match worktree path {worktree_path}")]
    WorkingDirWorktreeMismatch {
        /// The configured working directory.
        working_dir: PathBuf,
        /// The worktree path.
        worktree_path: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_error_display_timeout() {
        let err = SessionError::Timeout {
            elapsed: Duration::from_secs(120),
            limit: Duration::from_secs(60),
        };
        let msg = err.to_string();
        assert!(msg.contains("timed out"));
        assert!(msg.contains("120"));
    }

    #[test]
    fn session_error_display_budget_exceeded() {
        let err = SessionError::BudgetExceeded {
            accumulated: 5.1234,
            budget: 5.0,
        };
        let msg = err.to_string();
        assert!(msg.contains("5.1234"));
        assert!(msg.contains("5.0000"));
    }

    #[test]
    fn config_error_display_relative_path() {
        let err = SessionConfigError::RelativePath(PathBuf::from("relative/path"));
        let msg = err.to_string();
        assert!(msg.contains("relative/path"));
        assert!(msg.contains("absolute"));
    }

    #[test]
    fn config_error_display_agent_teams() {
        let err = SessionConfigError::AgentTeamsRequiresOpus {
            model: "sonnet".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Opus"));
        assert!(msg.contains("sonnet"));
    }

    #[test]
    fn session_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let session_err = SessionError::from(io_err);
        assert!(matches!(session_err, SessionError::Io(_)));
    }

    #[test]
    fn session_error_from_config_error() {
        let config_err = SessionConfigError::InvalidBudget(-1.0);
        let session_err = SessionError::from(config_err);
        assert!(matches!(session_err, SessionError::ConfigError(_)));
    }
}
