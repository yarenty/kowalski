use crate::{
    config::{memory_uses_postgres, MemoryConfig},
    error::KowalskiError,
    memory::{
        MemoryProvider, MemoryUnit, episodic::EpisodicBuffer, semantic::SemanticStore,
    },
};
#[cfg(feature = "postgres")]
use crate::memory::semantic_pg::PostgresSemanticStore;
#[cfg(feature = "postgres")]
use sqlx::postgres::PgPool;
use log::{debug, info};
use std::error::Error;

/// Trait for memory consolidation strategies ("Weavers")
#[async_trait::async_trait]
pub trait MemoryWeaver {
    async fn run(&mut self, delete_original: bool) -> Result<(), Box<dyn Error>>;
}

pub struct Consolidator {
    episodic_memory: EpisodicBuffer,
    semantic_memory: Box<dyn MemoryProvider + Send + Sync>,
    llm_provider: std::sync::Arc<dyn crate::llm::LLMProvider>,
    model: String,
}

impl Consolidator {
    pub async fn new(
        memory: &MemoryConfig,
        llm_provider: std::sync::Arc<dyn crate::llm::LLMProvider>,
        model: &str,
    ) -> Result<Self, KowalskiError> {
        let episodic_memory = EpisodicBuffer::open(memory, llm_provider.clone()).await?;
        let semantic_memory: Box<dyn MemoryProvider + Send + Sync> =
            if memory_uses_postgres(memory) {
                #[cfg(feature = "postgres")]
                {
                    let url = memory
                        .database_url
                        .as_ref()
                        .expect("memory_uses_postgres implies database_url");
                    let pool = PgPool::connect(url.as_str())
                        .await
                        .map_err(|e| {
                            KowalskiError::Memory(format!("consolidator semantic Postgres: {e}"))
                        })?;
                    Box::new(PostgresSemanticStore::new(
                        pool,
                        llm_provider.clone(),
                        memory.embedding_vector_dimensions,
                    ))
                }
                #[cfg(not(feature = "postgres"))]
                {
                    return Err(crate::config::postgres_feature_required_error());
                }
            } else {
                Box::new(SemanticStore::new())
            };
        Ok(Self {
            episodic_memory,
            semantic_memory,
            llm_provider,
            model: model.to_string(),
        })
    }

    async fn summarize_with_llm(&self, content: &str) -> Result<String, KowalskiError> {
        let prompt = format!("Summarize the following text:\n\n{}", content);
        let messages = vec![crate::conversation::Message {
            role: "user".to_string(),
            content: prompt,
            tool_calls: None,
        }];
        self.llm_provider.chat(&self.model, &messages).await
    }

    async fn create_graph_with_llm(&self, content: &str) -> Result<String, KowalskiError> {
        let prompt = format!(
            "Create a graph representation of the following text in the format {{ \"subject\": \"...\", \"predicate\": \"...\", \"object\": \"...\" }}:\n\n{}",
            content
        );
        let messages = vec![crate::conversation::Message {
            role: "user".to_string(),
            content: prompt,
            tool_calls: None,
        }];
        self.llm_provider.chat(&self.model, &messages).await
    }
}

#[async_trait::async_trait]
impl MemoryWeaver for Consolidator {
    async fn run(&mut self, delete_original: bool) -> Result<(), Box<dyn Error>> {
        info!("Starting memory consolidation process...");

        let memories_to_process = self.episodic_memory.retrieve_all().await?;

        for memory in memories_to_process {
            info!("Processing memory: {}", memory.id);

            // LLM call to generate summary and graph
            let summary = self.summarize_with_llm(&memory.content).await?;
            let graph_representation = self.create_graph_with_llm(&memory.content).await?;

            debug!("Generated Summary: {}", summary);
            debug!("Generated Graph: {}", graph_representation);

            let summary_embedding = self.llm_provider.embed(&summary).await.ok();
            let graph_embedding = self.llm_provider.embed(&graph_representation).await.ok();

            // Create new memory units for the summary and graph
            let summary_memory = MemoryUnit {
                id: format!("{}-summary", memory.id),
                timestamp: memory.timestamp,
                content: summary,
                embedding: summary_embedding,
            };

            let graph_memory = MemoryUnit {
                id: format!("{}-graph", memory.id),
                timestamp: memory.timestamp,
                content: graph_representation,
                embedding: graph_embedding,
            };

            // Add the new memories to the semantic store
            self.semantic_memory.add(summary_memory).await?;
            self.semantic_memory.add(graph_memory).await?;

            info!("Successfully processed and stored memory: {}", memory.id);

            // Optionally, delete the original memory from the episodic store
            if delete_original {
                self.episodic_memory.delete(&memory.id).await?;
                info!("Deleted original memory: {}", memory.id);
            }
        }

        info!("Memory consolidation process finished.");
        Ok(())
    }
}
