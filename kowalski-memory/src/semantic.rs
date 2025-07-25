// Tier 3: Long-Term Semantic Store (The Library)
// Combines a Vector DB for semantic search and a Graph DB for relational knowledge.

use crate::{MemoryProvider, MemoryQuery, MemoryUnit};
use async_trait::async_trait;
use log::{debug, error, info, warn};
use petgraph::graph::{Graph, NodeIndex};
use petgraph::visit::EdgeRef;
use qdrant_client::Payload;
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{Condition, Filter, PointStruct};
use std::collections::HashMap;
use tokio::sync::Mutex;
use tokio::sync::OnceCell;
use serde_json::json;
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
}

impl SemanticStore {
    /// Creates a new `SemanticStore`.
    ///
    /// # Arguments
    ///
    /// * `qdrant_url` - The URL for the running Qdrant instance (e.g., "http://localhost:6333").
    pub async fn new(qdrant_url: &str) -> Result<Self, String> {
        info!("Initializing semantic memory with Qdrant at {}", qdrant_url);
        let vector_db = Qdrant::from_url(qdrant_url).build().map_err(|e| {
            error!("Failed to connect to Qdrant: {}", e);
            e.to_string()
        })?;

        // In a real application, you might want to ensure the collection exists.
        // For simplicity, we assume it's created beforehand.

        Ok(Self {
            vector_db,
            graph_db: Graph::new(),
            graph_nodes: HashMap::new(),
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
    async fn add(&mut self, memory: MemoryUnit) -> Result<(), String> {
        debug!("Adding memory unit to semantic store: {}", memory.id);

        // Add to vector store if an embedding exists
        if let Some(embedding) = &memory.embedding {
            let generated_uuid = Uuid::new_v4().to_string();
            // Store the original custom ID in the payload
            let mut payload_map = serde_json::Map::new();
            payload_map.insert("custom_id".to_string(), json!(memory.id.clone()));
            let payload: Payload = serde_json::Value::Object(payload_map).try_into().unwrap_or_else(|_| Payload::default());
            let point = PointStruct::new(
                generated_uuid,
                embedding.clone(),
                payload,
            );
            self.vector_db
                .upsert_points(qdrant_client::qdrant::UpsertPointsBuilder::new(
                    QDRANT_COLLECTION_NAME,
                    vec![point],
                ))
                .await
                .map_err(|e| e.to_string())?;
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
    ) -> Result<Vec<MemoryUnit>, String> {
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
            .map_err(|e| e.to_string())?;

        // This part is complex as we don't store the full MemoryUnit in Qdrant.
        // In a real system, you'd fetch the full unit from another store (like Tier 2)
        // using the retrieved IDs. For now, we return a simplified unit.
        let results = points
            .result
            .into_iter()
            .map(|p| {
                let vectors = p.vectors.unwrap();
                println!("DEBUG: vectors struct = {:?}", vectors);
                let custom_id = p.payload.get("custom_id")
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
    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryUnit>, String> {
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
                .map_err(|e| e.to_string())?;
            info!(
                "Found {} results from vector search.",
                search_result.result.len()
            );
            // Again, creating simplified MemoryUnits from results
            for point in search_result.result {
                let vectors = point.vectors.unwrap();
                println!("DEBUG: vectors struct = {:?}", vectors);
                let custom_id = point.payload.get("custom_id")
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

/// Get or initialize the singleton SemanticStore asynchronously.
pub async fn get_or_init_semantic_store(
    qdrant_url: &str,
) -> Result<&'static Mutex<SemanticStore>, String> {
    SEMANTIC_STORE
        .get_or_try_init(|| async { Ok(Mutex::new(SemanticStore::new(qdrant_url).await?)) })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Helper to create a test unit with a random embedding.
    fn create_vector_unit(id: &str, content: &str) -> MemoryUnit {
        MemoryUnit {
            id: id.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            content: content.to_string(),
            embedding: Some(vec![
                rand::random(),
                rand::random(),
                rand::random(),
                rand::random(),
            ]),
        }
    }

    // Helper to create a test unit representing a graph relationship.
    fn create_graph_unit(subject: &str, predicate: &str, object: &str) -> MemoryUnit {
        let content = serde_json::json!({
            "subject": subject,
            "predicate": predicate,
            "object": object
        })
        .to_string();
        MemoryUnit {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: 0,
            content,
            embedding: None,
        }
    }

    #[tokio::test]
    async fn test_add_and_query_graph() {
        let mut store = SemanticStore::new("http://localhost:6334").await.unwrap(); // Use different port to avoid collision

        store
            .add(create_graph_unit("Kowalski", "is_written_in", "Rust"))
            .await
            .unwrap();
        store
            .add(create_graph_unit(
                "Kowalski",
                "has_module",
                "kowalski-memory",
            ))
            .await
            .unwrap();

        let query = MemoryQuery {
            text_query: "Kowalski".to_string(),
            vector_query: None,
            top_k: 5,
        };

        let results = store.search(query).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(
            results
                .iter()
                .any(|m| m.content.contains("is_written_in Rust"))
        );
        assert!(
            results
                .iter()
                .any(|m| m.content.contains("has_module kowalski-memory"))
        );
    }

    /// NOTE: This is an integration test and requires a running Qdrant instance
    /// at localhost:6333 with a collection named "kowalski_memory" of vector size 4.
    #[tokio::test]
    #[ignore] // Ignore by default to not fail CI/CD pipelines.
    async fn test_add_and_search_vector() {
        let qdrant_url = "http://localhost:6333";
        let mut store = SemanticStore::new(qdrant_url).await.unwrap();

        let unit1 = create_vector_unit("vec1", "A message about cats");
        let unit2 = create_vector_unit("vec2", "A message about dogs");
        store.add(unit1.clone()).await.unwrap();
        store.add(unit2.clone()).await.unwrap();

        // Wait a moment for Qdrant to index the points.
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let query = MemoryQuery {
            text_query: "animal query".to_string(),
            vector_query: unit1.embedding, // Search for vectors similar to unit1
            top_k: 1,
        };

        let results = store.search(query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "vec1");
    }
}
