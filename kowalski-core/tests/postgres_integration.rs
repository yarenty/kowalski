//! Integration tests against a real Postgres when `DATABASE_URL` is set (CI provides it).

#![cfg(feature = "postgres")]

use sqlx::postgres::PgPool;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok()
}

#[tokio::test]
async fn postgres_connects_and_selects() {
    let Some(url) = database_url() else {
        eprintln!("skip postgres_integration: DATABASE_URL not set");
        return;
    };

    let pool = PgPool::connect(&url)
        .await
        .expect("DATABASE_URL connect");
    let v: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect("SELECT 1");
    assert_eq!(v, 1);
}

#[tokio::test]
async fn postgres_graph_status_smoke() {
    let Some(url) = database_url() else {
        eprintln!("skip postgres_integration: DATABASE_URL not set");
        return;
    };

    let status = kowalski_core::postgres_graph_status(&url)
        .await
        .expect("graph status");
    assert_eq!(status["postgres"], true);
}
