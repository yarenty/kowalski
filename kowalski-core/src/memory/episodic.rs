// Tier 2: Episodic Buffer (The Journal)
// Persistent log of recent conversations using embedded SQLite (no RocksDB, no separate server).

use crate::{
    error::KowalskiError,
    memory::{MemoryProvider, MemoryQuery, MemoryUnit},
};
use async_trait::async_trait;
use log::{debug, error, info};
use serde_json;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use sqlx::Row;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Schema for the episodic SQLite file (same as `migrations/sqlite/002_episodic_kv.sql`).
const EPISODIC_KV_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS episodic_kv (
    id TEXT PRIMARY KEY NOT NULL,
    payload TEXT NOT NULL
);
"#;

/// Resolve filesystem path for the episodic DB file and ensure parent directories exist.
fn episodic_db_file(episodic_path: &str) -> Result<PathBuf, KowalskiError> {
    let p = episodic_path.trim_end_matches('/');
    let file_path: PathBuf = if p.ends_with(".sqlite") || p.ends_with(".db") {
        PathBuf::from(p)
    } else {
        Path::new(p).join("episodic.sqlite")
    };
    if let Some(parent) = file_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                KowalskiError::Memory(format!("create episodic directory: {e}"))
            })?;
        }
    }
    Ok(file_path)
}

/// A persistent memory store using **embedded SQLite** (single file on disk, no daemon).
///
/// Memory units are stored as JSON in `episodic_kv`, keyed by `MemoryUnit.id`.
pub struct EpisodicBuffer {
    pool: SqlitePool,
    llm_provider: std::sync::Arc<dyn crate::llm::LLMProvider>,
}

impl EpisodicBuffer {
    /// Opens or creates an episodic buffer at `episodic_path`.
    ///
    /// * If `episodic_path` ends with `.sqlite` or `.db`, that file is used.
    /// * Otherwise it is treated as a **directory** and `episodic.sqlite` is created inside it
    ///   (same layout as the former RocksDB directory convention).
    pub async fn open(
        episodic_path: &str,
        llm_provider: std::sync::Arc<dyn crate::llm::LLMProvider>,
    ) -> Result<Self, KowalskiError> {
        let file = episodic_db_file(episodic_path)?;
        info!("Opening episodic SQLite buffer at {}", file.display());
        let opts = SqliteConnectOptions::new()
            .filename(&file)
            .create_if_missing(true);
        let pool = SqlitePool::connect_with(opts)
            .await
            .map_err(|e| KowalskiError::Memory(format!("episodic SQLite connect: {e}")))?;
        sqlx::query(EPISODIC_KV_SCHEMA)
            .execute(&pool)
            .await
            .map_err(|e| KowalskiError::Memory(format!("episodic schema: {e}")))?;
        Ok(Self { pool, llm_provider })
    }

    pub async fn retrieve_all(&self) -> Result<Vec<MemoryUnit>, KowalskiError> {
        let rows = sqlx::query("SELECT id, payload FROM episodic_kv ORDER BY id")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| KowalskiError::Memory(e.to_string()))?;
        let mut memories = Vec::with_capacity(rows.len());
        for row in rows {
            let payload: String = row
                .try_get("payload")
                .map_err(|e| KowalskiError::Memory(e.to_string()))?;
            match serde_json::from_str::<MemoryUnit>(&payload) {
                Ok(unit) => memories.push(unit),
                Err(e) => error!("Failed to deserialize memory unit from SQLite: {}", e),
            }
        }
        Ok(memories)
    }

    pub async fn delete(&mut self, id: &str) -> Result<(), KowalskiError> {
        sqlx::query("DELETE FROM episodic_kv WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| KowalskiError::Memory(e.to_string()))?;
        Ok(())
    }

    pub async fn add_with_embedding(
        &mut self,
        mut memory: MemoryUnit,
    ) -> Result<(), KowalskiError> {
        info!("[EpisodicBuffer] Adding memory unit: {}", memory.id);
        debug!("Adding memory unit to episodic buffer: {}", memory.id);
        if memory.embedding.is_none() {
            match self.llm_provider.embed(&memory.content).await {
                Ok(embedding) => memory.embedding = Some(embedding),
                Err(e) => {
                    error!("Failed to get embedding for memory {}: {}", memory.id, e);
                }
            }
        }
        self.upsert_unit(&memory).await
    }

    async fn upsert_unit(&self, memory: &MemoryUnit) -> Result<(), KowalskiError> {
        let key = memory.id.clone();
        let value = serde_json::to_string(memory).map_err(|e| {
            error!("Failed to serialize memory unit {}: {}", key, e);
            KowalskiError::Memory(e.to_string())
        })?;
        sqlx::query(
            "INSERT INTO episodic_kv (id, payload) VALUES (?, ?)
             ON CONFLICT(id) DO UPDATE SET payload = excluded.payload",
        )
        .bind(&key)
        .bind(&value)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            error!("Failed to write episodic row {}: {}", key, e);
            KowalskiError::Memory(e.to_string())
        })?;
        Ok(())
    }

    async fn load_all_units(&self) -> Result<Vec<MemoryUnit>, KowalskiError> {
        let rows = sqlx::query("SELECT id, payload FROM episodic_kv")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| KowalskiError::Memory(e.to_string()))?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let payload: String = row
                .try_get("payload")
                .map_err(|e| KowalskiError::Memory(e.to_string()))?;
            if let Ok(unit) = serde_json::from_str::<MemoryUnit>(&payload) {
                out.push(unit);
            } else {
                error!("Failed to deserialize episodic payload");
            }
        }
        Ok(out)
    }

    pub async fn retrieve_with_embedding(
        &self,
        query: &str,
        retrieval_limit: usize,
    ) -> Result<Vec<MemoryUnit>, KowalskiError> {
        info!("[EpisodicBuffer][RETRIEVE] Query: '{}'", query);
        let query_embedding = self.llm_provider.embed(query).await.ok();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let max_time_span = 60 * 60 * 24 * 30u64;
        let units = self.load_all_units().await?;
        let mut scored = Vec::new();
        let mut fallback_results = Vec::new();
        for unit in units {
            if let (Some(q_emb), Some(m_emb)) = (query_embedding.as_ref(), unit.embedding.as_ref())
            {
                let sim = cosine_similarity(q_emb, m_emb);
                let recency = 1.0
                    - ((now.saturating_sub(unit.timestamp)) as f32 / max_time_span as f32).min(1.0);
                let recency = recency.max(0.0);
                let score = 0.85 * sim + 0.15 * recency;
                scored.push((score, unit));
            } else {
                let lower_query = query.to_lowercase().trim().to_string();
                let query_words: Vec<&str> = lower_query.split_whitespace().collect();
                let content = unit.content.to_lowercase();
                if query_words.iter().any(|w| content.contains(w)) {
                    fallback_results.push(unit);
                }
            }
        }
        if !scored.is_empty() {
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            return Ok(scored
                .into_iter()
                .map(|(_, u)| u)
                .take(retrieval_limit)
                .collect());
        }
        let results = if fallback_results.len() > retrieval_limit {
            fallback_results[fallback_results.len() - retrieval_limit..].to_vec()
        } else {
            fallback_results
        };
        Ok(results)
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot = a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
    let norm_a = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

#[async_trait]
impl MemoryProvider for EpisodicBuffer {
    async fn add(&mut self, mut memory: MemoryUnit) -> Result<(), KowalskiError> {
        info!("[EpisodicBuffer] Adding memory unit: {}", memory.id);
        debug!("Adding memory unit to episodic buffer: {}", memory.id);
        if memory.embedding.is_none() {
            match self.llm_provider.embed(&memory.content).await {
                Ok(embedding) => memory.embedding = Some(embedding),
                Err(e) => {
                    error!("Failed to get embedding for memory {}: {}", memory.id, e);
                }
            }
        }
        self.upsert_unit(&memory).await
    }

    async fn retrieve(
        &self,
        query: &str,
        retrieval_limit: usize,
    ) -> Result<Vec<MemoryUnit>, KowalskiError> {
        info!("[EpisodicBuffer][RETRIEVE] Query: '{}'", query);
        let query_embedding = self.llm_provider.embed(query).await.ok();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let max_time_span = 60 * 60 * 24 * 30u64;
        let units = self.load_all_units().await?;
        let mut scored = Vec::new();
        let mut fallback_results = Vec::new();
        for unit in units {
            if let (Some(q_emb), Some(m_emb)) = (query_embedding.as_ref(), unit.embedding.as_ref())
            {
                let sim = cosine_similarity(q_emb, m_emb);
                let recency = 1.0
                    - ((now.saturating_sub(unit.timestamp)) as f32 / max_time_span as f32).min(1.0);
                let recency = recency.max(0.0);
                let score = 0.85 * sim + 0.15 * recency;
                scored.push((score, unit));
            } else {
                let lower_query = query.to_lowercase().trim().to_string();
                let query_words: Vec<&str> = lower_query.split_whitespace().collect();
                let content = unit.content.to_lowercase();
                if query_words.iter().any(|w| content.contains(w)) {
                    fallback_results.push(unit);
                }
            }
        }
        if !scored.is_empty() {
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            return Ok(scored
                .into_iter()
                .map(|(_, u)| u)
                .take(retrieval_limit)
                .collect());
        }
        let results = if fallback_results.len() > retrieval_limit {
            fallback_results[fallback_results.len() - retrieval_limit..].to_vec()
        } else {
            fallback_results
        };
        Ok(results)
    }

    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryUnit>, KowalskiError> {
        debug!("Searching episodic buffer with query: {:?}", query);
        self.retrieve(&query.text_query, 3).await
    }
}
