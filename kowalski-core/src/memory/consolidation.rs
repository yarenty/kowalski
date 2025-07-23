use crate::{
    error::KowalskiError,
    memory::{MemoryProvider, MemoryUnit, episodic::EpisodicBuffer, semantic::SemanticStore},
};
use log::{debug, info};
use reqwest;
use serde_json;
use std::error::Error;

/// Trait for memory consolidation strategies ("Weavers")
#[async_trait::async_trait]
pub trait MemoryWeaver {
    async fn run(&mut self, delete_original: bool) -> Result<(), Box<dyn Error>>;
}

pub struct Consolidator {
    episodic_memory: EpisodicBuffer,
    semantic_memory: SemanticStore,
    ollama_host: String,
    ollama_port: u16,
    ollama_model: String,
}

impl Consolidator {
    pub async fn new(
        episodic_path: &str,
        qdrant_url: &str,
        ollama_host: &str,
        ollama_port: u16,
        ollama_model: &str,
    ) -> Result<Self, KowalskiError> {
        let episodic_memory =
            EpisodicBuffer::new(episodic_path, ollama_host, ollama_port, ollama_model)?;
        let semantic_memory = SemanticStore::new(qdrant_url).await?;
        Ok(Self {
            episodic_memory,
            semantic_memory,
            ollama_host: ollama_host.to_string(),
            ollama_port,
            ollama_model: ollama_model.to_string(),
        })
    }

    async fn get_ollama_embedding(&self, text: &str) -> Result<Vec<f32>, KowalskiError> {
        let client = reqwest::Client::new();
        let response = client
            .post(format!(
                "http://{}:{}/api/embeddings",
                self.ollama_host, self.ollama_port
            ))
            .json(&serde_json::json!({
                "model": self.ollama_model,
                "prompt": text
            }))
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        let embedding = json["embedding"]
            .as_array()
            .ok_or(KowalskiError::Memory(
                "No embedding in response".to_string(),
            ))?
            .iter()
            .map(|v| v.as_f64().unwrap_or(0.0) as f32)
            .collect();
        Ok(embedding)
    }

    async fn summarize_with_llm(&self, content: &str) -> Result<String, KowalskiError> {
        let client = reqwest::Client::new();
        let response = client
            .post(format!(
                "http://{}:{}/api/generate",
                self.ollama_host, self.ollama_port
            ))
            .json(&serde_json::json!({
                "model": self.ollama_model,
                "prompt": format!("Summarize the following text:\n\n{}", content),
                "stream": false
            }))
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        let summary = json["response"].as_str().unwrap_or("").to_string();
        Ok(summary)
    }

    async fn create_graph_with_llm(&self, content: &str) -> Result<String, KowalskiError> {
        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{}:{}/api/generate", self.ollama_host, self.ollama_port))
            .json(&serde_json::json!({
                "model": self.ollama_model,
                "prompt": format!("Create a graph representation of the following text in the format {{ \"subject\": \"...\", \"predicate\": \"...\", \"object\": \"...\" }}:\n\n{}", content),
                "stream": false
            }))
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        let graph = json["response"].as_str().unwrap_or("").to_string();
        Ok(graph)
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

            let summary_embedding = self.get_ollama_embedding(&summary).await.ok();
            let graph_embedding = self.get_ollama_embedding(&graph_representation).await.ok();

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
