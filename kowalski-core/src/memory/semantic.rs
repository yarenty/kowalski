// Tier 3: Long-Term Semantic Store (The Library)
// In-process vector similarity + simple in-memory relation edges (std only for the graph part).

use crate::{
    error::KowalskiError,
    memory::{MemoryProvider, MemoryQuery, MemoryUnit},
};
use async_trait::async_trait;
use log::{debug, info, warn};
use std::collections::HashMap;

/// Cosine similarity in \[−1, 1\]; returns 0 if lengths differ or norms are zero.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na * nb)
}

/// Long-term memory: **in-memory** embedding index (cosine search) plus a **lightweight relation map**
/// (`subject` → list of `(predicate, object)` triples). No extra crates for the relational layer—only `std::collections`.
///
/// When `memory.database_url` is **`postgres://…`**, use [`super::semantic_pg::PostgresSemanticStore`] instead (pgvector + SQL tables).
///
/// No network services required for this type. Embeddings are compared in-process; scale is limited by RAM.
pub struct SemanticStore {
    /// Memories that include an embedding vector (used for semantic search).
    embedded_entries: Vec<MemoryUnit>,
    /// Directed edges from each subject: `subject -> [(predicate, object), ...]`.
    relations: HashMap<String, Vec<(String, String)>>,
}

impl SemanticStore {
    /// Creates an empty semantic store (no external services).
    pub fn new() -> Self {
        info!("Initializing in-process semantic memory (vectors + relation map)");
        Self {
            embedded_entries: Vec::new(),
            relations: HashMap::new(),
        }
    }
}

impl Default for SemanticStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MemoryProvider for SemanticStore {
    async fn add(&mut self, memory: MemoryUnit) -> Result<(), KowalskiError> {
        debug!("Adding memory unit to semantic store: {}", memory.id);

        if let Some(embedding) = &memory.embedding {
            if !embedding.is_empty() {
                self.embedded_entries.push(MemoryUnit {
                    id: memory.id.clone(),
                    timestamp: memory.timestamp,
                    content: memory.content.clone(),
                    embedding: Some(embedding.clone()),
                });
                info!("Added memory unit {} to in-process vector index.", memory.id);
            }
        }

        if let Ok(relation) = serde_json::from_str::<HashMap<String, String>>(&memory.content) {
            if let (Some(subject), Some(predicate), Some(object)) = (
                relation.get("subject"),
                relation.get("predicate"),
                relation.get("object"),
            ) {
                self.relations
                    .entry(subject.clone())
                    .or_default()
                    .push((predicate.clone(), object.clone()));
                info!(
                    "Added relationship: {} -[{}]-> {}",
                    subject, predicate, object
                );
            }
        }

        Ok(())
    }

    async fn retrieve(
        &self,
        query: &str,
        _retrieval_limit: usize,
    ) -> Result<Vec<MemoryUnit>, KowalskiError> {
        warn!(
            "SemanticStore::retrieve filters by id prefix / substring; use search() for vector similarity."
        );
        let q = query.trim();
        let results: Vec<MemoryUnit> = self
            .embedded_entries
            .iter()
            .filter(|m| m.id == q || m.id.contains(q) || m.content.contains(q))
            .cloned()
            .collect();
        Ok(results)
    }

    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryUnit>, KowalskiError> {
        debug!("Searching semantic store with query: {:?}", query);
        let mut out: Vec<MemoryUnit> = Vec::new();

        if let Some(vector) = &query.vector_query {
            let mut scored: Vec<(f32, MemoryUnit)> = Vec::new();
            for m in &self.embedded_entries {
                let Some(emb) = &m.embedding else { continue };
                let score = cosine_similarity(vector, emb);
                scored.push((
                    score,
                    MemoryUnit {
                        id: m.id.clone(),
                        content: format!("{} (similarity {:.4})", m.content, score),
                        timestamp: m.timestamp,
                        embedding: None,
                    },
                ));
            }
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            scored.truncate(query.top_k.max(1));
            out.extend(scored.into_iter().map(|(_, u)| u));
        }

        if let Some(edges) = self.relations.get(&query.text_query) {
            for (predicate, target) in edges {
                info!(
                    "Found graph relationship: {} -[{}]-> {}",
                    query.text_query, predicate, target
                );
                out.push(MemoryUnit {
                    id: uuid::Uuid::new_v4().to_string(),
                    content: format!(
                        "Graph Relationship: {} {} {}",
                        query.text_query, predicate, target
                    ),
                    timestamp: 0,
                    embedding: None,
                });
            }
        }

        Ok(out)
    }
}
