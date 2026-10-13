use std::time::Duration;

use async_trait::async_trait;
use redis::AsyncCommands;
use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use tokio::sync::Mutex;

const REDIS_TIMEOUT: Duration = Duration::from_secs(1);

const CLAIM_LEASE_SECS: u64 = 60;
pub const CLAIM_RENEW_EVERY: Duration = Duration::from_secs(20);
const DONE: &str = "done";

const COMPLETE_IF_OWNED: &str = r#"
if redis.call('GET', KEYS[1]) == ARGV[1] then
  redis.call('SET', KEYS[1], ARGV[2], 'EX', ARGV[3])
  return 1
end
return 0
"#;

const RENEW_IF_OWNED: &str = r#"
if redis.call('GET', KEYS[1]) == ARGV[1] then
  return redis.call('EXPIRE', KEYS[1], ARGV[2])
end
return 0
"#;

const RELEASE_IF_OWNED: &str = r#"
if redis.call('GET', KEYS[1]) == ARGV[1] then
  return redis.call('DEL', KEYS[1])
end
return 0
"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Claim {
    Acquired(String),
    Done,
    Busy,
    Unavailable,
}

#[async_trait]
pub trait HandledEvents: Send + Sync {
    async fn claim(&self, consumer: &str, event_id: &str) -> Claim;
    async fn renew(&self, consumer: &str, event_id: &str, token: &str);
    async fn complete(&self, consumer: &str, event_id: &str, token: &str);
    async fn release(&self, consumer: &str, event_id: &str, token: &str);
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

    async fn run_if_owned(&self, script: &str, key: String, args: &[String], what: &str) {
        let Some(mut connection) = self.connection().await else {
            return;
        };
        let script = redis::Script::new(script);
        let mut invocation = script.key(key);
        for arg in args {
            invocation.arg(arg);
        }
        if let Err(e) = invocation.invoke_async::<i64>(&mut connection).await {
            tracing::warn!(error = %e, "Failed to {what}");
        }
    }
}

#[async_trait]
impl HandledEvents for RedisHandledEvents {
    async fn claim(&self, consumer: &str, event_id: &str) -> Claim {
        let Some(mut connection) = self.connection().await else {
            return Claim::Unavailable;
        };
        let key = Self::key(consumer, event_id);
        let token = format!("pending:{}", uuid::Uuid::new_v4());
        let claimed: Result<Option<String>, _> = redis::cmd("SET")
            .arg(&key)
            .arg(&token)
            .arg("NX")
            .arg("EX")
            .arg(CLAIM_LEASE_SECS)
            .query_async(&mut connection)
            .await;
        match claimed {
            Ok(Some(_)) => Claim::Acquired(token),
            Ok(None) => match connection.get::<_, Option<String>>(&key).await {
                Ok(Some(state)) if state == DONE => Claim::Done,
                Ok(_) => Claim::Busy,
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

    async fn renew(&self, consumer: &str, event_id: &str, token: &str) {
        self.run_if_owned(
            RENEW_IF_OWNED,
            Self::key(consumer, event_id),
            &[token.to_string(), CLAIM_LEASE_SECS.to_string()],
            "renew an event claim",
        )
        .await;
    }

    async fn complete(&self, consumer: &str, event_id: &str, token: &str) {
        self.run_if_owned(
            COMPLETE_IF_OWNED,
            Self::key(consumer, event_id),
            &[token.to_string(), DONE.to_string(), self.ttl_secs.to_string()],
            "remember a handled event",
        )
        .await;
    }

    async fn release(&self, consumer: &str, event_id: &str, token: &str) {
        self.run_if_owned(
            RELEASE_IF_OWNED,
            Self::key(consumer, event_id),
            &[token.to_string()],
            "release an event claim",
        )
        .await;
    }
}
