// Tier 2: Episodic Buffer (The Journal)
// A persistent, chronological log of recent conversations using RocksDB.

use crate::{MemoryProvider, MemoryUnit, MemoryQuery};
use async_trait::async_trait;
use log::{debug, info, error};
use rocksdb::{DB, Options, IteratorMode};

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
            error!("Failed to open RocksDB at {}: {}", path, e);
            e.to_string()
        })?;
        Ok(Self { db })
    }
}

#[async_trait]
impl MemoryProvider for EpisodicBuffer {
    /// Adds a `MemoryUnit` to the RocksDB store.
    ///
    /// The unit is serialized to JSON. The `MemoryUnit.id` is used as the key.
    async fn add(&mut self, memory: MemoryUnit) -> Result<(), String> {
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
    async fn retrieve(&self, query: &str) -> Result<Vec<MemoryUnit>, String> {
        debug!("Retrieving from episodic buffer with query: '{}'", query);
        let lower_query = query.to_lowercase();
        let mut results = Vec::new();
        let iter = self.db.iterator(IteratorMode::Start);

        for item in iter {
            match item {
                Ok((_key, value)) => {
                    match serde_json::from_slice::<MemoryUnit>(&value) {
                        Ok(unit) => {
                            if unit.content.to_lowercase().contains(&lower_query) {
                                results.push(unit);
                            }
                        }
                        Err(e) => error!("Failed to deserialize memory unit from DB: {}", e),
                    }
                }
                Err(e) => error!("Error during RocksDB iteration: {}", e),
            }
        }
        Ok(results)
    }

    /// Performs a structured search, currently equivalent to `retrieve`.
    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryUnit>, String> {
        debug!("Searching episodic buffer with query: {:?}", query);
        self.retrieve(&query.text_query).await
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
        let results = memory.retrieve("first message").await.unwrap();
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
            let results = memory.retrieve("persistent").await.unwrap();
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

        let results = memory.retrieve("test").await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|m| m.id == "id1"));
        assert!(results.iter().any(|m| m.id == "id2"));
    }

    #[tokio::test]
    async fn test_retrieve_no_results() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_str().unwrap();
        let memory = EpisodicBuffer::new(path).unwrap();

        let results = memory.retrieve("nonexistent").await.unwrap();
        assert!(results.is_empty());
    }
}