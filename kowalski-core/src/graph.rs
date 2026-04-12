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

/// Run a Cypher query via Apache AGE [`cypher`](https://age.apache.org/).
/// The query must expose a single `agtype` column named `result` (e.g. `RETURN x AS result`).
#[cfg(feature = "postgres")]
pub async fn postgres_age_cypher(
    database_url: &str,
    graph_name: &str,
    cypher: &str,
) -> Result<serde_json::Value, KowalskiError> {
    use serde_json::json;
    use sqlx::postgres::PgPool;

    let pool = PgPool::connect(database_url)
        .await
        .map_err(|e| KowalskiError::Configuration(format!("age cypher connect: {e}")))?;

    let age: bool = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')",
    )
    .fetch_one(&pool)
    .await
    .map_err(|e| KowalskiError::Configuration(format!("age cypher: {e}")))?;

    if !age {
        return Err(KowalskiError::Configuration(
            "Apache AGE extension is not installed on this Postgres instance".into(),
        ));
    }

    // AGE requires `ag_catalog` on `search_path` (and often `LOAD 'age'`) on the same session as
    // `cypher(...)`. Pool connections are arbitrary; use one connection for SET + query.
    let mut conn = pool
        .acquire()
        .await
        .map_err(|e| KowalskiError::Configuration(format!("age cypher acquire: {e}")))?;
    let _ = sqlx::query("LOAD 'age'").execute(&mut *conn).await;
    sqlx::query("SET search_path = ag_catalog, public")
        .execute(&mut *conn)
        .await
        .map_err(|e| KowalskiError::Configuration(format!("age search_path: {e}")))?;

    let raw: Vec<Option<String>> = sqlx::query_scalar(
        r#"SELECT (result)::text FROM cypher($1::name, $2::cstring) AS (result agtype)"#,
    )
    .bind(graph_name)
    .bind(cypher)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| KowalskiError::Configuration(format!("cypher execution: {e}")))?;

    let rows: Vec<serde_json::Value> = raw.into_iter().map(|t| json!(t)).collect();
    Ok(json!({ "rows": rows }))
}

#[cfg(not(feature = "postgres"))]
pub async fn postgres_age_cypher(
    _database_url: &str,
    _graph_name: &str,
    _cypher: &str,
) -> Result<serde_json::Value, KowalskiError> {
    Err(KowalskiError::Configuration(
        "postgres feature not enabled".into(),
    ))
}
