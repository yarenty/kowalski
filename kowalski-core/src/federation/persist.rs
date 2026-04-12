//! Load / upsert [`AgentRecord`] in Postgres (`federation_registry` table) and optional
//! [`AgentStateSnapshot`] rows (`agent_state` from the initial schema migration).

use crate::error::KowalskiError;
use crate::federation::registry::{AgentRecord, AgentRegistry};

#[cfg(feature = "postgres")]
use serde::{Deserialize, Serialize};

/// Row from `agent_state` (heartbeat / task metadata).
#[cfg(feature = "postgres")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStateSnapshot {
    pub agent_id: String,
    pub current_task: Option<String>,
    pub active: bool,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub capabilities: Vec<String>,
}

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

/// Upsert `agent_state` for a registered agent (capabilities mirror the in-memory registry).
#[cfg(feature = "postgres")]
pub async fn upsert_agent_state_for_record(
    database_url: &str,
    record: &AgentRecord,
) -> Result<(), KowalskiError> {
    use sqlx::postgres::PgPool;
    let pool = PgPool::connect(database_url)
        .await
        .map_err(|e| KowalskiError::Federation(format!("agent_state upsert connect: {e}")))?;
    let caps = serde_json::to_value(&record.capabilities).map_err(KowalskiError::Json)?;
    sqlx::query(
        r#"INSERT INTO agent_state (agent_id, capabilities, active, updated_at)
           VALUES ($1, $2::jsonb, true, NOW())
           ON CONFLICT (agent_id) DO UPDATE SET
             capabilities = EXCLUDED.capabilities,
             active = true,
             updated_at = NOW()"#,
    )
    .bind(&record.id)
    .bind(caps)
    .execute(&pool)
    .await
    .map_err(|e| KowalskiError::Federation(format!("agent_state upsert: {e}")))?;
    Ok(())
}

#[cfg(not(feature = "postgres"))]
pub async fn upsert_agent_state_for_record(
    _database_url: &str,
    _record: &AgentRecord,
) -> Result<(), KowalskiError> {
    Ok(())
}

/// All `agent_state` rows (for merging with the in-memory registry).
#[cfg(feature = "postgres")]
pub async fn load_agent_states(
    database_url: &str,
) -> Result<std::collections::HashMap<String, AgentStateSnapshot>, KowalskiError> {
    use sqlx::postgres::PgPool;
    use sqlx::Row;
    use std::collections::HashMap;

    let pool = PgPool::connect(database_url)
        .await
        .map_err(|e| KowalskiError::Federation(format!("agent_state load connect: {e}")))?;
    let rows = sqlx::query(
        "SELECT agent_id, current_task, active, updated_at, capabilities FROM agent_state",
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| KowalskiError::Federation(format!("agent_state load query: {e}")))?;

    let mut out = HashMap::new();
    for row in rows {
        let agent_id: String = row.try_get("agent_id").map_err(|e| {
            KowalskiError::Federation(format!("agent_state row: {e}"))
        })?;
        let current_task: Option<String> = row.try_get("current_task").map_err(|e| {
            KowalskiError::Federation(format!("agent_state row: {e}"))
        })?;
        let active: bool = row
            .try_get("active")
            .map_err(|e| KowalskiError::Federation(format!("agent_state row: {e}")))?;
        let updated_at: chrono::DateTime<chrono::Utc> = row.try_get("updated_at").map_err(|e| {
            KowalskiError::Federation(format!("agent_state row: {e}"))
        })?;
        let caps_val: serde_json::Value = row.try_get("capabilities").map_err(|e| {
            KowalskiError::Federation(format!("agent_state row: {e}"))
        })?;
        let capabilities: Vec<String> = serde_json::from_value(caps_val).unwrap_or_default();
        out.insert(
            agent_id.clone(),
            AgentStateSnapshot {
                agent_id,
                current_task,
                active,
                updated_at,
                capabilities,
            },
        );
    }
    Ok(out)
}

/// Heartbeat: bump `updated_at` and set `active` (insert minimal row if missing).
#[cfg(feature = "postgres")]
pub async fn touch_agent_heartbeat(database_url: &str, agent_id: &str) -> Result<(), KowalskiError> {
    use sqlx::postgres::PgPool;
    let pool = PgPool::connect(database_url)
        .await
        .map_err(|e| KowalskiError::Federation(format!("agent_state heartbeat connect: {e}")))?;
    sqlx::query(
        r#"INSERT INTO agent_state (agent_id, capabilities, active, updated_at)
           VALUES ($1, '[]'::jsonb, true, NOW())
           ON CONFLICT (agent_id) DO UPDATE SET
             updated_at = NOW(),
             active = true"#,
    )
    .bind(agent_id)
    .execute(&pool)
    .await
    .map_err(|e| KowalskiError::Federation(format!("agent_state heartbeat: {e}")))?;
    Ok(())
}

#[cfg(not(feature = "postgres"))]
pub async fn touch_agent_heartbeat(_database_url: &str, _agent_id: &str) -> Result<(), KowalskiError> {
    Ok(())
}
