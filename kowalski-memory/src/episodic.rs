// Tier 2: Episodic Buffer (The Journal)
// A persistent, chronological log of recent conversations using RocksDB.

use crate::{MemoryProvider, MemoryQuery, MemoryUnit};
use async_trait::async_trait;
use log::{debug, error, info};
use rocksdb::{DB, IteratorMode, Options};
use tokio::sync::{Mutex, OnceCell};
use reqwest;
use serde_json;
use std::time::{SystemTime, UNIX_EPOCH};

/// A persistent, chronological memory store using RocksDB.
///
/// This acts as the agent's "journal," storing a high-fidelity log of past interactions.
/// Memory units are stored as key-value pairs, where the key is the `MemoryUnit.id`.
pub struct EpisodicBuffer {
    db: DB,
}

impl EpisodicBuffer {
    /// Creates a new or opens an existing `EpisodicBuffer` at the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - The filesystem path where the RocksDB database will be stored.
    pub fn new(path: &str) -> Result<Self, String> {
        info!("Initializing episodic buffer at path: {}", path);
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path).map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("No locks available") || err_str.contains("lock hold by current process") {
                error!("RocksDB lock error at {}: {}", path, err_str);
                format!(
                    "Failed to open RocksDB at {} due to a lock error: {}\n\
                    This usually means another process is using the database, or a previous process crashed and left a stale lock.\n\
                    - Ensure no other process is using the database.\n\
                    - If safe, remove the LOCK file in the database directory and try again.\n\
                    - If the problem persists, try rebooting your system.\n\
                    - For development, you can move or delete the entire database directory to start fresh.\n\
                    (Original error: {})",
                    path, err_str, err_str
                )
            } else {
                error!("Failed to open RocksDB at {}: {}", path, err_str);
                err_str
            }
        })?;
        Ok(Self { db })
    }
}

static EPISODIC_BUFFER: OnceCell<Mutex<EpisodicBuffer>> = OnceCell::const_new();

/// Get or initialize the singleton EpisodicBuffer asynchronously, wrapped in a Mutex for safe mutable access.
pub async fn get_or_init_episodic_buffer(
    path: &str,
) -> Result<&'static Mutex<EpisodicBuffer>, String> {
    EPISODIC_BUFFER
        .get_or_try_init(|| async move { Ok(Mutex::new(EpisodicBuffer::new(path)?)) })
        .await
}

/// Utility: Get Ollama embedding for a string
async fn get_ollama_embedding(text: &str) -> Result<Vec<f32>, String> {
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:11434/api/embeddings")
        .json(&serde_json::json!({
            "model": "llama3.2", // or "nomic-embed-text"
            "prompt": text
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to send request to Ollama: {}", e))?;

    let json: serde_json::Value = response.json().await.map_err(|e| format!("Failed to parse Ollama response: {}", e))?;
    let embedding = json["embedding"]
        .as_array()
        .ok_or("No embedding in response")?
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
        .collect();
    Ok(embedding)
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
    async fn add(&mut self, mut memory: MemoryUnit) -> Result<(), String> {
        info!("[EpisodicBuffer] Adding memory unit: {}", memory.id);
        debug!("Adding memory unit to episodic buffer: {}", memory.id);
        // If embedding is missing, generate it
        if memory.embedding.is_none() {
            match get_ollama_embedding(&memory.content).await {
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
            e.to_string()
        })?;
        self.db.put(key.as_bytes(), value.as_bytes()).map_err(|e| {
            error!("Failed to write to RocksDB for key {}: {}", key, e);
            e.to_string()
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
    ) -> Result<Vec<MemoryUnit>, String> {
        info!("[EpisodicBuffer][RETRIEVE] Query: '{}'", query);
        // Try to get embedding for the query
        let query_embedding = get_ollama_embedding(query).await.ok();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let max_time_span = 60 * 60 * 24 * 30; // 30 days in seconds, for recency normalization
        let iter = self.db.iterator(IteratorMode::Start);
        let mut scored = Vec::new();
        let mut fallback_results = Vec::new();
        for item in iter {
            match item {
                Ok((_key, value)) => match serde_json::from_slice::<MemoryUnit>(&value) {
                    Ok(unit) => {
                        // If both query and memory have embeddings, use semantic+recency
                        if let (Some(ref q_emb), Some(ref m_emb)) = (query_embedding.as_ref(), unit.embedding.as_ref()) {
                            let sim = cosine_similarity(q_emb, m_emb);
                            // Recency: newer memories get higher score
                            let recency = 1.0 - ((now.saturating_sub(unit.timestamp)) as f32 / max_time_span as f32);
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
            let results: Vec<MemoryUnit> = scored.into_iter().map(|(_, u)| u).take(retrieval_limit).collect();
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
    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryUnit>, String> {
        debug!("Searching episodic buffer with query: {:?}", query);
        self.retrieve(&query.text_query, 3).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tempfile::tempdir;

    // Helper to create a new memory unit for testing.
    fn create_test_unit(id: &str, content: &str) -> MemoryUnit {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        MemoryUnit {
            id: id.to_string(),
            timestamp,
            content: content.to_string(),
            embedding: None,
        }
    }

    #[tokio::test]
    async fn test_add_and_retrieve_one_item() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_str().unwrap();
        let mut memory = EpisodicBuffer::new(path).unwrap();

        let unit = create_test_unit("id1", "This is the first message.");
        memory.add(unit.clone()).await.unwrap();

        // Retrieve it back
        let results = memory.retrieve("first message", 3).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "id1");
    }

    #[tokio::test]
    async fn test_persistence() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_str().unwrap();

        // Create a DB and add an item
        {
            let mut memory = EpisodicBuffer::new(path).unwrap();
            let unit = create_test_unit("id2", "A persistent message.");
            memory.add(unit).await.unwrap();
        } // DB is closed here as `memory` goes out of scope

        // Re-open the DB and check if the item is still there
        {
            let memory = EpisodicBuffer::new(path).unwrap();
            let results = memory.retrieve("persistent", 3).await.unwrap();
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].id, "id2");
        }
    }

    #[tokio::test]
    async fn test_retrieve_multiple_items() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_str().unwrap();
        let mut memory = EpisodicBuffer::new(path).unwrap();

        memory
            .add(create_test_unit("id1", "test one"))
            .await
            .unwrap();
        memory
            .add(create_test_unit("id2", "another test"))
            .await
            .unwrap();
        memory
            .add(create_test_unit("id3", "something else"))
            .await
            .unwrap();

        let results = memory.retrieve("test", 3).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|m| m.id == "id1"));
        assert!(results.iter().any(|m| m.id == "id2"));
    }

    #[tokio::test]
    async fn test_retrieve_no_results() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_str().unwrap();
        let memory = EpisodicBuffer::new(path).unwrap();

        let results = memory.retrieve("nonexistent", 3).await.unwrap();
        assert!(results.is_empty());
    }
}
