// Tier 3: Semantic memory backed by PostgreSQL + pgvector (when `memory.database_url` is postgres).

use crate::error::KowalskiError;
use crate::llm::LLMProvider;
use crate::memory::{MemoryProvider, MemoryQuery, MemoryUnit};
use async_trait::async_trait;
use log::{debug, info, warn};
use pgvector::Vector;
use sqlx::Row;
use sqlx::postgres::PgPool;
use std::collections::HashMap;
use std::sync::Arc;

/// Semantic store using **`semantic_memory`** and **`semantic_relation`** tables (see `migrations/postgres/003_semantic_memory.sql`).
///
/// [`MemoryProvider::retrieve`] embeds the query via [`LLMProvider::embed`] and runs **cosine-distance** ordering (`<=>`).
pub struct PostgresSemanticStore {
    pool: PgPool,
    llm: Arc<dyn LLMProvider>,
    embedding_dims: usize,
}

impl PostgresSemanticStore {
    pub fn new(pool: PgPool, llm: Arc<dyn LLMProvider>, embedding_dims: usize) -> Self {
        info!(
            "Initializing PostgreSQL semantic memory (pgvector dim={})",
            embedding_dims
        );
        Self {
            pool,
            llm,
            embedding_dims,
        }
    }

    fn expect_embedding_vec(&self, embedding: &[f32], context: &str) -> Result<(), KowalskiError> {
        if embedding.len() != self.embedding_dims {
            return Err(KowalskiError::Memory(format!(
                "{context}: embedding length {} does not match memory.embedding_vector_dimensions ({})",
                embedding.len(),
                self.embedding_dims
            )));
        }
        Ok(())
    }

    async fn insert_relation_triple(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
    ) -> Result<(), KowalskiError> {
        sqlx::query(
            r#"INSERT INTO semantic_relation (subject, predicate, object)
               VALUES ($1, $2, $3)
               ON CONFLICT (subject, predicate, object) DO NOTHING"#,
        )
        .bind(subject)
        .bind(predicate)
        .bind(object)
        .execute(&self.pool)
        .await
        .map_err(|e| KowalskiError::Memory(format!("semantic_relation insert: {e}")))?;
        Ok(())
    }

    async fn try_parse_and_store_relations(
        &self,
        memory: &MemoryUnit,
    ) -> Result<(), KowalskiError> {
        if let Ok(relation) = serde_json::from_str::<HashMap<String, String>>(&memory.content)
            && let (Some(subject), Some(predicate), Some(object)) = (
                relation.get("subject"),
                relation.get("predicate"),
                relation.get("object"),
            )
        {
            self.insert_relation_triple(subject, predicate, object)
                .await?;
            info!(
                "Added relationship: {} -[{}]-> {}",
                subject, predicate, object
            );
        }
        Ok(())
    }
}

#[async_trait]
impl MemoryProvider for PostgresSemanticStore {
    async fn add(&mut self, memory: MemoryUnit) -> Result<(), KowalskiError> {
        debug!(
            "Adding memory unit to PostgreSQL semantic store: {}",
            memory.id
        );

        if let Some(ref emb) = memory.embedding
            && !emb.is_empty()
        {
            self.expect_embedding_vec(emb, "semantic add")?;
            let v = Vector::from(emb.to_vec());
            sqlx::query(
                r#"INSERT INTO semantic_memory (id, content_text, embedding)
                       VALUES ($1, $2, $3)
                       ON CONFLICT (id) DO UPDATE SET
                         content_text = EXCLUDED.content_text,
                         embedding = EXCLUDED.embedding,
                         created_at = NOW()"#,
            )
            .bind(&memory.id)
            .bind(&memory.content)
            .bind(v)
            .execute(&self.pool)
            .await
            .map_err(|e| KowalskiError::Memory(format!("semantic_memory insert: {e}")))?;
            info!("Stored semantic row {} (vector)", memory.id);
        }

        self.try_parse_and_store_relations(&memory).await?;
        Ok(())
    }

    async fn retrieve(
        &self,
        query: &str,
        retrieval_limit: usize,
    ) -> Result<Vec<MemoryUnit>, KowalskiError> {
        let q = query.trim();
        if q.is_empty() || retrieval_limit == 0 {
            return Ok(vec![]);
        }

        // Primary path: embed query and run pgvector similarity search (matches chat history injection).
        match self.llm.embed(q).await {
            Ok(query_emb) if query_emb.len() == self.embedding_dims => {
                let v = Vector::from(query_emb);
                let rows = sqlx::query(
                    r#"SELECT id, content_text,
                              EXTRACT(EPOCH FROM created_at)::bigint AS ts,
                              (embedding <=> $1) AS dist
                       FROM semantic_memory
                       ORDER BY embedding <=> $1
                       LIMIT $2"#,
                )
                .bind(v)
                .bind(retrieval_limit as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| KowalskiError::Memory(format!("semantic vector retrieve: {e}")))?;

                let mut out = Vec::with_capacity(rows.len());
                for row in rows {
                    let id: String = row
                        .try_get("id")
                        .map_err(|e| KowalskiError::Memory(format!("semantic row decode: {e}")))?;
                    let content_text: String = row
                        .try_get("content_text")
                        .map_err(|e| KowalskiError::Memory(format!("semantic row decode: {e}")))?;
                    let ts: i64 = row
                        .try_get("ts")
                        .map_err(|e| KowalskiError::Memory(format!("semantic row decode: {e}")))?;
                    let dist: f32 = row
                        .try_get("dist")
                        .map_err(|e| KowalskiError::Memory(format!("semantic row decode: {e}")))?;
                    let score = (1.0_f32 - dist).clamp(-1.0, 1.0);
                    out.push(MemoryUnit {
                        id,
                        content: format!("{} (similarity {:.4})", content_text, score),
                        timestamp: ts.max(0) as u64,
                        embedding: None,
                    });
                }
                if !out.is_empty() {
                    return Ok(out);
                }
            }
            Ok(wrong) => {
                warn!(
                    "Semantic retrieve: embedding length {} != {}; falling back to text search",
                    wrong.len(),
                    self.embedding_dims
                );
            }
            Err(e) => {
                warn!(
                    "Semantic retrieve: embed failed ({}); falling back to text search",
                    e
                );
            }
        }

        let pattern = format!("%{q}%");
        let rows = sqlx::query(
            r#"SELECT id, content_text, EXTRACT(EPOCH FROM created_at)::bigint AS ts
               FROM semantic_memory
               WHERE id ILIKE $1 OR content_text ILIKE $1
               ORDER BY created_at DESC
               LIMIT $2"#,
        )
        .bind(&pattern)
        .bind(retrieval_limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| KowalskiError::Memory(format!("semantic text retrieve: {e}")))?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let id: String = row
                .try_get("id")
                .map_err(|e| KowalskiError::Memory(format!("semantic row decode: {e}")))?;
            let content_text: String = row
                .try_get("content_text")
                .map_err(|e| KowalskiError::Memory(format!("semantic row decode: {e}")))?;
            let ts: i64 = row
                .try_get("ts")
                .map_err(|e| KowalskiError::Memory(format!("semantic row decode: {e}")))?;
            out.push(MemoryUnit {
                id,
                content: content_text,
                timestamp: ts.max(0) as u64,
                embedding: None,
            });
        }
        Ok(out)
    }

    async fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryUnit>, KowalskiError> {
        debug!("PostgreSQL semantic search: {:?}", query);
        let mut out: Vec<MemoryUnit> = Vec::new();

        if let Some(vector) = query.vector_query {
            if vector.len() == self.embedding_dims {
                let v = Vector::from(vector);
                let rows = sqlx::query(
                    r#"SELECT id, content_text,
                              EXTRACT(EPOCH FROM created_at)::bigint AS ts,
                              (embedding <=> $1) AS dist
                       FROM semantic_memory
                       ORDER BY embedding <=> $1
                       LIMIT $2"#,
                )
                .bind(v)
                .bind(query.top_k.max(1) as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| KowalskiError::Memory(format!("semantic vector search: {e}")))?;

                for row in rows {
                    let id: String = row
                        .try_get("id")
                        .map_err(|e| KowalskiError::Memory(format!("semantic row decode: {e}")))?;
                    let content_text: String = row
                        .try_get("content_text")
                        .map_err(|e| KowalskiError::Memory(format!("semantic row decode: {e}")))?;
                    let ts: i64 = row
                        .try_get("ts")
                        .map_err(|e| KowalskiError::Memory(format!("semantic row decode: {e}")))?;
                    let dist: f32 = row
                        .try_get("dist")
                        .map_err(|e| KowalskiError::Memory(format!("semantic row decode: {e}")))?;
                    let score = (1.0_f32 - dist).clamp(-1.0, 1.0);
                    out.push(MemoryUnit {
                        id,
                        content: format!("{} (similarity {:.4})", content_text, score),
                        timestamp: ts.max(0) as u64,
                        embedding: None,
                    });
                }
            } else {
                warn!(
                    "search: vector_query length {} != {}",
                    vector.len(),
                    self.embedding_dims
                );
            }
        }

        let rows =
            sqlx::query(r#"SELECT predicate, object FROM semantic_relation WHERE subject = $1"#)
                .bind(&query.text_query)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| KowalskiError::Memory(format!("semantic relation search: {e}")))?;

        for row in rows {
            let predicate: String = row
                .try_get("predicate")
                .map_err(|e| KowalskiError::Memory(format!("semantic row decode: {e}")))?;
            let object: String = row
                .try_get("object")
                .map_err(|e| KowalskiError::Memory(format!("semantic row decode: {e}")))?;
            info!(
                "Found graph relationship: {} -[{}]-> {}",
                query.text_query, predicate, object
            );
            out.push(MemoryUnit {
                id: uuid::Uuid::new_v4().to_string(),
                content: format!(
                    "Graph Relationship: {} {} {}",
                    query.text_query, predicate, object
                ),
                timestamp: 0,
                embedding: None,
            });
        }

        Ok(out)
    }
}
