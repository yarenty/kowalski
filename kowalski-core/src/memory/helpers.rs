// Helper module for creating memory providers
// This is a temporary helper to make migration easier

use crate::config::{memory_uses_postgres, Config};
use crate::error::KowalskiError;
use crate::memory::MemoryProvider;
use crate::memory::episodic::EpisodicBuffer;
#[cfg(feature = "postgres")]
use crate::memory::semantic_pg::PostgresSemanticStore;
use crate::memory::semantic::SemanticStore;
use crate::memory::working::WorkingMemory;
#[cfg(feature = "postgres")]
use sqlx::postgres::PgPool;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type MemoryProviderArc = Arc<Mutex<dyn MemoryProvider + Send + Sync>>;

/// Tier 3 semantic memory: in-process vectors by default, or **PostgreSQL + pgvector** when `memory.database_url` is `postgres://…` and the **`postgres`** feature is enabled.
pub async fn create_semantic_memory(
    config: &Config,
    llm: Arc<dyn crate::llm::LLMProvider>,
) -> Result<MemoryProviderArc, KowalskiError> {
    if memory_uses_postgres(&config.memory) {
        #[cfg(feature = "postgres")]
        {
            let url = config
                .memory
                .database_url
                .as_ref()
                .expect("memory_uses_postgres implies database_url");
            let pool = PgPool::connect(url.as_str())
                .await
                .map_err(|e| KowalskiError::Memory(format!("semantic Postgres pool: {e}")))?;
            return Ok(Arc::new(Mutex::new(PostgresSemanticStore::new(
                pool,
                llm,
                config.memory.embedding_vector_dimensions,
            ))));
        }
        #[cfg(not(feature = "postgres"))]
        {
            drop(llm);
            return Err(crate::config::postgres_feature_required_error());
        }
    }
    drop(llm);
    Ok(Arc::new(Mutex::new(SemanticStore::new())))
}

/// Creates the standard set of memory providers from a config
pub async fn create_memory_providers(
    config: &Config,
) -> Result<(MemoryProviderArc, MemoryProviderArc, MemoryProviderArc), KowalskiError> {
    let working_memory = Arc::new(Mutex::new(WorkingMemory::new(100))) as MemoryProviderArc;

    let llm_provider = crate::llm::create_llm_provider(config)?;
    let episodic_memory = Arc::new(Mutex::new(
        EpisodicBuffer::open(&config.memory, llm_provider.clone()).await?,
    )) as MemoryProviderArc;

    let semantic_memory = create_semantic_memory(config, llm_provider).await?;

    Ok((working_memory, episodic_memory, semantic_memory))
}
