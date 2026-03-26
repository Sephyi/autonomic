//! Core types, traits, and configuration for Autonomic.

pub mod error;
pub mod types;

pub use error::CoreError;
pub use types::{ModelTier, ProjectId, SessionId, SessionStatus, Subsystem};
