# Memory stack: history and design goals

This document is the **canonical reference** for why Kowalski’s memory layer looks the way it does, and how that relates to **dependencies and operational complexity**. It is linked from root and component `AGENTS.md` files and from memory-related articles so the story stays consistent.

---

## Historical note: Qdrant as proof of concept

Early iterations used **[Qdrant](https://qdrant.tech/)** as a **proof of concept** for **semantic (vector) memory**: an external vector database showed how embeddings could be stored and queried at scale, and helped validate the **multi-tier** idea (working → episodic → semantic) before locking in storage details.

That work was valuable for **exploring** retrieval and consolidation flows. It is **not** the long-term default for a minimal, robust deployment story.

---

## Design goals: simple, robust, few moving parts

The **main direction** for the framework is:

| Goal | Implication |
|------|-------------|
| **Simplicity** | Prefer what runs **in-process** or in **embedded** stores (e.g. RocksDB, optional SQLite/Postgres) over mandatory network services. |
| **Robustness** | Fewer daemons and fewer failure points: every extra service is another thing that must be up, versioned, and secured. |
| **Dependency minimization** | Avoid **required** heavy or external dependencies for core workflows; keep the default path **dependency-light** so installs and edge deployments stay predictable. |
| **Optional scale-out** | When a deployment **needs** a dedicated vector DB, hosted SQL, or cluster storage, those remain **additive**—not prerequisites for “hello world” or local dev. |

Today, the semantic tier uses **in-process cosine similarity** over stored embeddings plus **`petgraph`** for structured edges—**no Qdrant client** in the default build. Optional SQL migrations (`sqlite:` / `postgres://`) support durable metadata and episodic-style tables without mandating a second always-on vector service.

---

## For contributors and readers of older docs

Articles and diagrams that still mention “Qdrant” describe **earlier PoC wiring** or **generic** “vector DB” options in architecture discussions. Treat those as **historical or illustrative** unless explicitly labeled as current required setup. When in doubt, follow this file and [`kowalski-core/MEMORY_ARCHITECTURE.md`](../kowalski-core/MEMORY_ARCHITECTURE.md).

---

*Last updated as part of the rebuild emphasis on minimal moving parts and a dependency-light default path.*
