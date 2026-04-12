//! Orchestration: capability-based routing over a [`MessageBroker`].

use crate::error::KowalskiError;
use crate::federation::acl::{check_delegate_depth, AclEnvelope, AclMessage};
use crate::federation::broker::MessageBroker;
use crate::federation::registry::AgentRegistry;
use std::sync::Arc;

/// Holds registry + broker for one deployment (in-process or bridged to Postgres).
pub struct FederationOrchestrator {
    pub registry: Arc<AgentRegistry>,
    broker: Arc<dyn MessageBroker>,
    pub orchestrator_id: String,
    pub default_topic: String,
    /// Default cap on re-delegation chains (embedded in [`AclMessage::TaskDelegate`]).
    pub default_max_delegation_depth: u32,
}

impl FederationOrchestrator {
    pub fn new<B: MessageBroker + 'static>(registry: Arc<AgentRegistry>, broker: Arc<B>) -> Self {
        let broker: Arc<dyn MessageBroker> = broker;
        Self {
            registry,
            broker,
            orchestrator_id: "orchestrator".to_string(),
            default_topic: "federation".to_string(),
            default_max_delegation_depth: 5,
        }
    }

    /// Publish after validating delegation depth when applicable.
    pub async fn publish(&self, envelope: &AclEnvelope) -> Result<(), KowalskiError> {
        check_delegate_depth(&envelope.payload)?;
        self.broker.publish(envelope).await
    }

    /// First agent matching `required_capability` receives a [`AclMessage::TaskDelegate`].
    /// Returns the chosen agent id, or `None` if no match.
    pub async fn delegate_first_match(
        &self,
        task_id: &str,
        instruction: &str,
        required_capability: &str,
    ) -> Result<Option<String>, KowalskiError> {
        let candidates = self.registry.find_by_capability(required_capability);
        let Some(agent) = candidates.first() else {
            return Ok(None);
        };
        let msg = AclMessage::TaskDelegate {
            task_id: task_id.to_string(),
            from_agent: self.orchestrator_id.clone(),
            to_agent: agent.id.clone(),
            instruction: instruction.to_string(),
            delegation_depth: 0,
            max_delegation_depth: Some(self.default_max_delegation_depth),
        };
        check_delegate_depth(&msg)?;
        let env = AclEnvelope::new(
            self.default_topic.clone(),
            self.orchestrator_id.clone(),
            msg,
        );
        self.broker.publish(&env).await?;
        Ok(Some(agent.id.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::federation::broker::MpscBroker;

    #[tokio::test]
    async fn delegate_publishes_to_broker() {
        let broker = Arc::new(MpscBroker::new());
        let reg = Arc::new(AgentRegistry::new());
        reg.register(crate::federation::AgentRecord {
            id: "worker".into(),
            capabilities: vec!["search".into()],
        })
        .unwrap();
        let orch = FederationOrchestrator::new(reg, broker.clone());
        let mut rx = broker.subscribe("federation", 4);
        let to = orch
            .delegate_first_match("t1", "find X", "search")
            .await
            .unwrap();
        assert_eq!(to.as_deref(), Some("worker"));
        let env = rx.recv().await.unwrap();
        assert!(matches!(env.payload, AclMessage::TaskDelegate { .. }));
    }
}
