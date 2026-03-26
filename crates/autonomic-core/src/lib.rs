//! Core types, traits, and configuration for Autonomic.

pub mod error;
pub mod rate_budget;
pub mod types;

pub use error::CoreError;
pub use rate_budget::{BudgetAllocation, CostEntry, RateBudget};
pub use types::{ModelTier, ProjectId, SessionId, SessionStatus, Subsystem};
