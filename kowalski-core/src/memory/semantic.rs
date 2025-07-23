// Tier 3: Long-Term Semantic Store (The Library)
// Combines a Vector DB for semantic search and a Graph DB for relational knowledge.

use crate::{
    error::KowalskiError,
    memory::{MemoryProvider, MemoryQuery, MemoryUnit},
};
use async_trait::async_trait;
use log::{debug, error, info, warn};
use petgraph::graph::{Graph, NodeIndex};
use petgraph::visit::EdgeRef;
use qdrant_client::Payload;
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{Condition, Filter, PointStruct};
use serde_json::json;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::sync::OnceCell;
use uuid::Uuid;

const QDRANT_COLLECTION_NAME: &str = "kowalski_memory";

/// A persistent, long-term memory store combining semantic (vector) search
/// and relational (graph) search.
///
/// - **Vector Store (Qdrant):** Stores `MemoryUnit` embeddings for semantic similarity search.
/// - **Graph Store (petgraph):** Stores entities and their relationships for structured queries.
pub struct SemanticStore {
    vector_db: Qdrant,
    graph_db: Graph<String, String>,
    // A helper map to quickly find graph nodes by their string identifier
    graph_nodes: HashMap<String, NodeIndex>,
    #[allow(dead_code)]
    qdrant_url: String,
}

impl SemanticStore {
    /// Creates a new `SemanticStore`.
    ///
    /// # Arguments
    ///
    /// * `qdrant_url` - The URL for the running Qdrant instance (e.g., "http://localhost:6333").
    pub async fn new(qdrant_url: &str) -> Result<Self, KowalskiError> {
        info!("Initializing semantic memory with Qdrant at {}", qdrant_url);
        let vector_db = Qdrant::from_url(qdrant_url).build().map_err(|e| {
            error!("Failed to connect to Qdrant: {}", e);
            KowalskiError::Memory(e.to_string())
        })?;

        Ok(Self {
            vector_db,
            graph_db: Graph::new(),
            graph_nodes: HashMap::new(),
            qdrant_url: qdrant_url.to_string(),
        })
    }

    /// Adds a node to the graph if it doesn't already exist.
    fn get_or_add_node(&mut self, name: &str) -> NodeIndex {
        *self.graph_nodes.entry(name.to_string()).or_insert_with(|| {
            debug!("Adding new node '{}' to graph", name);
            self.graph_db.add_node(name.to_string())
        })
    }
}

#[async_trait]
impl MemoryProvider for SemanticStore {
    /// Adds a `MemoryUnit` to the semantic store.
    ///
    /// - If the unit has an embedding, it's added to the Qdrant vector DB.
    /// - If the unit's content is a JSON object representing a graph relationship
    ///   (e.g., `{"subject": "A", "predicate": "is_related_to", "object": "B"}`),
    ///   it's added to the `petgraph` graph.
    async fn add(&mut self, memory: MemoryUnit) -> Result<(), KowalskiError> {
        debug!("Adding memory unit to semantic store: {}", memory.id);

        // Add to vector store if an embedding exists
        if let Some(embedding) = &memory.embedding {
            let generated_uuid = Uuid::new_v4().to_string();
            // Store the original custom ID in the payload
            let mut payload_map = serde_json::Map::new();
            payload_map.insert("custom_id".to_string(), json!(memory.id.clone()));
            let payload: Payload = serde_json::Value::Object(payload_map)
                .try_into()
                .unwrap_or_else(|_| Payload::default());
            let point = PointStruct::new(generated_uuid, embedding.clone(), payload);
            self.vector_db
                .upsert_points(qdrant_client::qdrant::UpsertPointsBuilder::new(
                    QDRANT_COLLECTION_NAME,
                    vec![point],
                ))
                .await
                .map_err(|e| KowalskiError::Memory(e.to_string()))?;
            info!("Added memory unit {} to Qdrant collection.", memory.id);
        }

        // Add to graph store if content represents a relationship
        if let Ok(relation) = serde_json::from_str::<HashMap<String, String>>(&memory.content) {
            if let (Some(subject), Some(predicate), Some(object)) = (
                relation.get("subject"),
                relation.get("predicate"),
                relation.get("object"),
            ) {
                let subj_node = self.get_or_add_node(subject);
                let obj_node = self.get_or_add_node(object);
                self.graph_db
                    .add_edge(subj_node, obj_node, predicate.clone());
                info!(
                    "Added relationship to graph: {} -[{}]-> {}",
                    subject, predicate, object
                );
            }
        }

        Ok(())
    }

    /// `retrieve` is not the primary search method for this store. It performs a simple
    /// metadata search in Qdrant. Use `search` for semantic vector search.
    async fn retrieve(
        &self,
        query: &str,
        _retrieval_limit: usize,
    ) -> Result<Vec<MemoryUnit>, KowalskiError> {
        warn!(
            "Using retrieve on SemanticStore performs a simple metadata search, not a semantic vector search."
        );
        let filter = Filter::must([Condition::matches("custom_id", query.to_string())]);
        let points = self
            .vector_db
            .scroll(
                qdrant_client::qdrant::ScrollPointsBuilder::new(QDRANT_COLLECTION_NAME)
                    .filter(filter)
                    .with_payload(true),
            )
            .await
            .map_err(|e| KowalskiError::Memory(e.to_string()))?;

        // This part is complex as we don't store the full MemoryUnit in Qdrant.
        // In a real system, you'd fetch the full unit from another store (like Tier 2)
        // using the retrieved IDs. For now, we return a simplified unit.
        let results = points
            .result
            .into_iter()
            .map(|p| {
                let vectors = p.vectors.unwrap();
                println!("DEBUG: vectors struct = {:?}", vectors);
                let custom_id = p
                    .payload
                    .get("custom_id")
                    .and_then(|v| v.as_str())
                    .map_or(String::new(), |v| v.to_string());
                MemoryUnit {
                    id: custom_id,
                    content: "Retrieved from Qdrant by metadata filter".to_string(),
                    timestamp: 0,
                    embedding: None, // temporarily set to None for debugging
                }
            })
            .collect();

        Ok(results)
    }

    /// Performs a hybrid search across the vector and graph stores.
    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryUnit>, KowalskiError> {
        debug!("Searching semantic store with query: {:?}", query);
        let mut results = Vec::new();

        // 1. Vector Search
        if let Some(vector) = &query.vector_query {
            let search_points = qdrant_client::qdrant::SearchPointsBuilder::new(
                QDRANT_COLLECTION_NAME,
                vector.clone(),
                query.top_k as u64,
            )
            .with_payload(true);
            let search_result = self
                .vector_db
                .search_points(search_points)
                .await
                .map_err(|e| KowalskiError::Memory(e.to_string()))?;
            info!(
                "Found {} results from vector search.",
                search_result.result.len()
            );
            // Again, creating simplified MemoryUnits from results
            for point in search_result.result {
                let vectors = point.vectors.unwrap();
                println!("DEBUG: vectors struct = {:?}", vectors);
                let custom_id = point
                    .payload
                    .get("custom_id")
                    .and_then(|v| v.as_str())
                    .map_or(String::new(), |v| v.to_string());
                results.push(MemoryUnit {
                    id: custom_id,
                    content: format!("Retrieved from vector search (score: {})", point.score),
                    timestamp: 0,
                    embedding: None, // temporarily set to None for debugging
                });
            }
        }

        // 2. Graph Search (simple implementation)
        // A real implementation would use more advanced NLP to extract entities.
        if let Some(node_idx) = self.graph_nodes.get(&query.text_query) {
            for edge in self.graph_db.edges(*node_idx) {
                let target_node_idx = edge.target();
                let target_node_data = &self.graph_db[target_node_idx];
                let edge_data = edge.weight();
                info!(
                    "Found graph relationship: {} -[{}]-> {}",
                    query.text_query, edge_data, target_node_data
                );
                results.push(MemoryUnit {
                    id: uuid::Uuid::new_v4().to_string(),
                    content: format!(
                        "Graph Relationship: {} {} {}",
                        query.text_query, edge_data, target_node_data
                    ),
                    timestamp: 0,
                    embedding: None,
                });
            }
        }

        Ok(results)
    }
}

static SEMANTIC_STORE: OnceCell<Mutex<SemanticStore>> = OnceCell::const_new();

/// Get or initialize the singleton SemanticStore asynchronously, wrapped in a Mutex for safe mutable access.
pub async fn get_or_init_semantic_store(
    qdrant_url: &str,
) -> Result<&'static Mutex<SemanticStore>, KowalskiError> {
    SEMANTIC_STORE
        .get_or_try_init(|| async move { Ok(Mutex::new(SemanticStore::new(qdrant_url).await?)) })
        .await
}
