pub mod working;
pub mod episodic;
pub mod semantic;

use serde::{Deserialize, Serialize};

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
#[async_trait::async_trait]
pub trait MemoryProvider {
    /// Adds a memory unit to the store.
    async fn add(&self, memory: MemoryUnit) -> Result<(), String>;

    /// Retrieves a set of memories based on a query.
    /// The query could be a simple string or a more complex query structure.
    async fn retrieve(&self, query: &str) -> Result<Vec<MemoryUnit>, String>;

    /// A more advanced retrieval method using a structured query.
    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryUnit>, String>;
}

/// A structured query for more advanced memory retrieval.
#[derive(Debug)]
pub struct MemoryQuery {
    pub text_query: String,
    pub vector_query: Option<Vec<f32>>,
    pub top_k: usize,
}
