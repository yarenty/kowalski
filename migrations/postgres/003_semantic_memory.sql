-- Tier 3 semantic store: vectors + optional relation triples.
-- Default `vector(768)` matches Ollama `nomic-embed-text`. Change dimension only with a matching
-- `memory.embedding_vector_dimensions` config and a new migration / ALTER.

CREATE TABLE IF NOT EXISTS semantic_memory (
    id TEXT PRIMARY KEY NOT NULL,
    content_text TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    embedding vector(768) NOT NULL
);

CREATE TABLE IF NOT EXISTS semantic_relation (
    subject TEXT NOT NULL,
    predicate TEXT NOT NULL,
    object TEXT NOT NULL,
    PRIMARY KEY (subject, predicate, object)
);

CREATE INDEX IF NOT EXISTS idx_semantic_relation_subject ON semantic_relation (subject);
