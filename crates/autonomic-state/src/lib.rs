//! Git-backed state management with point-in-time recovery.

pub mod atomic;
pub mod error;
pub mod git;
pub mod lock;
pub mod manager;

pub use atomic::{atomic_toml_update, atomic_write_and_commit};
pub use error::StateError;
pub use git::{CommitKind, GitManager};
pub use lock::StateLock;
pub use manager::StateManager;
