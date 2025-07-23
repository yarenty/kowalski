// Tier 2: Episodic Buffer (The Journal)
// A persistent, chronological log of recent conversations using RocksDB.

use crate::{
    error::KowalskiError,
    memory::{MemoryProvider, MemoryQuery, MemoryUnit},
};
use async_trait::async_trait;
use log::{debug, error, info};
use reqwest;
use rocksdb::{DB, IteratorMode, Options};
use serde_json;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, OnceCell};

/// A persistent, chronological memory store using RocksDB.
///
/// This acts as the agent's "journal," storing a high-fidelity log of past interactions.
/// Memory units are stored as key-value pairs, where the key is the `MemoryUnit.id`.
pub struct EpisodicBuffer {
    db: DB,
    ollama_host: String,
    ollama_port: u16,
    ollama_model: String,
}

impl EpisodicBuffer {
    /// Creates a new or opens an existing `EpisodicBuffer` at the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - The filesystem path where the RocksDB database will be stored.
    /// * `ollama_host`, `ollama_port`, `ollama_model` - Ollama connection details for embeddings
    pub fn new(
        path: &str,
        ollama_host: &str,
        ollama_port: u16,
        ollama_model: &str,
    ) -> Result<Self, KowalskiError> {
        info!("Initializing episodic buffer at path: {}", path);
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path).map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("No locks available") || err_str.contains("lock hold by current process") {
                error!("RocksDB lock error at {}: {}", path, err_str);
                KowalskiError::Memory(format!(
                    "Failed to open RocksDB at {} due to a lock error: {}\n\
                    This usually means another process is using the database, or a previous process crashed and left a stale lock.\n\
                    - Ensure no other process is using the database.\n\
                    - If safe, remove the LOCK file in the database directory and try again.\n\
                    - If the problem persists, try rebooting your system.\n\
                    - For development, you can move or delete the entire database directory to start fresh.\n\
                    (Original error: {})",
                    path, err_str, err_str
                ))
            } else {
                error!("Failed to open RocksDB at {}: {}", path, err_str);
                KowalskiError::Memory(err_str)
            }
        })?;
        Ok(Self {
            db,
            ollama_host: ollama_host.to_string(),
            ollama_port,
            ollama_model: ollama_model.to_string(),
        })
    }

    pub async fn retrieve_all(&self) -> Result<Vec<MemoryUnit>, KowalskiError> {
        let mut memories = Vec::new();
        let iter = self.db.iterator(IteratorMode::Start);
        for item in iter {
            match item {
                Ok((_key, value)) => match serde_json::from_slice::<MemoryUnit>(&value) {
                    Ok(unit) => memories.push(unit),
                    Err(e) => error!("Failed to deserialize memory unit from DB: {}", e),
                },
                Err(e) => error!("Error during RocksDB iteration: {}", e),
            }
        }
        Ok(memories)
    }

    pub async fn delete(&mut self, id: &str) -> Result<(), KowalskiError> {
        self.db
            .delete(id.as_bytes())
            .map_err(|e| KowalskiError::Memory(e.to_string()))
    }

    async fn get_ollama_embedding(&self, text: &str) -> Result<Vec<f32>, KowalskiError> {
        let client = reqwest::Client::new();
        let response = client
            .post(format!(
                "http://{}:{}/api/embeddings",
                self.ollama_host, self.ollama_port
            ))
            .json(&serde_json::json!({
                "model": self.ollama_model,
                "prompt": text
            }))
            .send()
            .await
            .map_err(|e| {
                KowalskiError::Memory(format!("Failed to send request to Ollama: {}", e))
            })?;

        let json: serde_json::Value = response.json().await.map_err(|e| {
            KowalskiError::Memory(format!("Failed to parse Ollama response: {}", e))
        })?;
        let embedding = json["embedding"]
            .as_array()
            .ok_or(KowalskiError::Memory(
                "No embedding in response".to_string(),
            ))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();
        Ok(embedding)
    }

    pub async fn add_with_embedding(
        &mut self,
        mut memory: MemoryUnit,
    ) -> Result<(), KowalskiError> {
        info!("[EpisodicBuffer] Adding memory unit: {}", memory.id);
        debug!("Adding memory unit to episodic buffer: {}", memory.id);
        // If embedding is missing, generate it
        if memory.embedding.is_none() {
            match self.get_ollama_embedding(&memory.content).await {
                Ok(embedding) => memory.embedding = Some(embedding),
                Err(e) => {
                    error!("Failed to get embedding for memory {}: {}", memory.id, e);
                    // Continue without embedding
                }
            }
        }
        let key = memory.id.clone();
        let value = serde_json::to_string(&memory).map_err(|e| {
            error!("Failed to serialize memory unit {}: {}", key, e);
            KowalskiError::Memory(e.to_string())
        })?;
        self.db.put(key.as_bytes(), value.as_bytes()).map_err(|e| {
            error!("Failed to write to RocksDB for key {}: {}", key, e);
            KowalskiError::Memory(e.to_string())
        })
    }

    pub async fn retrieve_with_embedding(
        &self,
        query: &str,
        retrieval_limit: usize,
    ) -> Result<Vec<MemoryUnit>, KowalskiError> {
        info!("[EpisodicBuffer][RETRIEVE] Query: '{}'", query);
        // Try to get embedding for the query
        let query_embedding = self.get_ollama_embedding(query).await.ok();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let max_time_span = 60 * 60 * 24 * 30; // 30 days in seconds, for recency normalization
        let iter = self.db.iterator(IteratorMode::Start);
        let mut scored = Vec::new();
        let mut fallback_results = Vec::new();
        for item in iter {
            match item {
                Ok((_key, value)) => match serde_json::from_slice::<MemoryUnit>(&value) {
                    Ok(unit) => {
                        // If both query and memory have embeddings, use semantic+recency
                        if let (Some(q_emb), Some(m_emb)) =
                            (query_embedding.as_ref(), unit.embedding.as_ref())
                        {
                            let sim = cosine_similarity(q_emb, m_emb);
                            // Recency: newer memories get higher score
                            let recency = 1.0
                                - ((now.saturating_sub(unit.timestamp)) as f32
                                    / max_time_span as f32);
                            let recency = recency.max(0.0); // Clamp to [0,1]
                            let score = 0.85 * sim + 0.15 * recency;
                            scored.push((score, unit));
                        } else {
                            // Fallback: text search (case-insensitive, any word)
                            let lower_query = query.to_lowercase().trim().to_string();
                            let query_words: Vec<&str> = lower_query.split_whitespace().collect();
                            let content = unit.content.to_lowercase();
                            if query_words.iter().any(|w| content.contains(w)) {
                                fallback_results.push(unit);
                            }
                        }
                    }
                    Err(e) => error!("Failed to deserialize memory unit from DB: {}", e),
                },
                Err(e) => error!("Error during RocksDB iteration: {}", e),
            }
        }
        // If we have scored results, sort and return top N
        if !scored.is_empty() {
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            let results: Vec<MemoryUnit> = scored
                .into_iter()
                .map(|(_, u)| u)
                .take(retrieval_limit)
                .collect();
            Ok(results)
        } else {
            // Fallback: return text search results (limit)
            let results = if fallback_results.len() > retrieval_limit {
                fallback_results.into_iter().take(retrieval_limit).collect()
            } else {
                fallback_results
            };
            Ok(results)
        }
    }
}

static EPISODIC_BUFFER: OnceCell<Mutex<EpisodicBuffer>> = OnceCell::const_new();

/// Get or initialize the singleton EpisodicBuffer asynchronously, wrapped in a Mutex for safe mutable access.
pub async fn get_or_init_episodic_buffer(
    path: &str,
    ollama_host: &str,
    ollama_port: u16,
    ollama_model: &str,
) -> Result<&'static Mutex<EpisodicBuffer>, KowalskiError> {
    EPISODIC_BUFFER
        .get_or_try_init(|| async move {
            Ok(Mutex::new(EpisodicBuffer::new(
                path,
                ollama_host,
                ollama_port,
                ollama_model,
            )?))
        })
        .await
}

/// Utility: Cosine similarity between two vectors
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
    /// Adds a `MemoryUnit` to the RocksDB store.
    ///
    /// The unit is serialized to JSON. The `MemoryUnit.id` is used as the key.
    async fn add(&mut self, mut memory: MemoryUnit) -> Result<(), KowalskiError> {
        info!("[EpisodicBuffer] Adding memory unit: {}", memory.id);
        debug!("Adding memory unit to episodic buffer: {}", memory.id);
        // If embedding is missing, generate it
        if memory.embedding.is_none() {
            match self.get_ollama_embedding(&memory.content).await {
                Ok(embedding) => memory.embedding = Some(embedding),
                Err(e) => {
                    error!("Failed to get embedding for memory {}: {}", memory.id, e);
                    // Continue without embedding
                }
            }
        }
        let key = memory.id.clone();
        let value = serde_json::to_string(&memory).map_err(|e| {
            error!("Failed to serialize memory unit {}: {}", key, e);
            KowalskiError::Memory(e.to_string())
        })?;
        self.db.put(key.as_bytes(), value.as_bytes()).map_err(|e| {
            error!("Failed to write to RocksDB for key {}: {}", key, e);
            KowalskiError::Memory(e.to_string())
        })
    }

    /// Retrieves all memory units that contain the query string (case-insensitive).
    ///
    /// This performs a full scan of the database, which can be slow on large datasets.
    /// It is intended for retrieving recent, related conversational context, not for
    /// large-scale semantic search.
    async fn retrieve(
        &self,
        query: &str,
        retrieval_limit: usize,
    ) -> Result<Vec<MemoryUnit>, KowalskiError> {
        info!("[EpisodicBuffer][RETRIEVE] Query: '{}'", query);
        // Try to get embedding for the query
        let query_embedding = self.get_ollama_embedding(query).await.ok();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let max_time_span = 60 * 60 * 24 * 30; // 30 days in seconds, for recency normalization
        let iter = self.db.iterator(IteratorMode::Start);
        let mut scored = Vec::new();
        let mut fallback_results = Vec::new();
        for item in iter {
            match item {
                Ok((_key, value)) => match serde_json::from_slice::<MemoryUnit>(&value) {
                    Ok(unit) => {
                        // If both query and memory have embeddings, use semantic+recency
                        if let (Some(q_emb), Some(m_emb)) =
                            (query_embedding.as_ref(), unit.embedding.as_ref())
                        {
                            let sim = cosine_similarity(q_emb, m_emb);
                            // Recency: newer memories get higher score
                            let recency = 1.0
                                - ((now.saturating_sub(unit.timestamp)) as f32
                                    / max_time_span as f32);
                            let recency = recency.max(0.0); // Clamp to [0,1]
                            let score = 0.85 * sim + 0.15 * recency;
                            scored.push((score, unit));
                        } else {
                            // Fallback: text search (case-insensitive, any word)
                            let lower_query = query.to_lowercase().trim().to_string();
                            let query_words: Vec<&str> = lower_query.split_whitespace().collect();
                            let content = unit.content.to_lowercase();
                            if query_words.iter().any(|w| content.contains(w)) {
                                fallback_results.push(unit);
                            }
                        }
                    }
                    Err(e) => error!("Failed to deserialize memory unit from DB: {}", e),
                },
                Err(e) => error!("Error during RocksDB iteration: {}", e),
            }
        }
        // If we have scored results, sort and return top N
        if !scored.is_empty() {
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            let results: Vec<MemoryUnit> = scored
                .into_iter()
                .map(|(_, u)| u)
                .take(retrieval_limit)
                .collect();
            Ok(results)
        } else {
            // Fallback: return text search results (limit)
            let results = if fallback_results.len() > retrieval_limit {
                fallback_results[fallback_results.len() - retrieval_limit..].to_vec()
            } else {
                fallback_results
            };
            Ok(results)
        }
    }

    /// Performs a structured search, currently equivalent to `retrieve`.
    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryUnit>, KowalskiError> {
        debug!("Searching episodic buffer with query: {:?}", query);
        self.retrieve(&query.text_query, 3).await
    }
}
