use std::time::Duration;

use redis::AsyncCommands;
use uuid::Uuid;

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
    ) -> Result<(), redis::RedisError> {
        let Some(connection) = &self.connection else {
            return Ok(());
        };
        let mut connection = connection.clone();
        let _: () = connection
            .set_ex(Self::key(user_id), cutoff_unix, ttl.as_secs().max(1))
            .await?;
        Ok(())
    }

    pub async fn is_revoked(&self, user_id: &Uuid, issued_at_unix: i64) -> bool {
        let Some(connection) = &self.connection else {
            return false;
        };
        let mut connection = connection.clone();
        let cutoff: Result<Option<i64>, redis::RedisError> = connection.get(Self::key(user_id)).await;
        match cutoff {
            Ok(Some(cutoff)) => issued_at_unix <= cutoff,
            Ok(None) => false,
            Err(e) => {
                tracing::warn!(error = %e, "Redis unavailable; skipping access token revocation check");
                false
            }
        }
    }
}
