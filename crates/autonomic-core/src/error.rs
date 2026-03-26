//! Core error types for Autonomic.
//!
//! Each downstream crate defines its own error enum with `#[from]` conversions
//! for CoreError where needed. This crate provides only the shared error cases.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can originate from core operations.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Configuration file not found: {0}")]
    ConfigNotFound(PathBuf),

    #[error("Invalid path: {0}")]
    InvalidPath(PathBuf),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML deserialization error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
