//! Git-backed state management with point-in-time recovery.

pub mod error;
pub mod lock;

pub use error::StateError;
pub use lock::StateLock;
