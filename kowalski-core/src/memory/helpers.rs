// Helper module for creating memory providers
// This is a temporary helper to make migration easier

use crate::config::Config;
use crate::error::KowalskiError;
use crate::memory::MemoryProvider;
use crate::memory::episodic::EpisodicBuffer;
use crate::memory::semantic::SemanticStore;
use crate::memory::working::WorkingMemory;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type MemoryProviderArc = Arc<Mutex<dyn MemoryProvider + Send + Sync>>;

/// Creates the standard set of memory providers from a config
pub async fn create_memory_providers(
    config: &Config,
) -> Result<(MemoryProviderArc, MemoryProviderArc, MemoryProviderArc), KowalskiError> {
    let working_memory = Arc::new(Mutex::new(WorkingMemory::new(100))) as MemoryProviderArc;

    let llm_provider = crate::llm::create_llm_provider(config)?;
    let episodic_memory = Arc::new(Mutex::new(
        EpisodicBuffer::open(&config.memory, llm_provider).await?,
    )) as MemoryProviderArc;

    let semantic_memory =
        Arc::new(Mutex::new(SemanticStore::new())) as MemoryProviderArc;

    Ok((working_memory, episodic_memory, semantic_memory))
}
