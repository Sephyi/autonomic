//! Command builder for Claude Code subprocess invocations.
//!
//! Translates a `SessionConfig` into a `ContainerConfig` suitable for
//! execution by a `ContainerRuntime`. This is the sole place where
//! Claude Code CLI arguments are assembled (XD-001).

use std::path::Path;

use autonomic_container::ContainerConfig;

use crate::config::SessionConfig;

/// Build a `ContainerConfig` from a `SessionConfig` and the path to the
/// Claude binary.
///
/// This is a pure function: it does no I/O, only assembles the configuration.
pub fn build_container_config(config: &SessionConfig, claude_binary: &Path) -> ContainerConfig {
    let mut args = vec![
        "--print".to_string(),
        "--output-format".to_string(),
        "stream-json".to_string(),
        "--permission-mode".to_string(),
        "acceptEdits".to_string(),
        "--model".to_string(),
        config.model.clone(),
    ];

    // Optional system prompt.
    if let Some(ref system_prompt) = config.system_prompt {
        args.push("--system-prompt".to_string());
        args.push(system_prompt.clone());
    }

    // Optional allowed tools.
    if !config.allowed_tools.is_empty() {
        args.push("--allowedTools".to_string());
        args.push(config.allowed_tools.join(","));
    }

    // Resume / continue (mutually exclusive, validated by SessionConfig).
    if let Some(ref resume_id) = config.resume_session_id {
        args.push("--resume".to_string());
        args.push(resume_id.clone());
    } else if config.continue_recent {
        args.push("--continue".to_string());
    }

    // Prompt as positional argument after `--`.
    args.push("--".to_string());
    args.push(config.prompt.clone());

    // Build environment variables.
    let mut env = config.extra_env.clone();

    // Agent teams experimental flag.
    if config.agent_teams {
        env.insert(
            "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS".to_string(),
            "1".to_string(),
        );
    }

    // Always remove CLAUDECODE from the subprocess environment.
    let env_remove = vec!["CLAUDECODE".to_string()];

    ContainerConfig {
        executable: claude_binary.to_path_buf(),
        args,
        working_dir: config.working_dir.clone(),
        env,
        env_remove,
        timeout: config.timeout,
        ..ContainerConfig::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SessionConfig;
    use autonomic_core::SessionId;
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::time::Duration;

    fn test_config() -> SessionConfig {
        SessionConfig {
            session_id: SessionId::new(),
            working_dir: PathBuf::from("/tmp/test-project"),
            prompt: "Implement the feature".to_string(),
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

    fn claude_bin() -> PathBuf {
        PathBuf::from("/usr/local/bin/claude")
    }

    #[test]
    fn build_command_includes_core_flags() {
        let cfg = test_config();
        let cc = build_container_config(&cfg, &claude_bin());
        assert!(cc.args.contains(&"--print".to_string()));
        assert!(cc.args.contains(&"stream-json".to_string()));
        assert!(cc.args.contains(&"acceptEdits".to_string()));
    }

    #[test]
    fn build_command_includes_model() {
        let cfg = test_config();
        let cc = build_container_config(&cfg, &claude_bin());
        let model_idx = cc
            .args
            .iter()
            .position(|a| a == "--model")
            .expect("--model flag");
        assert_eq!(cc.args[model_idx + 1], "sonnet");
    }

    #[test]
    fn build_command_includes_prompt_as_positional() {
        let cfg = test_config();
        let cc = build_container_config(&cfg, &claude_bin());
        let sep_idx = cc
            .args
            .iter()
            .position(|a| a == "--")
            .expect("-- separator");
        assert_eq!(cc.args[sep_idx + 1], "Implement the feature");
    }

    #[test]
    fn build_command_removes_claudecode() {
        let cfg = test_config();
        let cc = build_container_config(&cfg, &claude_bin());
        assert!(cc.env_remove.contains(&"CLAUDECODE".to_string()));
    }

    #[test]
    fn build_command_with_system_prompt() {
        let mut cfg = test_config();
        cfg.system_prompt = Some("You are a coding assistant.".to_string());
        let cc = build_container_config(&cfg, &claude_bin());
        let idx = cc
            .args
            .iter()
            .position(|a| a == "--system-prompt")
            .expect("--system-prompt flag");
        assert_eq!(cc.args[idx + 1], "You are a coding assistant.");
    }

    #[test]
    fn build_command_with_allowed_tools() {
        let mut cfg = test_config();
        cfg.allowed_tools = vec!["bash".to_string(), "read".to_string()];
        let cc = build_container_config(&cfg, &claude_bin());
        let idx = cc
            .args
            .iter()
            .position(|a| a == "--allowedTools")
            .expect("--allowedTools flag");
        assert_eq!(cc.args[idx + 1], "bash,read");
    }

    #[test]
    fn build_command_with_resume() {
        let mut cfg = test_config();
        cfg.resume_session_id = Some("sess-xyz".to_string());
        let cc = build_container_config(&cfg, &claude_bin());
        let idx = cc
            .args
            .iter()
            .position(|a| a == "--resume")
            .expect("--resume flag");
        assert_eq!(cc.args[idx + 1], "sess-xyz");
    }

    #[test]
    fn build_command_with_continue() {
        let mut cfg = test_config();
        cfg.continue_recent = true;
        let cc = build_container_config(&cfg, &claude_bin());
        assert!(cc.args.contains(&"--continue".to_string()));
        assert!(!cc.args.contains(&"--resume".to_string()));
    }

    #[test]
    fn build_command_with_agent_teams() {
        let mut cfg = test_config();
        cfg.agent_teams = true;
        let cc = build_container_config(&cfg, &claude_bin());
        assert_eq!(
            cc.env.get("CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS"),
            Some(&"1".to_string())
        );
    }

    #[test]
    fn build_command_with_extra_env() {
        let mut cfg = test_config();
        cfg.extra_env
            .insert("MY_VAR".to_string(), "my_value".to_string());
        let cc = build_container_config(&cfg, &claude_bin());
        assert_eq!(cc.env.get("MY_VAR"), Some(&"my_value".to_string()));
    }
}
