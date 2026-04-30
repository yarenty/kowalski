//! In-memory [`AgentRegistry`] for capability-based discovery (Phase 1).
//!
//! Postgres-backed persistence can reuse the same record shape later.

use crate::error::KowalskiError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Registered agent metadata (no live handles — orchestration wires those separately).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentRecord {
    pub id: String,
    pub capabilities: Vec<String>,
}

/// Process-local registry (thread-safe).
#[derive(Clone)]
pub struct AgentRegistry {
    inner: Arc<RwLock<HashMap<String, AgentRecord>>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register(&self, record: AgentRecord) -> Result<(), KowalskiError> {
        let mut g = self
            .inner
            .write()
            .map_err(|e| KowalskiError::Federation(format!("registry lock poisoned: {e}")))?;
        g.insert(record.id.clone(), record);
        Ok(())
    }

    pub fn deregister(&self, id: &str) -> Result<(), KowalskiError> {
        let mut g = self
            .inner
            .write()
            .map_err(|e| KowalskiError::Federation(format!("registry lock poisoned: {e}")))?;
        g.remove(id)
            .ok_or_else(|| KowalskiError::NotFound(format!("agent {id}")))?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<AgentRecord> {
        self.inner.read().ok()?.get(id).cloned()
    }

    pub fn list(&self) -> Vec<AgentRecord> {
        self.inner
            .read()
            .map(|g| g.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Agents declaring `cap` in their capability list (substring match on capability token).
    pub fn find_by_capability(&self, cap: &str) -> Vec<AgentRecord> {
        let c = cap.to_lowercase();
        self.list()
            .into_iter()
            .filter(|a| a.capabilities.iter().any(|x| x.to_lowercase().contains(&c)))
            .collect()
    }

    /// Like [`find_by_capability`](Self::find_by_capability), ordered by match quality: exact capability
    /// token first, then longer substring matches; ties broken by agent id.
    pub fn find_ranked_by_capability(&self, cap: &str) -> Vec<AgentRecord> {
        let c = cap.to_lowercase();
        let mut v = self.find_by_capability(cap);
        v.sort_by(|a, b| {
            let sa = capability_match_score(a, &c);
            let sb = capability_match_score(b, &c);
            sb.cmp(&sa).then_with(|| a.id.cmp(&b.id))
        });
        v
    }
}

fn capability_match_score(agent: &AgentRecord, c: &str) -> i32 {
    let mut best = 0i32;
    for x in &agent.capabilities {
        let xl = x.to_lowercase();
        if xl == c {
            return 10_000;
        }
        if xl.contains(c) {
            best = best.max(xl.len() as i32);
        }
    }
    best
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_find() {
        let r = AgentRegistry::new();
        r.register(AgentRecord {
            id: "a1".into(),
            capabilities: vec!["web_search".into(), "pdf".into()],
        })
        .unwrap();
        let hits = r.find_by_capability("web");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "a1");
    }

    #[test]
    fn ranked_prefers_exact_capability() {
        let r = AgentRegistry::new();
        r.register(AgentRecord {
            id: "broad".into(),
            capabilities: vec!["chat_assistant".into()],
        })
        .unwrap();
        r.register(AgentRecord {
            id: "exact".into(),
            capabilities: vec!["chat".into(), "mcp".into()],
        })
        .unwrap();
        let ranked = r.find_ranked_by_capability("chat");
        assert_eq!(ranked[0].id, "exact");
        assert_eq!(ranked[1].id, "broad");
    }
}
