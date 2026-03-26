//! State management error types.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("Another Autonomic instance is already running")]
    AlreadyRunning,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Git error: {0}")]
    Git(String),

    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid path: {0}")]
    InvalidPath(PathBuf),

    #[error("State directory not found: {0}")]
    StateDirNotFound(PathBuf),

    #[error("Invalid rollback target: {0}")]
    InvalidTarget(String),

    #[error("Temp file persist error: {0}")]
    Persist(#[from] tempfile::PersistError),

    #[error("Path strip prefix error: {0}")]
    StripPrefix(#[from] std::path::StripPrefixError),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}
