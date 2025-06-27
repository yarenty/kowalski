use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::{FederatedAgent, FederationMessage, FederationRole};

/// Registry for managing federated agents
pub struct AgentRegistry {
    agents: Arc<RwLock<HashMap<String, Arc<dyn FederatedAgent + Send + Sync>>>>,
}

impl AgentRegistry {
    /// Create a new agent registry
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new agent in the federation
    pub async fn register_agent(
        &self,
        agent: Arc<dyn FederatedAgent + Send + Sync>,
    ) -> Result<(), FederationError> {
        let id = agent.federation_id().to_string();
        let mut agents = self.agents.write().await;
        
        if agents.contains_key(&id) {
            return Err(FederationError::DuplicateAgent(id));
        }

        agents.insert(id.clone(), agent.clone());
        info!("Registered agent: {}", id);
        Ok(())
    }

    /// Get an agent by ID
    pub async fn get_agent(
        &self,
        id: &str,
    ) -> Option<Arc<dyn FederatedAgent + Send + Sync>> {
        let agents = self.agents.read().await;
        agents.get(id).cloned()
    }

    /// List all agents in the federation
    pub async fn list_agents(&self) -> Vec<(String, FederationRole)> {
        let agents = self.agents.read().await;
        agents
            .iter()
            .map(|(id, agent)| (id.clone(), agent.federation_role()))
            .collect()
    }

    /// Broadcast a message to all agents
    pub async fn broadcast_message(
        &self,
        message: FederationMessage,
    ) -> Result<(), FederationError> {
        let agents = self.agents.read().await;
        for agent in agents.values() {
            if agent.federation_id() != message.sender {
                agent.handle_federation_message(message.clone()).await?;
            }
        }
        Ok(())
    }

    /// Send a message to a specific agent
    pub async fn send_message(
        &self,
        recipient: &str,
        message: FederationMessage,
    ) -> Result<(), FederationError> {
        if let Some(agent) = self.get_agent(recipient).await {
            agent.handle_federation_message(message).await?;
            Ok(())
        } else {
            Err(FederationError::AgentNotFound(recipient.to_string()))
        }
    }

    /// Remove an agent from the federation
    pub async fn remove_agent(&self, id: &str) -> Result<(), FederationError> {
        let mut agents = self.agents.write().await;
        if agents.remove(id).is_some() {
            info!("Removed agent: {}", id);
            Ok(())
        } else {
            Err(FederationError::AgentNotFound(id.to_string()))
        }
    }
}
