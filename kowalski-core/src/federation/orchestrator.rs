//! Thin orchestration helper: registry + broker (extensible for routing rules).

use crate::error::KowalskiError;
use crate::federation::acl::AclEnvelope;
use crate::federation::broker::{MessageBroker, MpscBroker};
use crate::federation::registry::AgentRegistry;
use std::sync::Arc;

/// Holds shared federation components for one process.
pub struct FederationOrchestrator {
    pub registry: Arc<AgentRegistry>,
    pub broker: Arc<MpscBroker>,
}

impl FederationOrchestrator {
    pub fn new(registry: Arc<AgentRegistry>, broker: Arc<MpscBroker>) -> Self {
        Self { registry, broker }
    }

    /// Convenience: publish through the [`MessageBroker`] trait object path.
    pub async fn publish(&self, envelope: &AclEnvelope) -> Result<(), KowalskiError> {
        self.broker.publish(envelope).await
    }
}
