//! Apache AGE `postgres_age_cypher` integration test. Requires the `age` extension (e.g. official
//! [`apache/age`](https://hub.docker.com/r/apache/age) image in CI). Skips when `DATABASE_URL` is
//! unset or `age` is not installed.

#![cfg(feature = "postgres")]

use sqlx::postgres::PgPool;
use uuid::Uuid;

#[tokio::test]
async fn postgres_age_cypher_returns_rows() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("skip age_cypher: DATABASE_URL not set");
        return;
    };

    let pool = PgPool::connect(&url).await.expect("DATABASE_URL connect");

    let age_ok: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age')",
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(false);

    if !age_ok {
        eprintln!("skip age_cypher: Apache AGE extension not installed");
        return;
    }

    let _ = sqlx::query("CREATE EXTENSION IF NOT EXISTS age")
        .execute(&pool)
        .await;

    let graph = format!("g{}", Uuid::new_v4().as_simple());
    let create_sql = format!("SELECT create_graph('{graph}')");
    sqlx::query(&create_sql)
        .execute(&pool)
        .await
        .expect("create_graph");

    let out = kowalski_core::postgres_age_cypher(&url, &graph, "RETURN 42 AS result")
        .await
        .expect("postgres_age_cypher");

    let rows = out.get("rows").expect("rows key");
    assert!(rows.is_array());
    assert!(!rows.as_array().unwrap().is_empty());
}
