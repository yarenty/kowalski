use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{
    agent::{FederatedAgent, FederationRole},
    registry::AgentRegistry,
    message::{FederationMessage, MessageType},
};

/// Represents a task that needs to be delegated
#[derive(Debug, Clone)]
pub struct FederationTask {
    pub id: String,
    pub task_type: String,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub priority: TaskPriority,
    pub status: TaskStatus,
    pub assigned_to: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Task status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Assigned,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Orchestrator manages task delegation and coordination
pub struct Orchestrator {
    registry: Arc<AgentRegistry>,
    tasks: Arc<RwLock<HashMap<String, FederationTask>>>,
}

impl Orchestrator {
    /// Create a new orchestrator
    pub fn new(registry: Arc<AgentRegistry>) -> Self {
        Self {
            registry,
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new task
    pub async fn create_task(
        &self,
        task_type: String,
        content: String,
        metadata: Option<serde_json::Value>,
        priority: TaskPriority,
    ) -> Result<String, FederationError> {
        let task_id = uuid::Uuid::new_v4().to_string();
        let task = FederationTask {
            id: task_id.clone(),
            task_type,
            content,
            metadata,
            priority,
            status: TaskStatus::Pending,
            assigned_to: None,
            created_at: get_timestamp(),
            updated_at: get_timestamp(),
        };

        self.tasks.write().await.insert(task_id.clone(), task);
        info!("Created task: {}", task_id);
        Ok(task_id)
    }

    /// Delegate a task to the most suitable agent
    pub async fn delegate_task(
        &self,
        task_id: &str,
    ) -> Result<(), FederationError> {
        let mut tasks = self.tasks.write().await;
        let task = tasks.get_mut(task_id).ok_or_else(|| {
            FederationError::TaskNotFound(task_id.to_string())
        })?;

        if task.status != TaskStatus::Pending {
            return Err(FederationError::InvalidTaskState(task_id.to_string()));
        }

        // Find the most suitable agent
        let agents = self.registry.list_agents().await;
        let mut suitable_agents: Vec<_> = agents
            .iter()
            .filter(|(_, role)| *role == FederationRole::Worker)
            .map(|(id, _)| id.clone())
            .collect();

        if suitable_agents.is_empty() {
            return Err(FederationError::NoSuitableAgents);
        }

        // For now, just pick the first available agent
        let assigned_agent = suitable_agents.remove(0);
        task.assigned_to = Some(assigned_agent.clone());
        task.status = TaskStatus::Assigned;
        task.updated_at = get_timestamp();

        // Send task delegation message
        let message = FederationMessage::new(
            MessageType::TaskDelegation,
            "coordinator".to_string(),
            Some(assigned_agent),
            serde_json::to_string(&task).unwrap_or_default(),
            Some(serde_json::json!({
                "task_id": task_id,
                "priority": format!("{:?}", task.priority),
            })),
        );

        self.registry
            .send_message(&assigned_agent, message)
            .await
            .map_err(|e| FederationError::MessageDeliveryFailed(e.to_string()))
    }

    /// Update task status
    pub async fn update_task_status(
        &self,
        task_id: &str,
        status: TaskStatus,
    ) -> Result<(), FederationError> {
        let mut tasks = self.tasks.write().await;
        let task = tasks.get_mut(task_id).ok_or_else(|| {
            FederationError::TaskNotFound(task_id.to_string())
        })?;

        task.status = status;
        task.updated_at = get_timestamp();
        info!("Task {} status updated to: {:?}", task_id, status);
        Ok(())
    }

    /// Get task status
    pub async fn get_task_status(&self, task_id: &str) -> Result<TaskStatus, FederationError> {
        let tasks = self.tasks.read().await;
        let task = tasks.get(task_id).ok_or_else(|| {
            FederationError::TaskNotFound(task_id.to_string())
        })?;

        Ok(task.status)
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: &str) -> Result<(), FederationError> {
        let mut tasks = self.tasks.write().await;
        let task = tasks.get_mut(task_id).ok_or_else(|| {
            FederationError::TaskNotFound(task_id.to_string())
        })?;

        task.status = TaskStatus::Cancelled;
        task.updated_at = get_timestamp();
        info!("Task {} cancelled", task_id);
        Ok(())
    }

    /// List all tasks
    pub async fn list_tasks(&self) -> Vec<FederationTask> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }
}

/// Helper function to get current timestamp
fn get_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Extended FederationError enum for orchestrator-specific errors
#[derive(Error, Debug, Serialize)]
pub enum FederationError {
    #[error("Task {0} not found")]
    TaskNotFound(String),
    #[error("Invalid task state for task {0}")]
    InvalidTaskState(String),
    #[error("No suitable agents available for task delegation")]
    NoSuitableAgents,
    #[error("Failed to deliver task delegation message: {0}")]
    MessageDeliveryFailed(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
}
