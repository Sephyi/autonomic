//! Git-backed state management using git CLI.
//!
//! Every config file mutation is auto-committed. Tags provide named
//! recovery points. Rollback restores tracked files without touching
//! gitignored content (XD-008).
//!
//! Phase 1 uses git CLI via tokio::process::Command for correctness.
//! Phase 2 will migrate to pure gix calls.

use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::info;

use crate::error::StateError;

/// Commit categories embedded in commit messages for filtering.
#[derive(Debug, Clone, Copy)]
pub enum CommitKind {
    Config,
    Evolution,
    Schedule,
    Snapshot,
    Migration,
    Rollback,
}

impl CommitKind {
    fn prefix(self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::Evolution => "evolution",
            Self::Schedule => "schedule",
            Self::Snapshot => "snapshot",
            Self::Migration => "migration",
            Self::Rollback => "rollback",
        }
    }
}

/// Manages the git repository at ~/.autonomic/.
pub struct GitManager {
    root: PathBuf,
}

impl GitManager {
    /// Open or initialize the git repository at the given path.
    pub async fn open_or_init(state_dir: &Path) -> Result<Self, StateError> {
        if !state_dir.exists() {
            tokio::fs::create_dir_all(state_dir).await?;
        }

        let git_dir = state_dir.join(".git");
        if !git_dir.exists() {
            let output = Command::new("git")
                .args(["init"])
                .current_dir(state_dir)
                .output()
                .await?;

            if !output.status.success() {
                return Err(StateError::Git(format!(
                    "git init failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }

            info!(path = %state_dir.display(), "Initialized new state repository");

            // Create .gitignore
            let gitignore = "logs/\nbackups/\n*.tmp\nsecrets.toml\ndaemon-heartbeat\nstate/sessions/*.pid\n.lock\n";
            tokio::fs::write(state_dir.join(".gitignore"), gitignore).await?;

            let manager = Self {
                root: state_dir.to_path_buf(),
            };
            manager
                .commit_all(CommitKind::Config, "initialize state repository")
                .await?;

            Ok(manager)
        } else {
            Ok(Self {
                root: state_dir.to_path_buf(),
            })
        }
    }

    /// Stage all changes and create a commit.
    pub async fn commit_all(&self, kind: CommitKind, message: &str) -> Result<String, StateError> {
        let full_message = format!("{}: {}", kind.prefix(), message);

        let output = Command::new("git")
            .args(["add", "-A"])
            .current_dir(&self.root)
            .output()
            .await?;

        if !output.status.success() {
            return Err(StateError::Git(format!(
                "git add failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        // Check if there's anything to commit
        let status_output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.root)
            .output()
            .await?;

        if status_output.stdout.is_empty() {
            return Ok(String::new());
        }

        let output = Command::new("git")
            .args([
                "commit",
                "-m",
                &full_message,
                "--author",
                "autonomic <autonomic@localhost>",
            ])
            .current_dir(&self.root)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("nothing to commit") {
                return Ok(String::new());
            }
            return Err(StateError::Git(format!("git commit failed: {stderr}")));
        }

        let hash_output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.root)
            .output()
            .await?;

        let hash = String::from_utf8_lossy(&hash_output.stdout)
            .trim()
            .to_string();
        Ok(hash)
    }

    /// Create an annotated tag at HEAD.
    pub async fn create_tag(&self, tag_name: &str, message: &str) -> Result<(), StateError> {
        let output = Command::new("git")
            .args(["tag", "-a", tag_name, "-m", message])
            .current_dir(&self.root)
            .output()
            .await?;

        if !output.status.success() {
            return Err(StateError::Git(format!(
                "git tag failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        info!(tag = tag_name, "Created tag");
        Ok(())
    }

    /// List all tags matching a pattern.
    pub async fn list_tags(&self, pattern: Option<&str>) -> Result<Vec<String>, StateError> {
        let mut args = vec!["tag", "-l"];
        if let Some(p) = pattern {
            args.push(p);
        }

        let output = Command::new("git")
            .args(&args)
            .current_dir(&self.root)
            .output()
            .await?;

        let tags: Vec<String> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect();

        Ok(tags)
    }

    /// Rollback to a specific tag or commit.
    /// Creates a pre-rollback snapshot tag first.
    /// Uses `git checkout` on tracked files only (XD-008: preserves gitignored files).
    pub async fn rollback(&self, target: &str) -> Result<(), StateError> {
        let verify = Command::new("git")
            .args(["rev-parse", "--verify", target])
            .current_dir(&self.root)
            .output()
            .await?;

        if !verify.status.success() {
            return Err(StateError::InvalidTarget(target.to_string()));
        }

        let now = chrono::Utc::now().format("%Y%m%d-%H%M%S").to_string();
        self.create_tag(
            &format!("pre-rollback/{now}"),
            &format!("State before rollback to {target}"),
        )
        .await?;

        // Checkout tracked files from target (XD-008: no git clean)
        let output = Command::new("git")
            .args(["checkout", target, "--", "."])
            .current_dir(&self.root)
            .output()
            .await?;

        if !output.status.success() {
            return Err(StateError::Git(format!(
                "git checkout failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        self.commit_all(CommitKind::Rollback, &format!("restored state to {target}"))
            .await?;

        info!(target = target, "Rolled back state");
        Ok(())
    }

    /// Create a daily snapshot tag.
    pub async fn create_daily_snapshot(&self) -> Result<String, StateError> {
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let tag_name = format!("daily/{today}");

        self.commit_all(CommitKind::Snapshot, &format!("daily snapshot {today}"))
            .await?;
        self.create_tag(&tag_name, &format!("Daily snapshot {today}"))
            .await?;

        Ok(tag_name)
    }

    /// Get the repository root path.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup() -> (TempDir, GitManager) {
        let dir = TempDir::new().unwrap();
        let manager = GitManager::open_or_init(dir.path()).await.unwrap();
        (dir, manager)
    }

    #[tokio::test]
    async fn init_creates_git_repo() {
        let (dir, _manager) = setup().await;
        assert!(dir.path().join(".git").exists());
        assert!(dir.path().join(".gitignore").exists());
    }

    #[tokio::test]
    async fn open_existing_repo() {
        let (dir, _manager1) = setup().await;
        let _manager2 = GitManager::open_or_init(dir.path()).await.unwrap();
    }

    #[tokio::test]
    async fn commit_and_tag() {
        let (dir, manager) = setup().await;

        tokio::fs::write(dir.path().join("test.toml"), "[test]\nkey = \"value\"")
            .await
            .unwrap();

        let hash = manager
            .commit_all(CommitKind::Config, "add test config")
            .await
            .unwrap();
        assert!(!hash.is_empty());

        manager.create_tag("test/v1", "Test tag").await.unwrap();
        let tags = manager.list_tags(Some("test/*")).await.unwrap();
        assert!(tags.contains(&"test/v1".to_string()));
    }

    #[tokio::test]
    async fn rollback_preserves_gitignored() {
        let (dir, manager) = setup().await;

        // Create a tracked file and commit
        tokio::fs::write(dir.path().join("config.toml"), "v1")
            .await
            .unwrap();
        manager
            .commit_all(CommitKind::Config, "v1 config")
            .await
            .unwrap();
        manager.create_tag("v1", "Version 1").await.unwrap();

        // Modify tracked file and create gitignored file
        tokio::fs::write(dir.path().join("config.toml"), "v2")
            .await
            .unwrap();
        tokio::fs::write(dir.path().join("secrets.toml"), "secret data")
            .await
            .unwrap();
        manager
            .commit_all(CommitKind::Config, "v2 config")
            .await
            .unwrap();

        // Rollback to v1
        manager.rollback("v1").await.unwrap();

        // Tracked file should be restored
        let content = tokio::fs::read_to_string(dir.path().join("config.toml"))
            .await
            .unwrap();
        assert_eq!(content, "v1");

        // Gitignored file should survive (XD-008)
        assert!(dir.path().join("secrets.toml").exists());
    }

    #[tokio::test]
    async fn rollback_invalid_target() {
        let (_dir, manager) = setup().await;
        let result = manager.rollback("nonexistent-tag").await;
        assert!(matches!(result, Err(StateError::InvalidTarget(_))));
    }
}
