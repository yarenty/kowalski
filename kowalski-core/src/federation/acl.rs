//! Agent Communication Language (ACL) — JSON-serializable messages for federation.
//!
//! Suitable for in-process brokers today and Postgres `NOTIFY` payloads later.

use crate::error::KowalskiError;
use serde::{Deserialize, Serialize};

/// Default cap on delegation depth when the sender omits `max_delegation_depth` (strict default).
pub const DEFAULT_MAX_DELEGATION_DEPTH: u32 = 3;

/// Hard upper bound: envelopes claiming a higher max are rejected (operator misconfiguration / abuse).
pub const ABSOLUTE_MAX_DELEGATION_DEPTH: u32 = 32;

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
    /// Directed delegation. Use `delegation_depth` / `max_delegation_depth` to avoid delegation loops.
    TaskDelegate {
        task_id: String,
        from_agent: String,
        to_agent: String,
        instruction: String,
        #[serde(default)]
        delegation_depth: u32,
        #[serde(default)]
        max_delegation_depth: Option<u32>,
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
    /// Horde run lifecycle: orchestrator announces a run has begun.
    RunStarted {
        run_id: String,
        horde: String,
        prompt: String,
        #[serde(default)]
        source: Option<String>,
        #[serde(default)]
        question: Option<String>,
        #[serde(default)]
        pipeline: Vec<String>,
    },
    /// Horde run lifecycle: orchestrator delegated a sub-agent task (UI conversation).
    TaskAssigned {
        run_id: String,
        horde: String,
        step: String,
        from: String,
        to: String,
        task_id: String,
        instruction: String,
    },
    /// Horde run lifecycle: a sub-agent worker started executing a task.
    TaskStarted {
        run_id: String,
        horde: String,
        step: String,
        agent: String,
        #[serde(default)]
        text: Option<String>,
    },
    /// Horde run lifecycle: arbitrary inter-agent or progress message.
    AgentMessage {
        run_id: String,
        horde: String,
        from: String,
        #[serde(default)]
        step: Option<String>,
        text: String,
    },
    /// Horde run lifecycle: a sub-agent finished (carries artifact path for chaining).
    TaskFinished {
        run_id: String,
        horde: String,
        step: String,
        agent: String,
        success: bool,
        #[serde(default)]
        artifact: Option<String>,
        summary: String,
    },
    /// Horde run lifecycle: orchestrator declares the run completed successfully.
    RunFinished {
        run_id: String,
        horde: String,
        #[serde(default)]
        artifacts: Vec<(String, String)>,
        #[serde(default)]
        text: Option<String>,
    },
    /// Horde run lifecycle: orchestrator declares the run failed.
    RunFailed {
        run_id: String,
        horde: String,
        reason: String,
        #[serde(default)]
        step: Option<String>,
    },
}

/// Reject [`AclMessage::TaskDelegate`] when `delegation_depth` exceeds the effective max.
/// When `max_delegation_depth` is omitted, [`DEFAULT_MAX_DELEGATION_DEPTH`] applies. Values above
/// [`ABSOLUTE_MAX_DELEGATION_DEPTH`] are rejected.
pub fn check_delegate_depth(msg: &AclMessage) -> Result<(), KowalskiError> {
    if let AclMessage::TaskDelegate {
        delegation_depth,
        max_delegation_depth,
        ..
    } = msg
    {
        let max: u32 = match max_delegation_depth {
            Some(m) if *m > ABSOLUTE_MAX_DELEGATION_DEPTH => {
                return Err(KowalskiError::Federation(format!(
                    "max_delegation_depth {} exceeds absolute limit {}",
                    *m, ABSOLUTE_MAX_DELEGATION_DEPTH
                )));
            }
            Some(m) => *m,
            None => DEFAULT_MAX_DELEGATION_DEPTH,
        };
        if *delegation_depth > max {
            return Err(KowalskiError::Federation(format!(
                "delegation_depth {} exceeds max {}",
                *delegation_depth, max
            )));
        }
    }
    Ok(())
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
            delegation_depth: 0,
            max_delegation_depth: Some(3),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let back: AclMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, back);
    }

    #[test]
    fn check_depth_rejects_overflow() {
        let msg = AclMessage::TaskDelegate {
            task_id: "t".into(),
            from_agent: "a".into(),
            to_agent: "b".into(),
            instruction: "x".into(),
            delegation_depth: 4,
            max_delegation_depth: Some(3),
        };
        assert!(check_delegate_depth(&msg).is_err());
    }

    #[test]
    fn check_depth_none_uses_default_cap() {
        let msg = AclMessage::TaskDelegate {
            task_id: "t".into(),
            from_agent: "a".into(),
            to_agent: "b".into(),
            instruction: "x".into(),
            delegation_depth: 4,
            max_delegation_depth: None,
        };
        assert!(check_delegate_depth(&msg).is_err());
        let ok = AclMessage::TaskDelegate {
            task_id: "t".into(),
            from_agent: "a".into(),
            to_agent: "b".into(),
            instruction: "x".into(),
            delegation_depth: 2,
            max_delegation_depth: None,
        };
        assert!(check_delegate_depth(&ok).is_ok());
    }

    #[test]
    fn check_depth_rejects_absurd_max() {
        let msg = AclMessage::TaskDelegate {
            task_id: "t".into(),
            from_agent: "a".into(),
            to_agent: "b".into(),
            instruction: "x".into(),
            delegation_depth: 0,
            max_delegation_depth: Some(ABSOLUTE_MAX_DELEGATION_DEPTH + 1),
        };
        assert!(check_delegate_depth(&msg).is_err());
    }
}
