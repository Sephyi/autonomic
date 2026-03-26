//! Database migrations, schemas, and compile-time checked queries for Autonomic.

pub mod error;
pub mod pool;

pub use error::DbError;
pub use pool::{create_pool, create_readonly_pool, run_migrations};

// Re-export sqlx::PgPool for downstream crates
pub use sqlx::PgPool;
