//! In-process message broker (fan-out to subscribers per topic).
//!
//! Future: `PgBroker` using Postgres `LISTEN` / `NOTIFY` with JSON payloads.

use crate::error::KowalskiError;
use crate::federation::acl::AclEnvelope;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Publishes [`AclEnvelope`] messages; transport-agnostic contract for federation.
#[async_trait]
pub trait MessageBroker: Send + Sync {
    async fn publish(&self, envelope: &AclEnvelope) -> Result<(), KowalskiError>;
}

type SubscriberVec = Vec<tokio::sync::mpsc::Sender<AclEnvelope>>;

/// Local broker: multiple [`subscribe`](MpscBroker::subscribe) handles per topic;
/// [`publish`](MpscBroker::publish) clones the envelope to each subscriber.
#[derive(Clone)]
pub struct MpscBroker {
    inner: Arc<Mutex<HashMap<String, SubscriberVec>>>,
}

impl MpscBroker {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Receive envelopes for `topic`. Buffer size per subscriber channel.
    pub fn subscribe(&self, topic: &str, buffer: usize) -> tokio::sync::mpsc::Receiver<AclEnvelope> {
        let (tx, rx) = tokio::sync::mpsc::channel(buffer);
        self.inner
            .lock()
            .expect("mpsc broker lock")
            .entry(topic.to_string())
            .or_default()
            .push(tx);
        rx
    }

    /// Publish to all subscribers on `envelope.topic`. No subscribers → Ok (no-op).
    pub async fn publish_to_topic(&self, envelope: &AclEnvelope) -> Result<(), KowalskiError> {
        let topic = envelope.topic.clone();
        let senders: Vec<_> = {
            let g = self.inner.lock().expect("mpsc broker lock");
            g.get(&topic).cloned().unwrap_or_default()
        };
        for s in senders {
            s.send(envelope.clone())
                .await
                .map_err(|e| KowalskiError::Federation(format!("subscriber dropped: {e}")))?;
        }
        Ok(())
    }
}

#[async_trait]
impl MessageBroker for MpscBroker {
    async fn publish(&self, envelope: &AclEnvelope) -> Result<(), KowalskiError> {
        self.publish_to_topic(envelope).await
    }
}

impl Default for MpscBroker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::federation::acl::AclMessage;

    #[tokio::test]
    async fn two_subscribers_receive_delegate() {
        let broker = MpscBroker::new();
        let mut a = broker.subscribe("tasks", 8);
        let mut b = broker.subscribe("tasks", 8);
        let env = AclEnvelope::new(
            "tasks",
            "orch",
            AclMessage::TaskDelegate {
                task_id: "1".into(),
                from_agent: "orch".into(),
                to_agent: "agent-b".into(),
                instruction: "go".into(),
            },
        );
        broker.publish_to_topic(&env).await.unwrap();
        let ra = a.recv().await.unwrap();
        let rb = b.recv().await.unwrap();
        assert_eq!(ra.payload, env.payload);
        assert_eq!(rb.payload, env.payload);
    }

    #[tokio::test]
    async fn topic_isolation() {
        let broker = MpscBroker::new();
        let mut t1 = broker.subscribe("t1", 4);
        broker
            .publish_to_topic(&AclEnvelope::new(
                "t2",
                "x",
                AclMessage::Ping {
                    text: "hi".into(),
                },
            ))
            .await
            .unwrap();
        assert!(t1.try_recv().is_err());
    }
}
