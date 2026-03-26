# Phase 2B: Hook System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `autonomic-hooks` crate (standalone-capable) and the managed hook scripts: compaction recovery, session-start-context, session-metrics, command guard, and format-on-edit. Wire hooks into the daemon's session lifecycle.

**Architecture:** `autonomic-hooks` is deliberately standalone — usable without the full orchestrator. It provides: hook configuration parsing, hook execution (subprocess with stdin/stdout/stderr), hook registration management, and managed hook templates. The daemon calls hook functions at session lifecycle points. Hook scripts live in `~/.autonomic/hooks/` and are registered in Claude Code's `settings.json`.

**Tech Stack:** Rust 2024 (1.94), tokio, serde/serde_json, tracing

**Spec:** `docs/architecture/hook-system.md` (46KB, 1,404 lines)

**Prereqs from Phase 1 + 2A:** `SessionManager`, `MemoryStore`, `assemble_context`, PgPool, session lifecycle events

**Key Constraints:**
- Hooks run as subprocesses — stdin receives JSON, stdout injects into agent context
- Exit code 2 blocks the tool call (PreToolUse only)
- autonomic-hooks crate must NOT depend on daemon or session crates (standalone)

## File Map

```txt
MODIFY: crates/autonomic-hooks/Cargo.toml             # Add dependencies
CREATE: crates/autonomic-hooks/src/lib.rs              # Re-exports
CREATE: crates/autonomic-hooks/src/config.rs           # HookConfig, HookEvent, HookEntry parsing
CREATE: crates/autonomic-hooks/src/executor.rs         # Execute hooks as subprocesses
CREATE: crates/autonomic-hooks/src/registry.rs         # Load/merge hook registrations from settings.json
CREATE: crates/autonomic-hooks/src/error.rs            # HookError enum
CREATE: crates/autonomic-hooks/src/managed/mod.rs      # Managed hook templates
CREATE: crates/autonomic-hooks/src/managed/command_guard.rs  # L1/L2 command guard (FR-010)
CREATE: crates/autonomic-hooks/src/managed/compaction.rs     # Compaction recovery (FR-009)

CREATE: infra/hooks/command-guard.py                   # Command guard script (Python for regex)
CREATE: infra/hooks/compaction-recovery.sh             # Compaction recovery script
CREATE: infra/hooks/session-start-context.sh           # Memory context injection
CREATE: infra/hooks/session-metrics.sh                 # Experience trace collection
CREATE: infra/hooks/pre-compact-marker.sh              # PreCompact marker for recovery detection
```

## Task 1: Hook Types and Error

**Files:**
- Create: `crates/autonomic-hooks/src/error.rs`
- Create: `crates/autonomic-hooks/src/config.rs`

- [ ] **Step 1: Update Cargo.toml**

```toml
[package]
name = "autonomic-hooks"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "Hook management for Claude Code lifecycle events (standalone-capable)"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }

[lints]
workspace = true
```

Note: NO dependency on autonomic-core or any sibling crate — standalone-capable.

- [ ] **Step 2: Create error.rs**

```rust
//! Hook subsystem errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum HookError {
    #[error("hook execution failed: {0}")]
    Execution(String),

    #[error("hook timed out after {timeout_secs}s: {hook_name}")]
    Timeout { hook_name: String, timeout_secs: u64 },

    #[error("hook blocked tool call: {reason}")]
    Blocked { reason: String },

    #[error("invalid hook config: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
```

- [ ] **Step 3: Create config.rs**

```rust
//! Hook configuration types matching Claude Code's settings.json format.

use serde::{Deserialize, Serialize};

/// Hook lifecycle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookEvent {
    SessionStart,
    UserPromptSubmit,
    InstructionsLoaded,
    SubagentStart,
    PreToolUse,
    PostToolUse,
    PreCompact,
    Notification,
    SubagentStop,
    Stop,
}

impl HookEvent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SessionStart => "SessionStart",
            Self::UserPromptSubmit => "UserPromptSubmit",
            Self::InstructionsLoaded => "InstructionsLoaded",
            Self::SubagentStart => "SubagentStart",
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
            Self::PreCompact => "PreCompact",
            Self::Notification => "Notification",
            Self::SubagentStop => "SubagentStop",
            Self::Stop => "Stop",
        }
    }

    /// Whether this event type can block (exit code 2).
    pub fn can_block(self) -> bool {
        matches!(self, Self::PreToolUse)
    }
}

/// A matcher + hooks pair from settings.json.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookMatcher {
    /// Regex pattern matching tool_name or trigger context.
    pub matcher: String,
    /// Hook commands to execute when matched.
    pub hooks: Vec<HookEntry>,
}

/// A single hook command entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEntry {
    /// Hook type: "command" or "prompt".
    #[serde(rename = "type")]
    pub hook_type: String,
    /// Shell command to execute (for type=command).
    pub command: String,
    /// Timeout in seconds. Defaults to 30.
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

fn default_timeout() -> u64 {
    30
}

/// The result of executing a hook.
#[derive(Debug, Clone)]
pub struct HookResult {
    /// Content from stdout (injected into agent context).
    pub stdout: String,
    /// Content from stderr (logged).
    pub stderr: String,
    /// Process exit code.
    pub exit_code: i32,
    /// Whether the hook blocked the operation (exit code 2 on PreToolUse).
    pub blocked: bool,
}
```

- [ ] **Step 4: Update lib.rs**

```rust
//! Hook management for Claude Code lifecycle events.
//!
//! This crate is standalone-capable — usable without the full Autonomic
//! orchestrator. It provides hook configuration parsing, execution, and
//! registration management.

pub mod config;
pub mod error;
pub mod executor;
pub mod managed;
pub mod registry;

pub use config::{HookEntry, HookEvent, HookMatcher, HookResult};
pub use error::HookError;
pub use executor::execute_hook;
pub use registry::HookRegistry;
```

- [ ] **Step 5: Commit**

```bash
git add crates/autonomic-hooks/
git commit -m "feat(hooks): add types, config, and error enum (standalone-capable)"
```

## Task 2: Hook Executor

**Files:**
- Create: `crates/autonomic-hooks/src/executor.rs`

- [ ] **Step 1: Create executor.rs — subprocess hook execution**

```rust
//! Execute hooks as subprocesses with stdin/stdout/stderr handling.

use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::config::{HookEntry, HookEvent, HookResult};
use crate::error::HookError;

/// Execute a single hook command.
///
/// - Passes `stdin_json` on stdin
/// - Captures stdout (for context injection) and stderr (for logging)
/// - Enforces timeout
/// - Returns structured result with exit code and blocked flag
pub async fn execute_hook(
    event: HookEvent,
    entry: &HookEntry,
    stdin_json: &str,
) -> Result<HookResult, HookError> {
    let timeout = Duration::from_secs(entry.timeout);

    let mut child = Command::new("sh")
        .args(["-c", &entry.command])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| HookError::Execution(format!("failed to spawn: {e}")))?;

    // Write stdin, then explicitly drop to signal EOF before waiting.
    // Without this drop, the child may block reading stdin and we deadlock on wait_with_output.
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(stdin_json.as_bytes()).await;
        let _ = stdin.flush().await;
        drop(stdin);
    }

    // Wait with timeout.
    let output = tokio::time::timeout(timeout, child.wait_with_output())
        .await
        .map_err(|_| HookError::Timeout {
            hook_name: entry.command.clone(),
            timeout_secs: entry.timeout,
        })?
        .map_err(|e| HookError::Execution(e.to_string()))?;

    let exit_code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let blocked = event.can_block() && exit_code == 2;

    if exit_code == 1 {
        tracing::warn!(
            command = %entry.command,
            exit_code,
            stderr = %stderr.trim(),
            "hook returned error"
        );
    }

    Ok(HookResult {
        stdout,
        stderr,
        exit_code,
        blocked,
    })
}

/// Execute all matching hooks for an event.
///
/// Returns combined stdout from all hooks and whether any hook blocked.
pub async fn execute_hooks(
    event: HookEvent,
    matchers: &[crate::config::HookMatcher],
    tool_name: Option<&str>,
    stdin_json: &str,
) -> Result<(String, bool), HookError> {
    let mut combined_stdout = String::new();
    let mut blocked = false;

    for matcher in matchers {
        // Check if this matcher applies.
        let pattern = &matcher.matcher;
        let matches = if pattern == "*" || pattern == ".*" {
            true
        } else if let Some(tool) = tool_name {
            regex_matches(pattern, tool)
        } else {
            true // Non-tool events: matcher is for context, not tool_name.
        };

        if !matches {
            continue;
        }

        for entry in &matcher.hooks {
            let result = execute_hook(event, entry, stdin_json).await?;

            if !result.stdout.is_empty() {
                combined_stdout.push_str(&result.stdout);
                combined_stdout.push('\n');
            }

            if result.blocked {
                blocked = true;
                break; // First block wins.
            }
        }

        if blocked {
            break;
        }
    }

    Ok((combined_stdout, blocked))
}

/// Pattern matching for hook matchers. Supports:
/// - `"*"` or `".*"` — match anything
/// - `"Edit|Write"` — pipe-separated exact alternatives
/// - `"Bash"` — exact match against tool name
fn regex_matches(pattern: &str, text: &str) -> bool {
    if pattern == "*" || pattern == ".*" {
        return true;
    }
    // Support pipe-separated alternatives: "Edit|Write"
    if pattern.contains('|') {
        return pattern.split('|').any(|p| p.trim() == text);
    }
    // Exact match (not substring) to avoid "Write" matching "BashWrite".
    pattern == text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regex_matches_wildcard() {
        assert!(regex_matches(".*", "Bash"));
        assert!(regex_matches("*", "anything"));
    }

    #[test]
    fn regex_matches_pipe_alternatives() {
        assert!(regex_matches("Edit|Write", "Edit"));
        assert!(regex_matches("Edit|Write", "Write"));
        assert!(!regex_matches("Edit|Write", "Bash"));
    }

    #[test]
    fn regex_matches_exact() {
        assert!(regex_matches("Bash", "Bash"));
        assert!(!regex_matches("Bash", "Read"));
        assert!(!regex_matches("Write", "BashWrite")); // No substring matching
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/autonomic-hooks/src/executor.rs
git commit -m "feat(hooks): add subprocess hook executor with timeout and blocking"
```

## Task 3: Hook Registry

**Files:**
- Create: `crates/autonomic-hooks/src/registry.rs`

- [ ] **Step 1: Create registry.rs — load and merge hook registrations**

```rust
//! Load and merge hook registrations from Claude Code settings.json files.
//!
//! Hooks are registered at three levels: user global, project, project local.
//! All levels are merged. Within the same event, hooks execute in registration order.

use std::collections::HashMap;
use std::path::Path;

use crate::config::{HookEvent, HookMatcher};
use crate::error::HookError;

/// A registry of all loaded hook configurations.
#[derive(Debug, Default)]
pub struct HookRegistry {
    hooks: HashMap<String, Vec<HookMatcher>>,
}

impl HookRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            hooks: HashMap::new(),
        }
    }

    /// Load hooks from a settings.json file and merge into this registry.
    pub async fn load_from(&mut self, path: &Path) -> Result<(), HookError> {
        if !path.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(path).await?;
        let settings: serde_json::Value = serde_json::from_str(&content)?;

        if let Some(hooks_obj) = settings.get("hooks").and_then(|h| h.as_object()) {
            for (event_name, matchers_val) in hooks_obj {
                if let Some(matchers_arr) = matchers_val.as_array() {
                    let matchers: Vec<HookMatcher> = matchers_arr
                        .iter()
                        .filter_map(|v| serde_json::from_value(v.clone()).ok())
                        .collect();

                    self.hooks
                        .entry(event_name.clone())
                        .or_default()
                        .extend(matchers);
                }
            }
        }

        Ok(())
    }

    /// Load hooks from all three levels for a project.
    pub async fn load_all(
        &mut self,
        user_settings: &Path,
        project_settings: &Path,
        project_local_settings: &Path,
    ) -> Result<(), HookError> {
        self.load_from(user_settings).await?;
        self.load_from(project_settings).await?;
        self.load_from(project_local_settings).await?;
        Ok(())
    }

    /// Get all matchers for a given event.
    pub fn get(&self, event: HookEvent) -> &[HookMatcher] {
        self.hooks
            .get(event.as_str())
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get all registered event names.
    pub fn events(&self) -> Vec<String> {
        self.hooks.keys().cloned().collect()
    }

    /// Total number of registered hook entries across all events.
    pub fn total_hooks(&self) -> usize {
        self.hooks.values().map(|v| v.iter().map(|m| m.hooks.len()).sum::<usize>()).sum()
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add crates/autonomic-hooks/src/registry.rs
git commit -m "feat(hooks): add HookRegistry with multi-level settings.json loading"
```

## Task 4: Command Guard (FR-010)

**Files:**
- Create: `crates/autonomic-hooks/src/managed/mod.rs`
- Create: `crates/autonomic-hooks/src/managed/command_guard.rs`
- Create: `infra/hooks/command-guard.py`

- [ ] **Step 1: Create managed/mod.rs**

```rust
pub mod command_guard;
pub mod compaction;
```

- [ ] **Step 2: Create command_guard.rs — Rust-side configuration**

```rust
//! Command guard configuration for the managed PreToolUse hook (FR-010).
//!
//! Defines L1 (block + ask) and L2 (self-verify prompt) patterns.
//! The actual execution is via a Python script for regex flexibility.

use serde::{Deserialize, Serialize};

/// A command guard rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardRule {
    /// Regex pattern to match against the command.
    pub pattern: String,
    /// Guard level: L1 (hard block) or L2 (self-verify prompt).
    pub level: GuardLevel,
    /// Human-readable reason for blocking.
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuardLevel {
    /// L1: Hard block — command is prevented with exit code 2.
    L1,
    /// L2: Self-verify — agent is prompted to reconsider but not blocked.
    L2,
}

/// Default L1 rules: always blocked, cannot be overridden.
pub fn default_l1_rules() -> Vec<GuardRule> {
    vec![
        GuardRule {
            pattern: r"rm\s+-rf\s+/\s*$|rm\s+-rf\s+/[^a-zA-Z]".to_string(),
            level: GuardLevel::L1,
            reason: "Destructive: rm -rf on root or near-root path".to_string(),
        },
        GuardRule {
            pattern: r"git\s+push\s+.*--force".to_string(),
            level: GuardLevel::L1,
            reason: "Destructive: force push can overwrite remote history".to_string(),
        },
        GuardRule {
            pattern: r"DROP\s+(TABLE|DATABASE|SCHEMA)".to_string(),
            level: GuardLevel::L1,
            reason: "Destructive: SQL DROP statement".to_string(),
        },
        GuardRule {
            pattern: r"prisma\s+migrate\s+reset".to_string(),
            level: GuardLevel::L1,
            reason: "Destructive: prisma migrate reset drops all data".to_string(),
        },
        GuardRule {
            pattern: r"git\s+reset\s+--hard".to_string(),
            level: GuardLevel::L1,
            reason: "Destructive: git reset --hard discards uncommitted changes".to_string(),
        },
        GuardRule {
            pattern: r"git\s+clean\s+-[a-zA-Z]*f".to_string(),
            level: GuardLevel::L1,
            reason: "Destructive: git clean -f removes untracked files".to_string(),
        },
    ]
}

/// Default L2 rules: agent gets a self-verify prompt.
pub fn default_l2_rules() -> Vec<GuardRule> {
    vec![
        GuardRule {
            pattern: r"rm\s+-r".to_string(),
            level: GuardLevel::L2,
            reason: "Potentially destructive: recursive deletion".to_string(),
        },
        GuardRule {
            pattern: r"chmod\s+-R".to_string(),
            level: GuardLevel::L2,
            reason: "Potentially destructive: recursive permission change".to_string(),
        },
        GuardRule {
            pattern: r"docker\s+(rm|rmi|system\s+prune)".to_string(),
            level: GuardLevel::L2,
            reason: "Potentially destructive: Docker resource removal".to_string(),
        },
    ]
}

/// Generate the command-guard.py script with the configured rules.
pub fn generate_guard_script(l1_rules: &[GuardRule], l2_rules: &[GuardRule]) -> String {
    let mut rules_json = Vec::new();
    for rule in l1_rules {
        rules_json.push(serde_json::json!({
            "pattern": rule.pattern,
            "level": "L1",
            "reason": rule.reason,
        }));
    }
    for rule in l2_rules {
        rules_json.push(serde_json::json!({
            "pattern": rule.pattern,
            "level": "L2",
            "reason": rule.reason,
        }));
    }

    // Build the Python script as a plain string, inserting the JSON rules literally.
    // We avoid Rust format!() here because the Python f-strings and JSON braces
    // conflict with Rust's brace escaping.
    let rules_str = serde_json::to_string_pretty(&rules_json).unwrap_or_default();
    let mut script = String::new();
    script.push_str("#!/usr/bin/env python3\n");
    script.push_str("\"\"\"Command guard hook — blocks destructive commands (FR-010).\n");
    script.push_str("Auto-generated by Autonomic. Regenerated on config change.\n");
    script.push_str("\"\"\"\n");
    script.push_str("import json, re, sys\n\n");
    script.push_str("RULES = ");
    script.push_str(&rules_str);
    script.push_str("\n\ndef main():\n");
    script.push_str("    data = json.load(sys.stdin)\n");
    script.push_str("    tool_name = data.get('tool_name', '')\n");
    script.push_str("    if tool_name != 'Bash':\n");
    script.push_str("        sys.exit(0)\n\n");
    script.push_str("    command = data.get('tool_input', {}).get('command', '')\n");
    script.push_str("    if not command:\n");
    script.push_str("        sys.exit(0)\n\n");
    script.push_str("    for rule in RULES:\n");
    script.push_str("        if re.search(rule['pattern'], command, re.IGNORECASE):\n");
    script.push_str("            if rule['level'] == 'L1':\n");
    script.push_str("                reason = 'BLOCKED by command guard (L1): ' + rule['reason']\n");
    script.push_str("                output = {'hookSpecificOutput': {'hookEventName': 'PreToolUse', 'permissionDecision': 'deny', 'permissionDecisionReason': reason}}\n");
    script.push_str("                print(json.dumps(output))\n");
    script.push_str("                sys.exit(2)\n");
    script.push_str("            else:\n");
    script.push_str("                reason = 'WARNING (L2): ' + rule['reason'] + '. Please verify this is intentional.'\n");
    script.push_str("                print(reason)  # stdout — injected into agent context\n");
    script.push_str("                sys.exit(0)\n\n");
    script.push_str("    sys.exit(0)\n\n");
    script.push_str("if __name__ == '__main__':\n");
    script.push_str("    main()\n");
    script
}
```

- [ ] **Step 3: Create compaction.rs — Recovery hook configuration**

```rust
//! Compaction recovery hook management (FR-009).
//!
//! The actual recovery script is a bash script (infra/hooks/compaction-recovery.sh).
//! This module manages the identity file and cognitive principles that the script injects.

use std::path::Path;

use crate::error::HookError;

/// Ensure the identity file exists at ~/.autonomic/identity.md.
pub async fn ensure_identity_file(state_dir: &Path) -> Result<(), HookError> {
    let identity_path = state_dir.join("identity.md");
    if identity_path.exists() {
        return Ok(());
    }

    let default_identity = r#"You are an AI-assisted development agent managed by the Autonomic orchestrator.

Key operating principles:
- Verify before claiming success
- One concern per commit
- Structure > Willpower — hooks enforce behavior, not instructions
- Check existing code before modifying
- Test before and after changes

Current orchestrator: autonomicd (Rust daemon)
"#;

    tokio::fs::write(&identity_path, default_identity).await?;
    Ok(())
}

/// Ensure the cognitive principles file exists.
pub async fn ensure_principles_file(state_dir: &Path) -> Result<(), HookError> {
    let principles_path = state_dir.join("cognitive-principles.md");
    if principles_path.exists() {
        return Ok(());
    }

    let default_principles = r#"## Cognitive Principles (re-injected after compaction)

1. **Read before writing** — Always read existing code before modifying it.
2. **Verify before claiming** — Run tests/checks before claiming something works.
3. **One concern per commit** — Each commit addresses a single logical change.
4. **Ask when uncertain** — If unsure, ask rather than guess.
5. **Preserve existing work** — Don't delete or overwrite without understanding why it exists.
"#;

    tokio::fs::write(&principles_path, default_principles).await?;
    Ok(())
}
```

- [ ] **Step 4: Commit**

```bash
git add crates/autonomic-hooks/src/managed/ infra/hooks/
git commit -m "feat(hooks): add command guard (FR-010) and compaction recovery config (FR-009)"
```

## Task 5: Managed Hook Scripts (from architecture doc)

**Note on command-guard.py:** Task 4 defines a Rust function that *generates* the Python script dynamically from configured rules. The generated script is written to `infra/hooks/command-guard.py` (or `~/.autonomic/hooks/` at runtime). Task 5 creates the other scripts from the architecture doc; the command guard comes from the generator.

**Files:**
- Create: `infra/hooks/compaction-recovery.sh`
- Create: `infra/hooks/session-start-context.sh`
- Create: `infra/hooks/session-metrics.sh`
- Create: `infra/hooks/pre-compact-marker.sh`

The exact scripts are specified in `docs/architecture/hook-system.md` §4.1-§4.6. The implementer should copy the scripts from that doc verbatim, as they are thoroughly designed and reviewed.

- [ ] **Step 1: Create all 4 hook scripts** from architecture doc §4.1-§4.4

- [ ] **Step 2: Make executable**

```bash
chmod +x infra/hooks/*.sh infra/hooks/*.py
```

- [ ] **Step 3: Commit**

```bash
git add infra/hooks/
git commit -m "feat(hooks): add managed hook scripts (compaction recovery, context, metrics, command guard)"
```

## Task 6: Daemon Hook Integration

**Files:**
- Modify: `crates/autonomic-daemon/Cargo.toml` — add `autonomic-hooks` dependency
- Modify: `crates/autonomic-daemon/src/main.rs` — load hook registry on startup
- Modify: `crates/autonomic-daemon/src/api.rs` — execute hooks at session lifecycle points

The `autonomic-hooks` crate is standalone, but the *daemon* depends on it to execute hooks at the right lifecycle moments. The daemon:
1. Loads `HookRegistry` from `~/.claude/settings.json` + project settings on startup
2. Before spawning a session: executes `SessionStart` hooks, captures stdout for context injection
3. On `run_session` completion: executes `Stop` hooks for metrics collection
4. The `POST /api/v1/sessions` handler passes hook stdout into the session's `system_prompt` field

This is where `autonomic-hooks` (standalone library) meets `autonomic-daemon` (composition root). The hooks crate has no knowledge of the daemon; the daemon orchestrates hook execution around session events.

- [ ] **Step 1: Add autonomic-hooks to daemon Cargo.toml**
- [ ] **Step 2: Load HookRegistry in daemon main.rs startup**
- [ ] **Step 3: Execute SessionStart hooks before session spawn in API handler**
- [ ] **Step 4: Execute Stop hooks after session completion**
- [ ] **Step 5: Commit**

```bash
git add crates/autonomic-daemon/
git commit -m "feat(daemon): integrate hook execution into session lifecycle"
```

## Task 7: Integration Verification

- [ ] **Step 1: Verify crate compiles**

```bash
SQLX_OFFLINE=true cargo check -p autonomic-hooks
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p autonomic-hooks
```

- [ ] **Step 3: Run clippy**

```bash
SQLX_OFFLINE=true cargo clippy -p autonomic-hooks --all-targets -- -D warnings
```

- [ ] **Step 4: Verify standalone — no sibling crate deps**

```bash
grep "autonomic-" crates/autonomic-hooks/Cargo.toml
# Expected: only "name = autonomic-hooks" lines, no dependency references
```

- [ ] **Step 5: Tag milestone**

```bash
git tag -a milestone/phase-2b -m "Phase 2B: Hook system — executor, registry, command guard, compaction recovery"
git tag -a milestone/phase-2 -m "Phase 2 complete: Memory and Hooks (FR-006 through FR-010)"
```
