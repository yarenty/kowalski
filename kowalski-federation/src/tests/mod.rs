use super::*;
use crate::agent::{FederatedAgent, FederationRole};
use crate::registry::AgentRegistry;
use crate::message::{FederationMessage, MessageType};
use kowalski_core::BaseAgent;
use kowalski_core::config::Config;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

#[tokio::test]
async fn test_agent_registry() {
    // Create a test registry
    let registry = AgentRegistry::new();

    // Create test agents
    let config = Config::default();
    let mut agent1 = BaseAgent::new(config.clone(), "agent1", "Test agent 1").unwrap();
    let mut agent2 = BaseAgent::new(config.clone(), "agent2", "Test agent 2").unwrap();

    // Register agents
    let agent1 = Arc::new(agent1);
    let agent2 = Arc::new(agent2);

    registry.register_agent(agent1.clone()).await.unwrap();
    registry.register_agent(agent2.clone()).await.unwrap();

    // Test getting agents
    let agents = registry.list_agents().await;
    assert_eq!(agents.len(), 2);
    assert!(agents.contains(&("agent1".to_string(), FederationRole::Worker)));
    assert!(agents.contains(&("agent2".to_string(), FederationRole::Worker)));

    // Test message broadcasting
    let message = FederationMessage::new(
        MessageType::Status,
        "agent1".to_string(),
        None,
        "Test status update".to_string(),
        None,
    );

    registry.broadcast_message(message.clone()).await.unwrap();

    // Test sending message to specific agent
    registry.send_message("agent2", message).await.unwrap();

    // Test removing agent
    registry.remove_agent("agent1").await.unwrap();
    let agents = registry.list_agents().await;
    assert_eq!(agents.len(), 1);
    assert!(agents.contains(&("agent2".to_string(), FederationRole::Worker)));
}

#[tokio::test]
async fn test_federation_message() {
    let message = FederationMessage::new(
        MessageType::TaskDelegation,
        "coordinator".to_string(),
        Some("worker".to_string()),
        "Analyze this data".to_string(),
        Some(serde_json::json!({
            "priority": "high",
            "deadline": "2025-06-27T14:00:00Z"
        })),
    );

    assert_eq!(message.message_type, MessageType::TaskDelegation);
    assert_eq!(message.sender, "coordinator");
    assert_eq!(message.recipient, Some("worker".to_string()));
    assert_eq!(message.content, "Analyze this data");
    assert!(message.metadata.is_some());
}

#[tokio::test]
async fn test_federated_agent() {
    let config = Config::default();
    let mut agent = BaseAgent::new(config, "test-agent", "Test federated agent").unwrap();

    assert_eq!(agent.federation_id(), "test-agent");
    assert_eq!(agent.federation_role(), FederationRole::Worker);

    agent.set_federation_role(FederationRole::Coordinator);
    assert_eq!(agent.federation_role(), FederationRole::Coordinator);

    // Test message handling
    let message = FederationMessage::new(
        MessageType::Register,
        "new-agent".to_string(),
        None,
        "Registering with federation".to_string(),
        None,
    );

    agent.handle_federation_message(message).await.unwrap();
}
