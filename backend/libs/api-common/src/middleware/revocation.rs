use std::time::Duration;

use redis::AsyncCommands;
use uuid::Uuid;

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

#[derive(Debug, thiserror::Error)]
pub enum RevocationError {
    #[error("access token revocation store is not configured")]
    Disabled,
    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),
}

#[derive(Clone)]
pub struct AccessRevocations {
    connection: Option<redis::aio::ConnectionManager>,
}

impl AccessRevocations {
    pub fn disabled() -> Self {
        Self { connection: None }
    }

    pub async fn connect(redis_url: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(redis_url)?;
        let connection = client.get_connection_manager().await?;
        Ok(Self {
            connection: Some(connection),
        })
    }

    pub async fn from_env() -> Self {
        let redis_url = std::env::var("REDIS_URL").ok().filter(|u| !u.is_empty());
        match redis_url {
            Some(url) => match Self::connect(&url).await {
                Ok(store) => store,
                Err(e) => {
                    tracing::warn!(error = %e, "Redis unavailable; access token revocation disabled");
                    Self::disabled()
                }
            },
            None => {
                tracing::warn!("REDIS_URL is not set; access token revocation disabled");
                Self::disabled()
            }
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.connection.is_some()
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
        let Some(connection) = &self.connection else {
            return Err(RevocationError::Disabled);
        };
        let mut connection = connection.clone();
        let _: i64 = redis::Script::new(SET_MAX_SCRIPT)
            .key(Self::key(user_id))
            .arg(cutoff_unix)
            .arg(ttl.as_secs().max(1))
            .invoke_async(&mut connection)
            .await?;
        Ok(())
    }

    pub async fn is_revoked(&self, user_id: &Uuid, issued_at_unix: i64) -> Result<bool, RevocationError> {
        let Some(connection) = &self.connection else {
            return Ok(false);
        };
        let mut connection = connection.clone();
        let cutoff: Option<i64> = connection.get(Self::key(user_id)).await?;
        Ok(cutoff.is_some_and(|cutoff| issued_at_unix <= cutoff))
    }
}
