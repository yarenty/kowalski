// Tier 2: Episodic Buffer (The Journal)
// A persistent, chronological log of recent conversations using RocksDB.

use crate::{MemoryProvider, MemoryUnit, MemoryQuery};
use async_trait::async_trait;
use log::{debug, info, error};
use rocksdb::{DB, Options, IteratorMode};
use tokio::sync::{OnceCell, Mutex};

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
pub async fn get_or_init_episodic_buffer(path: &str) -> Result<&'static Mutex<EpisodicBuffer>, String> {
    EPISODIC_BUFFER
        .get_or_try_init(|| async move { Ok(Mutex::new(EpisodicBuffer::new(path)?)) })
        .await
}

#[async_trait]
impl MemoryProvider for EpisodicBuffer {
    /// Adds a `MemoryUnit` to the RocksDB store.
    ///
    /// The unit is serialized to JSON. The `MemoryUnit.id` is used as the key.
    async fn add(&mut self, memory: MemoryUnit) -> Result<(), String> {
        info!("[EpisodicBuffer] Adding memory unit: {}", memory.id);
        debug!("Adding memory unit to episodic buffer: {}", memory.id);
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
    async fn retrieve(&self, query: &str, retrieval_limit: usize) -> Result<Vec<MemoryUnit>, String> {
        info!("[EpisodicBuffer][RETRIEVE] Query: '{}'", query);
        let iter = self.db.iterator(IteratorMode::Start);
        let lower_query = query.to_lowercase().trim().to_string();
        let query_words: Vec<&str> = lower_query.split_whitespace().collect();
        let mut results = Vec::new();
        for item in iter {
            match item {
                Ok((_key, value)) => {
                    match serde_json::from_slice::<MemoryUnit>(&value) {
                        Ok(unit) => {
                            info!("[EpisodicBuffer][RETRIEVE] Stored: '{}'", unit.content);
                            let content = unit.content.to_lowercase();
                            if query_words.iter().any(|w| content.contains(w)) {
                                results.push(unit);
                            }
                        }
                        Err(e) => error!("Failed to deserialize memory unit from DB: {}", e),
                    }
                }
                Err(e) => error!("Error during RocksDB iteration: {}", e),
            }
        }
        // Limit to retrieval_limit
        let results = if results.len() > retrieval_limit {
            results[results.len() - retrieval_limit..].to_vec()
        } else {
            results
        };
        Ok(results)
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
            .unwrap().as_secs();
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

        memory.add(create_test_unit("id1", "test one")).await.unwrap();
        memory.add(create_test_unit("id2", "another test")).await.unwrap();
        memory.add(create_test_unit("id3", "something else")).await.unwrap();

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