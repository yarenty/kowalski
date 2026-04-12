//! Load / upsert [`AgentRecord`] in Postgres (`federation_registry` table).

use crate::error::KowalskiError;
use crate::federation::registry::{AgentRecord, AgentRegistry};

#[cfg(feature = "postgres")]
pub async fn load_registry_into(registry: &AgentRegistry, database_url: &str) -> Result<(), KowalskiError> {
    use sqlx::postgres::PgPool;
    use sqlx::Row;
    let pool = PgPool::connect(database_url)
        .await
        .map_err(|e| KowalskiError::Federation(format!("registry load connect: {e}")))?;
    let rows = sqlx::query("SELECT agent_id, capabilities FROM federation_registry ORDER BY agent_id")
        .fetch_all(&pool)
        .await
        .map_err(|e| KowalskiError::Federation(format!("registry load query: {e}")))?;
    for row in rows {
        let id: String = row.try_get("agent_id").map_err(|e| {
            KowalskiError::Federation(format!("registry load row: {e}"))
        })?;
        let caps_val: serde_json::Value = row.try_get("capabilities").map_err(|e| {
            KowalskiError::Federation(format!("registry load row: {e}"))
        })?;
        let capabilities: Vec<String> = serde_json::from_value(caps_val).unwrap_or_default();
        registry.register(AgentRecord { id, capabilities })?;
    }
    Ok(())
}

#[cfg(not(feature = "postgres"))]
pub async fn load_registry_into(
    _registry: &AgentRegistry,
    _database_url: &str,
) -> Result<(), KowalskiError> {
    Ok(())
}

#[cfg(feature = "postgres")]
pub async fn upsert_registry_record(
    database_url: &str,
    record: &AgentRecord,
) -> Result<(), KowalskiError> {
    use sqlx::postgres::PgPool;
    let pool = PgPool::connect(database_url)
        .await
        .map_err(|e| KowalskiError::Federation(format!("registry upsert connect: {e}")))?;
    let caps = serde_json::to_value(&record.capabilities).map_err(KowalskiError::Json)?;
    sqlx::query(
        r#"INSERT INTO federation_registry (agent_id, capabilities)
           VALUES ($1, $2)
           ON CONFLICT (agent_id) DO UPDATE SET capabilities = $2, updated_at = NOW()"#,
    )
    .bind(&record.id)
    .bind(caps)
    .execute(&pool)
    .await
    .map_err(|e| KowalskiError::Federation(format!("registry upsert: {e}")))?;
    Ok(())
}

#[cfg(not(feature = "postgres"))]
pub async fn upsert_registry_record(
    _database_url: &str,
    _record: &AgentRecord,
) -> Result<(), KowalskiError> {
    Ok(())
}
