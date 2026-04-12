//! Optional Postgres extension probes (pgvector, Apache AGE).

use crate::error::KowalskiError;

/// Report whether `vector` and `age` extensions are installed.
#[cfg(feature = "postgres")]
pub async fn postgres_graph_status(database_url: &str) -> Result<serde_json::Value, KowalskiError> {
    use serde_json::json;
    use sqlx::postgres::PgPool;
    let pool = PgPool::connect(database_url)
        .await
        .map_err(|e| KowalskiError::Configuration(format!("graph status connect: {e}")))?;
    let vector: bool =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'vector')")
            .fetch_one(&pool)
            .await
            .map_err(|e| KowalskiError::Configuration(format!("graph status: {e}")))?;
    let age: bool =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')")
            .fetch_one(&pool)
            .await
            .map_err(|e| KowalskiError::Configuration(format!("graph status: {e}")))?;
    Ok(json!({
        "postgres": true,
        "vector_extension": vector,
        "age_extension": age,
    }))
}

#[cfg(not(feature = "postgres"))]
pub async fn postgres_graph_status(_database_url: &str) -> Result<serde_json::Value, KowalskiError> {
    Err(KowalskiError::Configuration(
        "postgres feature not enabled".into(),
    ))
}
