//! Core types, traits, and configuration for Autonomic.

pub mod config;
pub mod error;
pub mod rate_budget;
pub mod types;

pub use config::{AutonomicConfig, SecretsConfig, load_config, load_secrets};
pub use error::CoreError;
pub use rate_budget::{BudgetAllocation, CostEntry, RateBudget};
pub use types::{ModelTier, ProjectId, SessionId, SessionStatus, Subsystem};
