pub mod consolidation;
pub mod episodic;
pub mod semantic;
pub mod working;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::KowalskiError;

/// Represents a single unit of memory, which could be a message, a fact, or a summary.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MemoryUnit {
    pub id: String, // Unique identifier for this memory unit
    pub timestamp: u64,
    pub content: String,
    pub embedding: Option<Vec<f32>>,
}

/// The core trait for any memory system in Kowalski.
/// Defines the essential operations for storing and retrieving memories.
#[async_trait]
pub trait MemoryProvider {
    /// Adds a memory unit to the store.
    async fn add(&mut self, memory: MemoryUnit) -> Result<(), KowalskiError>;

    /// Retrieves a set of memories based on a query, limited to retrieval_limit.
    async fn retrieve(
        &self,
        query: &str,
        retrieval_limit: usize,
    ) -> Result<Vec<MemoryUnit>, KowalskiError>;

    /// A more advanced retrieval method using a structured query.
    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryUnit>, KowalskiError>;
}

/// A structured query for more advanced memory retrieval.
#[derive(Debug, Clone)]
pub struct MemoryQuery {
    pub text_query: String,
    pub vector_query: Option<Vec<f32>>,
    pub top_k: usize,
}
