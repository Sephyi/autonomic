//! Session configuration and validation.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use autonomic_core::SessionId;
use serde::{Deserialize, Serialize};

use crate::error::SessionConfigError;

/// A handle to a git worktree used for session isolation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeHandle {
    /// Filesystem path to the worktree checkout.
    pub path: PathBuf,
    /// Branch name checked out in this worktree.
    pub branch: String,
    /// Path to the main repository root.
    pub repo_root: PathBuf,
}

/// Configuration for a single Claude Code session.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Unique identifier for this session.
    pub session_id: SessionId,
    /// Absolute path to the working directory for the session.
    pub working_dir: PathBuf,
    /// The prompt to send to Claude Code.
    pub prompt: String,
    /// The model to use (e.g., "opus", "sonnet").
    pub model: String,
    /// Optional system prompt override.
    pub system_prompt: Option<String>,
    /// Tools that Claude Code is allowed to use.
    pub allowed_tools: Vec<String>,
    /// Maximum wall-clock time for the session.
    pub timeout: Duration,
    /// Maximum cost in USD for this session.
    pub cost_budget_usd: f64,
    /// Resume a specific previous session by its Claude session ID.
    pub resume_session_id: Option<String>,
    /// Continue the most recent session.
    pub continue_recent: bool,
    /// Additional environment variables to set in the subprocess.
    pub extra_env: HashMap<String, String>,
    /// Enable experimental agent teams mode.
    pub agent_teams: bool,
    /// Optional worktree handle for session isolation.
    pub worktree: Option<WorktreeHandle>,
}

impl SessionConfig {
    /// Validate this configuration, returning an error if any constraints
    /// are violated.
    pub fn validate(&self) -> Result<(), SessionConfigError> {
        // 1. working_dir must be absolute
        if !self.working_dir.is_absolute() {
            return Err(SessionConfigError::RelativePath(self.working_dir.clone()));
        }

        // 2. resume_session_id and continue_recent are mutually exclusive
        if self.resume_session_id.is_some() && self.continue_recent {
            return Err(SessionConfigError::ConflictingResumeOptions);
        }

        // 3. agent_teams requires model containing "opus"
        if self.agent_teams && !self.model.to_lowercase().contains("opus") {
            return Err(SessionConfigError::AgentTeamsRequiresOpus {
                model: self.model.clone(),
            });
        }

        // 4. cost_budget_usd must be > 0
        if self.cost_budget_usd <= 0.0 {
            return Err(SessionConfigError::InvalidBudget(self.cost_budget_usd));
        }

        // 5. extra_env must not contain "CLAUDECODE"
        if self.extra_env.contains_key("CLAUDECODE") {
            return Err(SessionConfigError::ForbiddenEnvVar(
                "CLAUDECODE".to_string(),
            ));
        }

        // 6. Worktree validation
        if let Some(ref wt) = self.worktree {
            if !wt.path.is_dir() {
                return Err(SessionConfigError::WorktreeNotFound(wt.path.clone()));
            }
            if !wt.path.join(".git").is_file() {
                return Err(SessionConfigError::InvalidWorktree(wt.path.clone()));
            }
            if self.working_dir != wt.path {
                return Err(SessionConfigError::WorkingDirWorktreeMismatch {
                    working_dir: self.working_dir.clone(),
                    worktree_path: wt.path.clone(),
                });
            }
        }

        Ok(())
    }
}

/// Create a minimal valid `SessionConfig` for testing.
#[cfg(test)]
fn test_config() -> SessionConfig {
    SessionConfig {
        session_id: SessionId::new(),
        working_dir: PathBuf::from("/tmp/test-project"),
        prompt: "Hello".to_string(),
        model: "sonnet".to_string(),
        system_prompt: None,
        allowed_tools: Vec::new(),
        timeout: Duration::from_secs(300),
        cost_budget_usd: 5.0,
        resume_session_id: None,
        continue_recent: false,
        extra_env: HashMap::new(),
        agent_teams: false,
        worktree: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_minimal_config_passes() {
        let config = test_config();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn relative_path_rejected() {
        let mut config = test_config();
        config.working_dir = PathBuf::from("relative/path");
        let err = config.validate().unwrap_err();
        assert!(matches!(err, SessionConfigError::RelativePath(_)));
    }

    #[test]
    fn conflicting_resume_options_rejected() {
        let mut config = test_config();
        config.resume_session_id = Some("sess-123".to_string());
        config.continue_recent = true;
        let err = config.validate().unwrap_err();
        assert!(matches!(err, SessionConfigError::ConflictingResumeOptions));
    }

    #[test]
    fn agent_teams_requires_opus() {
        let mut config = test_config();
        config.agent_teams = true;
        config.model = "sonnet".to_string();
        let err = config.validate().unwrap_err();
        assert!(matches!(
            err,
            SessionConfigError::AgentTeamsRequiresOpus { .. }
        ));
    }

    #[test]
    fn agent_teams_with_opus_passes() {
        let mut config = test_config();
        config.agent_teams = true;
        config.model = "claude-opus-4".to_string();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn zero_budget_rejected() {
        let mut config = test_config();
        config.cost_budget_usd = 0.0;
        let err = config.validate().unwrap_err();
        assert!(matches!(err, SessionConfigError::InvalidBudget(_)));
    }

    #[test]
    fn negative_budget_rejected() {
        let mut config = test_config();
        config.cost_budget_usd = -1.0;
        let err = config.validate().unwrap_err();
        assert!(matches!(err, SessionConfigError::InvalidBudget(_)));
    }

    #[test]
    fn claudecode_env_forbidden() {
        let mut config = test_config();
        config
            .extra_env
            .insert("CLAUDECODE".to_string(), "1".to_string());
        let err = config.validate().unwrap_err();
        assert!(matches!(err, SessionConfigError::ForbiddenEnvVar(_)));
    }

    #[test]
    fn resume_session_without_continue_passes() {
        let mut config = test_config();
        config.resume_session_id = Some("sess-abc".to_string());
        config.continue_recent = false;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn continue_without_resume_passes() {
        let mut config = test_config();
        config.resume_session_id = None;
        config.continue_recent = true;
        assert!(config.validate().is_ok());
    }
}
