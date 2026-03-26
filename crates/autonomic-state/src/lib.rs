//! Git-backed state management with point-in-time recovery.

pub mod error;
pub mod git;
pub mod lock;

pub use error::StateError;
pub use git::{CommitKind, GitManager};
pub use lock::StateLock;
