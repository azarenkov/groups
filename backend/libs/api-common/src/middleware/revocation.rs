use std::{sync::Arc, time::Duration};

use redis::AsyncCommands;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum RevocationError {
    #[error("access token revocation store is not configured")]
    Disabled,
    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),
}

const SET_MAX_SCRIPT: &str = r#"
local current = tonumber(redis.call('GET', KEYS[1]))
local proposed = tonumber(ARGV[1])
if current == nil or proposed > current then
  redis.call('SET', KEYS[1], proposed, 'EX', ARGV[2])
  return proposed
end
redis.call('EXPIRE', KEYS[1], ARGV[2])
return current
"#;

#[derive(Clone)]
pub struct AccessRevocations {
    client: Option<redis::Client>,
    connection: Arc<Mutex<Option<redis::aio::ConnectionManager>>>,
    explicitly_disabled: bool,
}

impl AccessRevocations {
    pub fn disabled() -> Self {
        Self {
            client: None,
            connection: Arc::new(Mutex::new(None)),
            explicitly_disabled: true,
        }
    }

    fn unconfigured() -> Self {
        Self {
            client: None,
            connection: Arc::new(Mutex::new(None)),
            explicitly_disabled: false,
        }
    }

    pub fn new(redis_url: &str) -> Result<Self, redis::RedisError> {
        Ok(Self {
            client: Some(redis::Client::open(redis_url)?),
            connection: Arc::new(Mutex::new(None)),
            explicitly_disabled: false,
        })
    }

    pub async fn connect(redis_url: &str) -> Result<Self, redis::RedisError> {
        let store = Self::new(redis_url)?;
        if let Err(e) = store.connection().await {
            tracing::warn!(error = %e, "Redis not reachable yet; access token revocation will connect lazily");
        }
        Ok(store)
    }

    pub async fn from_env() -> Self {
        if std::env::var("ACCESS_REVOCATION_DISABLED").is_ok_and(|v| v == "1" || v == "true") {
            tracing::warn!("ACCESS_REVOCATION_DISABLED is set; access token revocation is off");
            return Self::disabled();
        }
        let redis_url = std::env::var("REDIS_URL").ok().filter(|u| !u.is_empty());
        match redis_url {
            Some(url) => match Self::connect(&url).await {
                Ok(store) => store,
                Err(e) => {
                    tracing::error!(error = %e, "Invalid REDIS_URL; access token revocation checks fail closed");
                    Self::unconfigured()
                }
            },
            None => {
                tracing::error!("REDIS_URL is not set; access token revocation checks fail closed");
                Self::unconfigured()
            }
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.client.is_some()
    }

    async fn connection(&self) -> Result<redis::aio::ConnectionManager, RevocationError> {
        let Some(client) = &self.client else {
            return Err(RevocationError::Disabled);
        };
        let mut guard = self.connection.lock().await;
        if let Some(connection) = guard.as_ref() {
            return Ok(connection.clone());
        }
        let connection = client.get_connection_manager().await?;
        *guard = Some(connection.clone());
        Ok(connection)
    }

    fn key(user_id: &Uuid) -> String {
        format!("auth:revoked_before:{user_id}")
    }

    pub async fn revoke_issued_before(
        &self,
        user_id: &Uuid,
        cutoff_unix: i64,
        ttl: Duration,
    ) -> Result<(), RevocationError> {
        let mut connection = self.connection().await?;
        let _: i64 = redis::Script::new(SET_MAX_SCRIPT)
            .key(Self::key(user_id))
            .arg(cutoff_unix)
            .arg(ttl.as_secs().max(1))
            .invoke_async(&mut connection)
            .await?;
        Ok(())
    }

    pub async fn is_revoked(&self, user_id: &Uuid, issued_at_unix: i64) -> Result<bool, RevocationError> {
        if self.explicitly_disabled {
            return Ok(false);
        }
        let mut connection = self.connection().await?;
        let cutoff: Option<i64> = connection.get(Self::key(user_id)).await?;
        Ok(cutoff.is_some_and(|cutoff| issued_at_unix <= cutoff))
    }
}
