// Tier 1: Working Memory (The Scratchpad)
// This is a simple, volatile, in-memory store for the agent's current context.

use crate::{MemoryProvider, MemoryUnit, MemoryQuery};

pub struct WorkingMemory {
    store: Vec<MemoryUnit>,
    capacity: usize,
}

impl WorkingMemory {
    pub fn new(capacity: usize) -> Self {
        Self { store: Vec::with_capacity(capacity), capacity }
    }
}

#[async_trait::async_trait]
impl MemoryProvider for WorkingMemory {
    async fn add(&mut self, memory: MemoryUnit) -> Result<(), String> {
        // Implementation would add to the in-memory vector, handling capacity limits.
        unimplemented!()
    }

    async fn retrieve(&self, query: &str) -> Result<Vec<MemoryUnit>, String> {
        // Simple text search over the in-memory store.
        unimplemented!()
    }

    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryUnit>, String> {
        unimplemented!()
    }
}
