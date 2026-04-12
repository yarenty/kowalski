# Memory stack: history and design goals

This document is the **canonical reference** for why Kowalski’s memory layer looks the way it does, and how that relates to **dependencies and operational complexity**. It is linked from root and component `AGENTS.md` files and from memory-related articles so the story stays consistent.

---

## Historical note: Qdrant as proof of concept

Early iterations used **[Qdrant](https://qdrant.tech/)** as a **proof of concept** for **semantic (vector) memory**: an external vector database showed how embeddings could be stored and queried at scale, and helped validate the **multi-tier** idea (working → episodic → semantic) before locking in storage details.

That work was valuable for **exploring** retrieval and consolidation flows. It is **not** the long-term default for a minimal, robust deployment story.

---

## Historical note: `petgraph` (crate, not a service)

For structured **subject → predicate → object** edges, the code briefly used the **`petgraph`** crate. That was **never** an external installation—only a **Cargo dependency**. It added graph algorithms we did not need for the current behavior (outgoing edges from a subject string).

**Current default:** relation triples are stored in a plain **`HashMap<String, Vec<(String, String)>>`** (subject → list of `(predicate, object)`), **no extra graph crate**. This keeps the **relational** slice of semantic memory on **`std` collections** only, alongside the in-process vector list.

---

## Episodic tier: `episodic_kv` (SQLite file or PostgreSQL)

**Tier 2 (episodic buffer)** stores each [`MemoryUnit`](../../kowalski-core/src/memory/mod.rs) as JSON in table **`episodic_kv`** (`id`, `payload`), using **`sqlx`**. Default is **embedded SQLite** (no separate server). If **`memory.database_url`** is **`postgres://…`**, the same table and JSON shape are used in **PostgreSQL** (see `migrations/postgres/002_episodic_kv.sql`). **No C++ RocksDB** build.

| Aspect | Notes |
|--------|--------|
| **Path (default)** | Without a Postgres URL: `memory.episodic_path` — if it ends with `.sqlite` / `.db`, that file is used; otherwise a directory is created and **`episodic.sqlite`** is opened inside it. |
| **Postgres** | With **`memory.database_url`** = `postgres://…`, Tier 2 reads/writes **`episodic_kv`** in that database (run migrations via [`db::run_memory_migrations_if_configured`](../../kowalski-core/src/db/mod.rs)). |
| **Build** | Native SQLite via `libsqlite3-sys`; Postgres uses the existing **`sqlx`** Postgres driver. |
| **Historical note** | Episodic storage previously used **RocksDB**; it was replaced to **reduce native dependency surface** and align Tier 2 with **SQL** already in the stack. |

**Direction:** [WP3](../rebuild_tasks/wp3_postgres_data_layer_tasks.md) continues with **pgvector**, **`episodic_memory`** normalized rows, and **`agent_state`** as needed; Tier 2 `episodic_kv` JSON is the current interchange format for `MemoryUnit` blobs.

---

## Design goals: simple, robust, few moving parts

The **main direction** for the framework is:

| Goal | Implication |
|------|-------------|
| **Simplicity** | Prefer what runs **in-process** or in **embedded** stores (e.g. **SQLite** for episodic + optional SQL, Postgres when configured) over mandatory network services. |
| **Robustness** | Fewer daemons and fewer failure points: every extra service is another thing that must be up, versioned, and secured. |
| **Dependency minimization** | Avoid **required** heavy or external dependencies for core workflows; keep the default path **dependency-light** so installs and edge deployments stay predictable. |
| **Optional scale-out** | When a deployment **needs** a dedicated vector DB, hosted SQL, or cluster storage, those remain **additive**—not prerequisites for “hello world” or local dev. |

**Default:** the semantic tier uses **in-process cosine similarity** over stored embeddings and a **`HashMap` of relation edges** for triples—**no Qdrant client** and **no graph library** in the default build.

**With `postgres://`:** Tier 3 can use **`semantic_memory`** (`vector(768)` in `003_semantic_memory.sql`) and **`semantic_relation`** via **`PostgresSemanticStore`** (`kowalski-core/src/memory/semantic_pg.rs`); chat-time **`retrieve`** embeds the user query and runs **pgvector** `<=>` ordering. Set **`memory.embedding_vector_dimensions`** to match your embedder and the migration’s `vector(N)`.

Optional SQL migrations (`sqlite:` / `postgres://`) support durable metadata and episodic-style tables without mandating a separate vector-only service.

---

## For contributors and readers of older docs

Articles and diagrams that still mention “Qdrant” describe **earlier PoC wiring** or **generic** “vector DB” options in architecture discussions. Treat those as **historical or illustrative** unless explicitly labeled as current required setup. When in doubt, follow this file and [`kowalski-core/MEMORY_ARCHITECTURE.md`](../kowalski-core/MEMORY_ARCHITECTURE.md).

---

*Last updated as part of the rebuild emphasis on minimal moving parts and a dependency-light default path.*
