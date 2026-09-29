use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity::ports::captcha::{CaptchaStore, CaptchaStoreError};
use redis::AsyncCommands;
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

const REGISTER_ATTEMPT: &str = r#"
local max = 0
for _, key in ipairs(KEYS) do
  local count = redis.call('INCR', key)
  if count == 1 then
    redis.call('EXPIRE', key, ARGV[1])
  end
  if count > max then
    max = count
  end
end
return max
"#;

const RELEASE_ATTEMPT: &str = r#"
for _, key in ipairs(KEYS) do
  local count = tonumber(redis.call('GET', key))
  if count ~= nil and count > 0 then
    redis.call('DECR', key)
  end
end
return 0
"#;

pub struct RedisCaptchaStore {
    client: Option<redis::Client>,
    connection: Arc<Mutex<Option<redis::aio::ConnectionManager>>>,
    login_window: Duration,
}

impl RedisCaptchaStore {
    pub fn from_env(login_window: Duration) -> Self {
        let client = std::env::var("REDIS_URL")
            .ok()
            .filter(|u| !u.is_empty())
            .and_then(|url| match redis::Client::open(url) {
                Ok(client) => Some(client),
                Err(e) => {
                    tracing::error!(error = %e, "Invalid REDIS_URL; captcha checks fail closed");
                    None
                }
            });
        Self {
            client,
            connection: Arc::new(Mutex::new(None)),
            login_window,
        }
    }

    async fn connection(&self) -> Result<redis::aio::ConnectionManager, CaptchaStoreError> {
        let Some(client) = &self.client else {
            return Err(CaptchaStoreError::Unavailable("redis is not configured".to_string()));
        };
        let mut guard = self.connection.lock().await;
        if let Some(connection) = guard.as_ref() {
            return Ok(connection.clone());
        }
        let connection = client
            .get_connection_manager()
            .await
            .map_err(unavailable)?;
        *guard = Some(connection.clone());
        Ok(connection)
    }

    fn used_key(signature: &str) -> String {
        format!("captcha:used:{signature}")
    }

    fn failure_key(key: &str) -> String {
        format!("auth:failures:{}", hex::encode(Sha256::digest(key.as_bytes())))
    }
}

fn unavailable(e: redis::RedisError) -> CaptchaStoreError {
    CaptchaStoreError::Unavailable(e.to_string())
}

#[async_trait]
impl CaptchaStore for RedisCaptchaStore {
    async fn claim_solution(&self, signature: &str, expires_at: DateTime<Utc>) -> Result<bool, CaptchaStoreError> {
        let ttl = (expires_at - Utc::now()).num_seconds().max(1) as u64;
        let mut connection = self.connection().await?;
        let claimed: Option<String> = redis::cmd("SET")
            .arg(Self::used_key(signature))
            .arg(1)
            .arg("NX")
            .arg("EX")
            .arg(ttl)
            .query_async(&mut connection)
            .await
            .map_err(unavailable)?;
        Ok(claimed.is_some())
    }

    async fn register_login_attempt(&self, keys: &[String]) -> Result<u32, CaptchaStoreError> {
        if keys.is_empty() {
            return Ok(0);
        }
        let mut connection = self.connection().await?;
        let script = redis::Script::new(REGISTER_ATTEMPT);
        let mut invocation = script.prepare_invoke();
        for key in keys {
            invocation.key(Self::failure_key(key));
        }
        invocation
            .arg(self.login_window.as_secs().max(1))
            .invoke_async(&mut connection)
            .await
            .map_err(unavailable)
    }

    async fn release_login_attempt(&self, keys: &[String]) -> Result<(), CaptchaStoreError> {
        if keys.is_empty() {
            return Ok(());
        }
        let mut connection = self.connection().await?;
        let script = redis::Script::new(RELEASE_ATTEMPT);
        let mut invocation = script.prepare_invoke();
        for key in keys {
            invocation.key(Self::failure_key(key));
        }
        let _: u32 = invocation
            .invoke_async(&mut connection)
            .await
            .map_err(unavailable)?;
        Ok(())
    }

    async fn clear_login_attempts(&self, keys: &[String]) -> Result<(), CaptchaStoreError> {
        if keys.is_empty() {
            return Ok(());
        }
        let mut connection = self.connection().await?;
        let redis_keys: Vec<String> = keys.iter().map(|k| Self::failure_key(k)).collect();
        let _: u32 = connection.del(redis_keys).await.map_err(unavailable)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> Option<RedisCaptchaStore> {
        let url = std::env::var("CAPTCHA_REDIS_TEST_URL").ok()?;
        Some(RedisCaptchaStore {
            client: Some(redis::Client::open(url).ok()?),
            connection: Arc::new(Mutex::new(None)),
            login_window: Duration::from_secs(60),
        })
    }

    #[tokio::test]
    async fn claims_a_solution_once() {
        let Some(store) = store() else { return };
        let signature = uuid::Uuid::new_v4().to_string();
        let expires_at = Utc::now() + chrono::Duration::seconds(30);
        assert!(store.claim_solution(&signature, expires_at).await.unwrap());
        assert!(!store.claim_solution(&signature, expires_at).await.unwrap());
    }

    #[tokio::test]
    async fn registers_releases_and_clears_login_attempts() {
        let Some(store) = store() else { return };
        let email = format!("email:{}", uuid::Uuid::new_v4());
        let ip = format!("ip:{}", uuid::Uuid::new_v4());
        let keys = vec![email.clone(), ip.clone()];
        assert_eq!(store.register_login_attempt(&keys).await.unwrap(), 1);
        assert_eq!(store.register_login_attempt(std::slice::from_ref(&ip)).await.unwrap(), 2);
        assert_eq!(store.register_login_attempt(&keys).await.unwrap(), 3);
        store.release_login_attempt(std::slice::from_ref(&ip)).await.unwrap();
        store.clear_login_attempts(std::slice::from_ref(&email)).await.unwrap();
        assert_eq!(store.register_login_attempt(&keys).await.unwrap(), 3);
        store.clear_login_attempts(&keys).await.unwrap();
        store.release_login_attempt(&keys).await.unwrap();
        assert_eq!(store.register_login_attempt(&keys).await.unwrap(), 1);
        store.clear_login_attempts(&keys).await.unwrap();
    }
}
