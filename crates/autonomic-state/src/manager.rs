//! Unified state manager combining git, lock, and database pool.

use sqlx::PgPool;
use std::path::{Path, PathBuf};
use tracing::info;

use crate::error::StateError;
use crate::git::{CommitKind, GitManager};
use crate::lock::StateLock;

/// The single entry point for all state operations.
/// Owns the exclusive lock, git repo, and database pool.
pub struct StateManager {
    /// Root directory (~/.autonomic/).
    root: PathBuf,
    /// Git repository manager.
    git: GitManager,
    /// Exclusive file lock (held for daemon lifetime).
    _lock: StateLock,
    /// PostgreSQL connection pool.
    pool: PgPool,
}

impl StateManager {
    /// Open (or initialize) the state store.
    /// Acquires the exclusive lock, initializes git if needed,
    /// and connects to PostgreSQL.
    pub async fn open(state_dir: &Path, database_url: &str) -> Result<Self, StateError> {
        tokio::fs::create_dir_all(state_dir).await?;

        let lock = StateLock::acquire(state_dir)?;
        info!(path = %state_dir.display(), "State lock acquired");

        let git = GitManager::open_or_init(state_dir).await?;

        let pool = autonomic_db::create_pool(database_url)
            .await
            .map_err(|e| StateError::Git(e.to_string()))?;

        autonomic_db::run_migrations(&pool)
            .await
            .map_err(|e| StateError::Git(e.to_string()))?;

        Ok(StateManager {
            root: state_dir.to_path_buf(),
            git,
            _lock: lock,
            pool,
        })
    }

    /// Get the git manager for direct git operations.
    pub fn git(&self) -> &GitManager {
        &self.git
    }

    /// Get the database pool for SQL queries.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Get the state directory root path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Path to config.toml.
    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    /// Path to secrets.toml.
    pub fn secrets_path(&self) -> PathBuf {
        self.root.join("secrets.toml")
    }

    /// Create a named snapshot (tag).
    pub async fn snapshot(&self, tag_name: &str, message: &str) -> Result<(), StateError> {
        self.git
            .commit_all(CommitKind::Snapshot, &format!("snapshot for {tag_name}"))
            .await?;
        self.git.create_tag(tag_name, message).await
    }

    /// Rollback to a tag or commit. Preserves gitignored files (XD-008).
    pub async fn rollback(&self, target: &str) -> Result<(), StateError> {
        self.git.rollback(target).await
    }
}
