//! PostgreSQL connection pool management.

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tracing::info;

use crate::error::DbError;

/// Create a connection pool for the daemon (read-write, higher connection limit).
pub async fn create_pool(database_url: &str) -> Result<PgPool, DbError> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .map_err(|e| {
            if e.to_string().contains("Connection refused") {
                return DbError::Unavailable;
            }
            DbError::Connection(e)
        })?;

    info!("Database pool created (max_connections=10)");
    Ok(pool)
}

/// Create a read-only pool for the CLI (lower connection limit).
pub async fn create_readonly_pool(database_url: &str) -> Result<PgPool, DbError> {
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(database_url)
        .await
        .map_err(|e| {
            if e.to_string().contains("Connection refused") {
                return DbError::Unavailable;
            }
            DbError::Connection(e)
        })?;

    Ok(pool)
}

/// Run all pending migrations. Call on daemon startup before any other DB access.
pub async fn run_migrations(pool: &PgPool) -> Result<(), DbError> {
    sqlx::migrate!("../../infra/migrations").run(pool).await?;

    info!("Database migrations applied successfully");
    Ok(())
}
