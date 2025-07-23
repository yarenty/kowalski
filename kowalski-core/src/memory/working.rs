// Tier 1: Working Memory (The Scratchpad)
// This is a simple, volatile, in-memory store for the agent's current context.

use crate::{
    error::KowalskiError,
    memory::{MemoryProvider, MemoryQuery, MemoryUnit},
};
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
        Self {
            store: Vec::with_capacity(capacity),
            capacity,
        }
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
    async fn add(&mut self, memory: MemoryUnit) -> Result<(), KowalskiError> {
        info!("[WorkingMemory] Adding memory unit: {}", memory.id);
        debug!("Adding memory unit to working memory: {}", memory.id);
        if self.store.len() == self.capacity {
            let removed = self.store.remove(0);
            debug!(
                "Working memory at capacity. Removed oldest unit: {}",
                removed.id
            );
        }
        self.store.push(memory);
        Ok(())
    }

    /// Retrieves all memory units that contain the query string (case-insensitive).
    /// This is a simple text search.
    async fn retrieve(
        &self,
        query: &str,
        retrieval_limit: usize,
    ) -> Result<Vec<MemoryUnit>, KowalskiError> {
        info!("[WorkingMemory][RETRIEVE] Query: '{}'", query);
        for unit in &self.store {
            info!("[WorkingMemory][RETRIEVE] Stored: '{}'", unit.content);
        }
        let lower_query = query.to_lowercase().trim().to_string();
        let query_words: Vec<&str> = lower_query.split_whitespace().collect();
        let mut results = Vec::new();
        for unit in &self.store {
            let content = unit.content.to_lowercase();
            if query_words.iter().any(|w| content.contains(w)) {
                results.push(unit.clone());
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
    /// In a more advanced implementation, this could handle vector search if embeddings were stored.
    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryUnit>, KowalskiError> {
        debug!("Searching working memory with query: {:?}", query);
        // For working memory, a simple text search is usually sufficient.
        self.retrieve(&query.text_query, query.top_k).await
    }
}
