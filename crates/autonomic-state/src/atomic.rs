//! Atomic file write operations.
//!
//! Every state mutation follows: write tmp → fsync → rename → git commit.
//! This prevents partial writes on crash.

use serde::{Serialize, de::DeserializeOwned};
use std::path::Path;

use crate::error::StateError;
use crate::git::{CommitKind, GitManager};

/// Atomically write content to a file, then commit to git.
pub async fn atomic_write_and_commit(
    git: &GitManager,
    target: &Path,
    content: &[u8],
    kind: CommitKind,
    message: &str,
) -> Result<String, StateError> {
    // Ensure parent directory exists
    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Write to temp file in the same directory (same filesystem for atomic rename)
    let parent = target
        .parent()
        .ok_or_else(|| StateError::InvalidPath(target.to_path_buf()))?;

    // Use spawn_blocking for the tempfile + persist since those are sync operations
    let content_owned = content.to_vec();
    let parent_owned = parent.to_path_buf();
    let target_owned = target.to_path_buf();

    tokio::task::spawn_blocking(move || -> Result<(), StateError> {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new_in(&parent_owned)?;
        tmp.write_all(&content_owned)?;
        tmp.as_file().sync_all()?;
        tmp.persist(&target_owned)?;
        Ok(())
    })
    .await
    .expect("spawn_blocking panicked")?;

    git.commit_all(kind, message).await
}

/// Atomically read-modify-write a TOML config file, then commit.
pub async fn atomic_toml_update<T: Serialize + DeserializeOwned>(
    git: &GitManager,
    path: &Path,
    kind: CommitKind,
    message: &str,
    mutate: impl FnOnce(&mut T) -> Result<(), StateError>,
) -> Result<String, StateError> {
    let content = tokio::fs::read_to_string(path).await?;
    let mut value: T = toml::from_str(&content)?;
    mutate(&mut value)?;
    let new_content = toml::to_string_pretty(&value)?;
    atomic_write_and_commit(git, path, new_content.as_bytes(), kind, message).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::GitManager;
    use serde::{Deserialize, Serialize};
    use tempfile::TempDir;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestConfig {
        name: String,
        value: i32,
    }

    async fn setup() -> (TempDir, GitManager) {
        let dir = TempDir::new().unwrap();
        let git = GitManager::open_or_init(dir.path()).await.unwrap();
        (dir, git)
    }

    #[tokio::test]
    async fn atomic_write_creates_file_and_commits() {
        let (dir, git) = setup().await;
        let target = dir.path().join("test.txt");

        let hash = atomic_write_and_commit(
            &git,
            &target,
            b"hello world",
            CommitKind::Config,
            "add test file",
        )
        .await
        .unwrap();

        assert!(target.exists());
        assert_eq!(
            tokio::fs::read_to_string(&target).await.unwrap(),
            "hello world"
        );
        assert!(!hash.is_empty());
    }

    #[tokio::test]
    async fn atomic_toml_update_modifies_and_commits() {
        let (dir, git) = setup().await;
        let target = dir.path().join("config.toml");

        let initial = TestConfig {
            name: "test".into(),
            value: 1,
        };
        tokio::fs::write(&target, toml::to_string_pretty(&initial).unwrap())
            .await
            .unwrap();
        git.commit_all(CommitKind::Config, "initial").await.unwrap();

        atomic_toml_update::<TestConfig>(
            &git,
            &target,
            CommitKind::Config,
            "update value",
            |config| {
                config.value = 42;
                Ok(())
            },
        )
        .await
        .unwrap();

        let content = tokio::fs::read_to_string(&target).await.unwrap();
        let updated: TestConfig = toml::from_str(&content).unwrap();
        assert_eq!(updated.value, 42);
    }

    #[tokio::test]
    async fn atomic_write_creates_parent_dirs() {
        let (dir, git) = setup().await;
        let target = dir.path().join("nested/deep/file.toml");

        atomic_write_and_commit(&git, &target, b"content", CommitKind::Config, "nested file")
            .await
            .unwrap();

        assert!(target.exists());
    }
}
