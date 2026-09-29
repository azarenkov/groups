use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity::ports::captcha::{CaptchaStore, CaptchaStoreError};
use redis::AsyncCommands;
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

const INCREMENT_WITH_WINDOW: &str = r#"
local count = redis.call('INCR', KEYS[1])
if count == 1 then
  redis.call('EXPIRE', KEYS[1], ARGV[1])
end
return count
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

    async fn login_failures(&self, keys: &[String]) -> Result<u32, CaptchaStoreError> {
        if keys.is_empty() {
            return Ok(0);
        }
        let mut connection = self.connection().await?;
        let redis_keys: Vec<String> = keys.iter().map(|k| Self::failure_key(k)).collect();
        let counts: Vec<Option<u32>> = connection.mget(redis_keys).await.map_err(unavailable)?;
        Ok(counts.into_iter().flatten().max().unwrap_or(0))
    }

    async fn record_login_failure(&self, keys: &[String]) -> Result<(), CaptchaStoreError> {
        let mut connection = self.connection().await?;
        let script = redis::Script::new(INCREMENT_WITH_WINDOW);
        for key in keys {
            let _: u32 = script
                .key(Self::failure_key(key))
                .arg(self.login_window.as_secs().max(1))
                .invoke_async(&mut connection)
                .await
                .map_err(unavailable)?;
        }
        Ok(())
    }

    async fn clear_login_failures(&self, keys: &[String]) -> Result<(), CaptchaStoreError> {
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
    async fn counts_and_clears_login_failures() {
        let Some(store) = store() else { return };
        let email = format!("email:{}", uuid::Uuid::new_v4());
        let ip = format!("ip:{}", uuid::Uuid::new_v4());
        let keys = vec![email.clone(), ip.clone()];
        assert_eq!(store.login_failures(&keys).await.unwrap(), 0);
        store.record_login_failure(&keys).await.unwrap();
        store.record_login_failure(&[ip.clone()]).await.unwrap();
        assert_eq!(store.login_failures(&keys).await.unwrap(), 2);
        assert_eq!(store.login_failures(&[email.clone()]).await.unwrap(), 1);
        store.clear_login_failures(&keys).await.unwrap();
        assert_eq!(store.login_failures(&keys).await.unwrap(), 0);
    }
}
