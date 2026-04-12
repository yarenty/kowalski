//! Optional SQL persistence for durable episodic and agent metadata.
//!
//! **SQLite** (`sqlite:…`) is the default recommendation for single-node setups: no server, one file.
//! **PostgreSQL** (`postgres://…`) remains optional for scale and [`pgvector`](https://github.com/pgvector/pgvector).
//!
//! Native vector search in SQLite is not part of the base schema; semantic tier uses in-process cosine search,
//! or you can add [`sqlite-vec`](https://github.com/asg017/sqlite-vec) / pgvector later.

use crate::config::Config;
use crate::error::KowalskiError;
#[cfg(feature = "postgres")]
use sqlx::postgres::PgPool;
use sqlx::sqlite::SqlitePool;

fn db_err(e: sqlx::Error) -> KowalskiError {
    KowalskiError::Configuration(format!("database: {e}"))
}

fn migrate_err(e: sqlx::migrate::MigrateError) -> KowalskiError {
    KowalskiError::Configuration(format!("migration: {e}"))
}

/// Run embedded SQL migrations for `database_url`.
///
/// - **SQLite**: URLs starting with `sqlite:` or `sqlite://` (e.g. `sqlite:kowalski.db`, `sqlite::memory:`).
/// - **PostgreSQL**: `postgres://` or `postgresql://`.
pub async fn run_migrations(database_url: &str) -> Result<(), KowalskiError> {
    if database_url.starts_with("sqlite:") || database_url.starts_with("sqlite://") {
        let pool = SqlitePool::connect(database_url).await.map_err(db_err)?;
        sqlx::migrate!("../migrations/sqlite")
            .run(&pool)
            .await
            .map_err(migrate_err)?;
        pool.close().await;
        return Ok(());
    }
    if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
        #[cfg(feature = "postgres")]
        {
            let pool = PgPool::connect(database_url).await.map_err(db_err)?;
            sqlx::migrate!("../migrations/postgres")
                .run(&pool)
                .await
                .map_err(migrate_err)?;
            pool.close().await;
            return Ok(());
        }
        #[cfg(not(feature = "postgres"))]
        {
            return Err(crate::config::postgres_feature_required_error());
        }
    }
    Err(KowalskiError::Configuration(format!(
        "unsupported database_url (use sqlite:… or postgres://…): {database_url}"
    )))
}

/// When [`Config::memory`](crate::config::MemoryConfig) has `database_url` set, apply migrations before the agent uses the DB.
pub async fn run_memory_migrations_if_configured(config: &Config) -> Result<(), KowalskiError> {
    if let Some(ref url) = config.memory.database_url {
        run_migrations(url).await
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn sqlite_migrations_apply_on_memory() {
        run_migrations("sqlite::memory:").await.unwrap();
    }
}
