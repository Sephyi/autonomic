//! Database error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Database connection failed: {0}")]
    Connection(#[from] sqlx::Error),

    #[error("Migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("Database not available — is Postgres running? (infra/compose.yaml)")]
    Unavailable,
}
