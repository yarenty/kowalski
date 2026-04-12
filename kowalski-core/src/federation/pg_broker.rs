//! Postgres `LISTEN` / `NOTIFY` transport for [`AclEnvelope`] JSON (requires `--features postgres`).
//!
//! Payload must stay under PostgreSQL's ~8000-byte NOTIFY limit.

use crate::error::KowalskiError;
use crate::federation::acl::AclEnvelope;
use crate::federation::broker::MessageBroker;
use async_trait::async_trait;
use sqlx::postgres::{PgListener, PgPool};

/// NOTIFY channel name is fixed at construction; payload is JSON [`AclEnvelope`].
pub struct PgBroker {
    pool: PgPool,
    channel: String,
}

impl PgBroker {
    pub fn new(pool: PgPool, channel: impl Into<String>) -> Self {
        Self {
            pool,
            channel: channel.into(),
        }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub fn channel_name(&self) -> &str {
        &self.channel
    }

    /// Subscribe to NOTIFY on this broker's channel; spawns a background task.
    pub fn subscribe(
        &self,
        buffer: usize,
    ) -> Result<tokio::sync::mpsc::Receiver<AclEnvelope>, KowalskiError> {
        let (tx, rx) = tokio::sync::mpsc::channel(buffer);
        let pool = self.pool.clone();
        let channel = self.channel.clone();
        tokio::spawn(async move {
            let mut listener = match PgListener::connect_with(&pool).await {
                Ok(l) => l,
                Err(e) => {
                    log::error!("PgListener::connect_with: {}", e);
                    return;
                }
            };
            if let Err(e) = listener.listen(&channel).await {
                log::error!("LISTEN {}: {}", channel, e);
                return;
            }
            loop {
                match listener.recv().await {
                    Ok(n) => {
                        let payload = n.payload();
                        match serde_json::from_str::<AclEnvelope>(payload) {
                            Ok(env) => {
                                if tx.send(env).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => log::warn!("skip NOTIFY JSON: {}", e),
                        }
                    }
                    Err(e) => {
                        log::error!("PgListener::recv: {}", e);
                        break;
                    }
                }
            }
        });
        Ok(rx)
    }
}

const MAX_NOTIFY_BYTES: usize = 7500;

#[async_trait]
impl MessageBroker for PgBroker {
    async fn publish(&self, envelope: &AclEnvelope) -> Result<(), KowalskiError> {
        let json = serde_json::to_string(envelope).map_err(|e| {
            KowalskiError::Federation(format!("ACL JSON serialize: {e}"))
        })?;
        if json.len() > MAX_NOTIFY_BYTES {
            return Err(KowalskiError::Federation(format!(
                "ACL JSON ({} bytes) exceeds safe NOTIFY limit (~{})",
                json.len(),
                MAX_NOTIFY_BYTES
            )));
        }
        sqlx::query("SELECT pg_notify($1, $2)")
            .bind(&self.channel)
            .bind(&json)
            .execute(&self.pool)
            .await
            .map_err(|e| KowalskiError::Federation(format!("pg_notify: {e}")))?;
        Ok(())
    }
}

/// `LISTEN` on `notify_channel` and republish JSON [`AclEnvelope`]s into the in-process
/// [`super::broker::MpscBroker`] (e.g. so `serve` SSE clients see cross-process NOTIFY).
pub async fn bridge_postgres_notify_to_mpsc(
    database_url: &str,
    notify_channel: &str,
    local: std::sync::Arc<super::broker::MpscBroker>,
) -> Result<(), KowalskiError> {
    let pool = PgPool::connect(database_url)
        .await
        .map_err(|e| KowalskiError::Federation(format!("PgPool::connect: {e}")))?;
    let pg = PgBroker::new(pool, notify_channel);
    let mut rx = pg.subscribe(256)?;
    tokio::spawn(async move {
        while let Some(env) = rx.recv().await {
            let _ = local.publish_to_topic(&env).await;
        }
    });
    Ok(())
}
