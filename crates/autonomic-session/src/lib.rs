//! Claude Code subprocess management, output parsing, and cost tracking.
//!
//! This crate owns the full lifecycle of Claude Code subprocess instances.
//! It is the ONLY component that spawns `claude` processes (XD-001).

pub mod command;
pub mod config;
pub mod cost;
pub mod error;
pub mod manager;
pub mod parser;

pub use command::build_container_config;
pub use config::{SessionConfig, WorktreeHandle};
pub use cost::CostTracker;
pub use error::{SessionConfigError, SessionError};
pub use manager::{SessionEvent, SessionManager, SessionRecord, SessionState};
pub use parser::{SessionOutcome, StreamMessage, classify_result, parse_line};
