// Tier 1: Working Memory (The Scratchpad)
// This is a simple, volatile, in-memory store for the agent's current context.

use crate::{MemoryProvider, MemoryUnit, MemoryQuery};
use async_trait::async_trait;
use log::{debug, info};

/// A simple, in-memory, volatile store for an agent's short-term working memory.
///
/// It holds a collection of `MemoryUnit`s up to a defined capacity.
/// When the capacity is exceeded, the oldest memory unit is discarded.
pub struct WorkingMemory {
    store: Vec<MemoryUnit>,
    capacity: usize,
}

impl WorkingMemory {
    /// Creates a new `WorkingMemory` instance with a specified capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The maximum number of `MemoryUnit`s to hold.
    pub fn new(capacity: usize) -> Self {
        info!("Initializing working memory with capacity: {}", capacity);
        Self { store: Vec::with_capacity(capacity), capacity }
    }

    /// Returns the current number of units in memory.
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Checks if the memory store is empty.
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }
}

#[async_trait]
impl MemoryProvider for WorkingMemory {
    /// Adds a `MemoryUnit` to the working memory.
    ///
    /// If the memory is at capacity, the oldest unit is removed to make space.
    async fn add(&mut self, memory: MemoryUnit) -> Result<(), String> {
        debug!("Adding memory unit to working memory: {}", memory.id);
        if self.store.len() == self.capacity {
            let removed = self.store.remove(0);
            debug!("Working memory at capacity. Removed oldest unit: {}", removed.id);
        }
        self.store.push(memory);
        Ok(())
    }

    /// Retrieves all memory units that contain the query string (case-insensitive).
    /// This is a simple text search.
    async fn retrieve(&self, query: &str) -> Result<Vec<MemoryUnit>, String> {
        debug!("Retrieving from working memory with query: '{}'", query);
        let lower_query = query.to_lowercase();
        let results = self.store
            .iter()
            .filter(|unit| unit.content.to_lowercase().contains(&lower_query))
            .cloned()
            .collect();
        Ok(results)
    }

    /// Performs a structured search, currently equivalent to `retrieve`.
    /// In a more advanced implementation, this could handle vector search if embeddings were stored.
    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryUnit>, String> {
        debug!("Searching working memory with query: {:?}", query);
        // For working memory, a simple text search is usually sufficient.
        self.retrieve(&query.text_query).await
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Helper to create a new memory unit for testing.
    fn create_test_unit(content: &str) -> MemoryUnit {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap().as_secs();
        MemoryUnit {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp,
            content: content.to_string(),
            embedding: None,
        }
    }

    #[tokio::test]
    async fn test_add_to_memory() {
        let mut memory = WorkingMemory::new(3);
        assert_eq!(memory.len(), 0);

        memory.add(create_test_unit("first message")).await.unwrap();
        assert_eq!(memory.len(), 1);
        assert_eq!(memory.store[0].content, "first message");
    }

    #[tokio::test]
    async fn test_memory_capacity() {
        let mut memory = WorkingMemory::new(2);
        memory.add(create_test_unit("one")).await.unwrap();
        memory.add(create_test_unit("two")).await.unwrap();
        assert_eq!(memory.len(), 2);

        // Add a third unit, which should push "one" out.
        memory.add(create_test_unit("three")).await.unwrap();
        assert_eq!(memory.len(), 2);
        assert_eq!(memory.store[0].content, "two");
        assert_eq!(memory.store[1].content, "three");
    }

    #[tokio::test]
    async fn test_retrieve_simple() {
        let mut memory = WorkingMemory::new(5);
        memory.add(create_test_unit("Hello world")).await.unwrap();
        memory.add(create_test_unit("Another message")).await.unwrap();
        memory.add(create_test_unit("HELLO again")).await.unwrap();

        let results = memory.retrieve("hello").await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().any(|m| m.content == "Hello world"));
        assert!(results.iter().any(|m| m.content == "HELLO again"));
    }

    #[tokio::test]
    async fn test_retrieve_no_results() {
        let mut memory = WorkingMemory::new(5);
        memory.add(create_test_unit("A test message")).await.unwrap();

        let results = memory.retrieve("xyz").await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_search_delegates_to_retrieve() {
        let mut memory = WorkingMemory::new(5);
        memory.add(create_test_unit("This is a search test")).await.unwrap();

        let query = MemoryQuery {
            text_query: "search".to_string(),
            vector_query: None,
            top_k: 5,
        };

        let results = memory.search(query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "This is a search test");
    }
}