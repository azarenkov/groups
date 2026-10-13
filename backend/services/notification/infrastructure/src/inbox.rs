use std::time::Duration;

use async_trait::async_trait;
use redis::AsyncCommands;
use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use tokio::sync::Mutex;

const REDIS_TIMEOUT: Duration = Duration::from_secs(1);

const CLAIM_LEASE_SECS: u64 = 60;
const PENDING: &str = "pending";
const DONE: &str = "done";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Claim {
    Acquired,
    Done,
    Busy,
    Unavailable,
}

#[async_trait]
pub trait HandledEvents: Send + Sync {
    async fn claim(&self, consumer: &str, event_id: &str) -> Claim;
    async fn complete(&self, consumer: &str, event_id: &str);
    async fn release(&self, consumer: &str, event_id: &str);
}

pub struct RedisHandledEvents {
    client: Option<redis::Client>,
    connection: Mutex<Option<ConnectionManager>>,
    ttl_secs: u64,
}

impl RedisHandledEvents {
    pub fn new(redis_url: Option<&str>, ttl_secs: u64) -> Self {
        let client = redis_url.filter(|url| !url.is_empty()).and_then(|url| match redis::Client::open(url) {
            Ok(client) => Some(client),
            Err(e) => {
                tracing::warn!(error = %e, "Invalid REDIS_URL; repeated events will not be detected");
                None
            }
        });
        Self { client, connection: Mutex::new(None), ttl_secs }
    }

    async fn connection(&self) -> Option<ConnectionManager> {
        let client = self.client.as_ref()?;
        let mut guard = self.connection.lock().await;
        if let Some(connection) = guard.as_ref() {
            return Some(connection.clone());
        }
        let config = ConnectionManagerConfig::new()
            .set_connection_timeout(REDIS_TIMEOUT)
            .set_response_timeout(REDIS_TIMEOUT);
        match client.get_connection_manager_with_config(config).await {
            Ok(connection) => {
                *guard = Some(connection.clone());
                Some(connection)
            }
            Err(e) => {
                tracing::warn!(error = %e, "Redis unavailable; repeated events will not be detected");
                None
            }
        }
    }

    fn key(consumer: &str, event_id: &str) -> String {
        format!("notification:handled:{consumer}:{event_id}")
    }
}

#[async_trait]
impl HandledEvents for RedisHandledEvents {
    async fn claim(&self, consumer: &str, event_id: &str) -> Claim {
        let Some(mut connection) = self.connection().await else {
            return Claim::Unavailable;
        };
        let key = Self::key(consumer, event_id);
        let claimed: Result<Option<String>, _> = redis::cmd("SET")
            .arg(&key)
            .arg(PENDING)
            .arg("NX")
            .arg("EX")
            .arg(CLAIM_LEASE_SECS)
            .query_async(&mut connection)
            .await;
        match claimed {
            Ok(Some(_)) => Claim::Acquired,
            Ok(None) => match connection.get::<_, Option<String>>(&key).await {
                Ok(Some(state)) if state == DONE => Claim::Done,
                Ok(Some(_)) => Claim::Busy,
                Ok(None) => Claim::Busy,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to read an event claim; handling it");
                    Claim::Unavailable
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "Failed to claim an event; handling it");
                Claim::Unavailable
            }
        }
    }

    async fn complete(&self, consumer: &str, event_id: &str) {
        let Some(mut connection) = self.connection().await else {
            return;
        };
        if let Err(e) = connection
            .set_ex::<_, _, ()>(Self::key(consumer, event_id), DONE, self.ttl_secs)
            .await
        {
            tracing::warn!(error = %e, "Failed to remember a handled event");
        }
    }

    async fn release(&self, consumer: &str, event_id: &str) {
        let Some(mut connection) = self.connection().await else {
            return;
        };
        if let Err(e) = connection.del::<_, ()>(Self::key(consumer, event_id)).await {
            tracing::warn!(error = %e, "Failed to release an event claim");
        }
    }
}
