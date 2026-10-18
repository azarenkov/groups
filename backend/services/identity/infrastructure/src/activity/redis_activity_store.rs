use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use identity::ports::activity_store::{ActivityStore, ActivityStoreError, StoredActivity};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

const LEGACY_HIDDEN_PREFIX: &str = "activity:hidden:";
const LEGACY_HIDDEN_PATTERN: &str = "activity:hidden:*";

#[derive(Serialize, Deserialize)]
struct ActivityRecord {
    game_id: String,
    name: String,
    started_at: DateTime<Utc>,
    #[serde(default = "default_true")]
    published: bool,
    #[serde(default)]
    source: String,
    #[serde(default = "Utc::now")]
    refreshed_at: DateTime<Utc>,
    #[serde(default = "Utc::now")]
    changed_at: DateTime<Utc>,
}

fn default_true() -> bool {
    true
}


const SET_IF_CURRENT: &str = r#"
if redis.call('GET', KEYS[1]) == ARGV[1] then
  redis.call('SET', KEYS[1], ARGV[2], 'EX', ARGV[3])
  return 1
end
return 0
"#;

pub struct RedisActivityStore {
    client: Option<redis::Client>,
    connection: Arc<Mutex<Option<redis::aio::ConnectionManager>>>,
}

impl RedisActivityStore {
    pub fn from_env() -> Self {
        let client = std::env::var("REDIS_URL")
            .ok()
            .filter(|u| !u.is_empty())
            .and_then(|url| match redis::Client::open(url) {
                Ok(client) => Some(client),
                Err(e) => {
                    tracing::error!(error = %e, "Invalid REDIS_URL; game activity is disabled");
                    None
                }
            });
        if client.is_none() {
            tracing::error!("REDIS_URL is not set; game activity is disabled");
        }
        Self {
            client,
            connection: Arc::new(Mutex::new(None)),
        }
    }

    fn encode(activity: &StoredActivity) -> Result<String, ActivityStoreError> {
        serde_json::to_string(&ActivityRecord {
            game_id: activity.game_id.clone(),
            name: activity.name.clone(),
            started_at: activity.started_at,
            published: activity.published,
            source: activity.source.clone(),
            refreshed_at: activity.refreshed_at,
            changed_at: activity.changed_at,
        })
        .map_err(|e| ActivityStoreError::Unavailable(e.to_string()))
    }

    async fn connection(&self) -> Result<redis::aio::ConnectionManager, ActivityStoreError> {
        let Some(client) = &self.client else {
            return Err(ActivityStoreError::Unavailable("redis is not configured".to_string()));
        };
        let mut guard = self.connection.lock().await;
        if let Some(connection) = guard.as_ref() {
            return Ok(connection.clone());
        }
        let connection = client
            .get_connection_manager()
            .await
            .map_err(|e| ActivityStoreError::Unavailable(e.to_string()))?;
        *guard = Some(connection.clone());
        Ok(connection)
    }

    fn key(user_id: &UserId) -> String {
        format!("activity:{}", user_id.uuid())
    }

    fn decode(raw: Option<String>) -> Option<StoredActivity> {
        let raw = raw?;
        let record: ActivityRecord = serde_json::from_str(&raw).ok()?;
        Some(StoredActivity {
            game_id: record.game_id,
            name: record.name,
            started_at: record.started_at,
            published: record.published,
            source: record.source,
            refreshed_at: record.refreshed_at,
            changed_at: record.changed_at,
        })
    }
}

#[async_trait]
impl ActivityStore for RedisActivityStore {
    async fn get(&self, user_id: &UserId) -> Result<Option<StoredActivity>, ActivityStoreError> {
        let mut connection = self.connection().await?;
        let raw: Option<String> = connection
            .get(Self::key(user_id))
            .await
            .map_err(|e| ActivityStoreError::Unavailable(e.to_string()))?;
        Ok(Self::decode(raw))
    }

    async fn set(
        &self,
        user_id: &UserId,
        activity: &StoredActivity,
        ttl: Duration,
    ) -> Result<Option<StoredActivity>, ActivityStoreError> {
        let mut connection = self.connection().await?;
        let key = Self::key(user_id);
        let previous: Option<String> = connection
            .get(&key)
            .await
            .map_err(|e| ActivityStoreError::Unavailable(e.to_string()))?;
        let payload = Self::encode(activity)?;
        let _: () = connection
            .set_ex(&key, payload, ttl.as_secs().max(1))
            .await
            .map_err(|e| ActivityStoreError::Unavailable(e.to_string()))?;
        Ok(Self::decode(previous))
    }

    async fn set_if_current(
        &self,
        user_id: &UserId,
        expected: &StoredActivity,
        activity: &StoredActivity,
        ttl: Duration,
    ) -> Result<bool, ActivityStoreError> {
        let mut connection = self.connection().await?;
        let replaced: i64 = redis::Script::new(SET_IF_CURRENT)
            .key(Self::key(user_id))
            .arg(Self::encode(expected)?)
            .arg(Self::encode(activity)?)
            .arg(ttl.as_secs().max(1))
            .invoke_async(&mut connection)
            .await
            .map_err(|e| ActivityStoreError::Unavailable(e.to_string()))?;
        Ok(replaced == 1)
    }

    async fn clear(&self, user_id: &UserId) -> Result<Option<StoredActivity>, ActivityStoreError> {
        let mut connection = self.connection().await?;
        let key = Self::key(user_id);
        let previous: Option<String> = connection
            .get(&key)
            .await
            .map_err(|e| ActivityStoreError::Unavailable(e.to_string()))?;
        let _: () = connection
            .del(&key)
            .await
            .map_err(|e| ActivityStoreError::Unavailable(e.to_string()))?;
        Ok(Self::decode(previous))
    }

    async fn get_many(
        &self,
        user_ids: &[UserId],
    ) -> Result<Vec<(UserId, StoredActivity)>, ActivityStoreError> {
        if user_ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut connection = self.connection().await?;
        let keys: Vec<String> = user_ids.iter().map(Self::key).collect();
        let values: Vec<Option<String>> = if keys.len() == 1 {
            let single: Option<String> = connection
                .get(&keys[0])
                .await
                .map_err(|e| ActivityStoreError::Unavailable(e.to_string()))?;
            vec![single]
        } else {
            connection
                .mget(&keys)
                .await
                .map_err(|e| ActivityStoreError::Unavailable(e.to_string()))?
        };
        Ok(user_ids
            .iter()
            .zip(values)
            .filter_map(|(id, raw)| Self::decode(raw).map(|a| (*id, a)))
            .collect())
    }


    async fn legacy_hidden_users(&self) -> Result<Vec<UserId>, ActivityStoreError> {
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
                .map_err(|e| ActivityStoreError::Unavailable(e.to_string()))?;
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

    async fn legacy_hidden(&self, user_id: &UserId) -> Result<bool, ActivityStoreError> {
        let mut connection = self.connection().await?;
        connection
            .exists(format!("{LEGACY_HIDDEN_PREFIX}{}", user_id.uuid()))
            .await
            .map_err(|e| ActivityStoreError::Unavailable(e.to_string()))
    }

    async fn forget_legacy_hidden(&self, user_id: &UserId) -> Result<(), ActivityStoreError> {
        let mut connection = self.connection().await?;
        let _: () = connection
            .del(format!("{LEGACY_HIDDEN_PREFIX}{}", user_id.uuid()))
            .await
            .map_err(|e| ActivityStoreError::Unavailable(e.to_string()))?;
        Ok(())
    }
}
