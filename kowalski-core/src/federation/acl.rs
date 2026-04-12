//! Agent Communication Language (ACL) — JSON-serializable messages for federation.
//!
//! Suitable for in-process brokers today and Postgres `NOTIFY` payloads later.

use serde::{Deserialize, Serialize};

/// Wire envelope: every publish carries topic routing + provenance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AclEnvelope {
    pub id: String,
    pub topic: String,
    pub sender: String,
    pub payload: AclMessage,
}

impl AclEnvelope {
    pub fn new(topic: impl Into<String>, sender: impl Into<String>, payload: AclMessage) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            topic: topic.into(),
            sender: sender.into(),
            payload,
        }
    }
}

/// ACL payload variants (extend as orchestration grows).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AclMessage {
    /// Health / diagnostics.
    Ping { text: String },
    /// Orchestrator announces work matching capabilities (discovery).
    TaskOffer {
        task_id: String,
        summary: String,
        required_capabilities: Vec<String>,
    },
    /// Directed delegation.
    TaskDelegate {
        task_id: String,
        from_agent: String,
        to_agent: String,
        instruction: String,
    },
    TaskResult {
        task_id: String,
        from_agent: String,
        outcome: String,
        success: bool,
    },
    Error {
        code: String,
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acl_round_trips_json() {
        let msg = AclMessage::TaskDelegate {
            task_id: "t1".into(),
            from_agent: "orch".into(),
            to_agent: "worker".into(),
            instruction: "Summarize".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: AclMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, back);
    }
}
