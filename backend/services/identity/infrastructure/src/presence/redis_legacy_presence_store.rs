use std::sync::Arc;

use async_trait::async_trait;
use domain::user::id::UserId;
use identity::ports::legacy_presence_store::{LegacyPresenceStore, LegacyPresenceStoreError};
use redis::AsyncCommands;
use tokio::sync::Mutex;

const LEGACY_HIDDEN_PREFIX: &str = "activity:hidden:";
const LEGACY_HIDDEN_PATTERN: &str = "activity:hidden:*";

pub struct RedisLegacyPresenceStore {
    client: Option<redis::Client>,
    connection: Arc<Mutex<Option<redis::aio::ConnectionManager>>>,
}

impl RedisLegacyPresenceStore {
    pub fn from_env() -> Self {
        let client = std::env::var("REDIS_URL")
            .ok()
            .filter(|u| !u.is_empty())
            .and_then(|url| match redis::Client::open(url) {
                Ok(client) => Some(client),
                Err(e) => {
                    tracing::error!(error = %e, "Invalid REDIS_URL; legacy presence preferences cannot be imported");
                    None
                }
            });
        if client.is_none() {
            tracing::error!("REDIS_URL is not set; legacy presence preferences cannot be imported");
        }
        Self {
            client,
            connection: Arc::new(Mutex::new(None)),
        }
    }

    async fn connection(&self) -> Result<redis::aio::ConnectionManager, LegacyPresenceStoreError> {
        let Some(client) = &self.client else {
            return Err(LegacyPresenceStoreError::Unavailable("redis is not configured".to_string()));
        };
        let mut guard = self.connection.lock().await;
        if let Some(connection) = guard.as_ref() {
            return Ok(connection.clone());
        }
        let connection = client
            .get_connection_manager()
            .await
            .map_err(|e| LegacyPresenceStoreError::Unavailable(e.to_string()))?;
        *guard = Some(connection.clone());
        Ok(connection)
    }

    fn key(user_id: &UserId) -> String {
        format!("{LEGACY_HIDDEN_PREFIX}{}", user_id.uuid())
    }
}

#[async_trait]
impl LegacyPresenceStore for RedisLegacyPresenceStore {
    async fn legacy_hidden_users(&self) -> Result<Vec<UserId>, LegacyPresenceStoreError> {
        let mut connection = self.connection().await?;
        let mut users = Vec::new();
        let mut cursor: u64 = 0;
        loop {
            let (next, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(LEGACY_HIDDEN_PATTERN)
                .arg("COUNT")
                .arg(500)
                .query_async(&mut connection)
                .await
                .map_err(|e| LegacyPresenceStoreError::Unavailable(e.to_string()))?;
            users.extend(
                keys.iter()
                    .filter_map(|key| key.strip_prefix(LEGACY_HIDDEN_PREFIX))
                    .filter_map(|raw| uuid::Uuid::parse_str(raw).ok())
                    .map(UserId::new),
            );
            if next == 0 {
                return Ok(users);
            }
            cursor = next;
        }
    }

    async fn legacy_hidden(&self, user_id: &UserId) -> Result<bool, LegacyPresenceStoreError> {
        let mut connection = self.connection().await?;
        connection
            .exists(Self::key(user_id))
            .await
            .map_err(|e| LegacyPresenceStoreError::Unavailable(e.to_string()))
    }

    async fn forget_legacy_hidden(&self, user_id: &UserId) -> Result<(), LegacyPresenceStoreError> {
        let mut connection = self.connection().await?;
        let _: () = connection
            .del(Self::key(user_id))
            .await
            .map_err(|e| LegacyPresenceStoreError::Unavailable(e.to_string()))?;
        Ok(())
    }
}
