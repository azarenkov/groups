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
    #[serde(default)]
    as_of: i64,
}

fn default_true() -> bool {
    true
}


const SET_IF_NEWER: &str = r#"
local current = redis.call('GET', KEYS[1])
if current then
  local as_of = cjson.decode(current)['as_of']
  if as_of and tonumber(as_of) > tonumber(ARGV[2]) then
    return 0
  end
end
redis.call('SET', KEYS[1], ARGV[1], 'EX', ARGV[3])
return 1
"#;

const CLEAR_IF_NOT_NEWER: &str = r#"
local current = redis.call('GET', KEYS[1])
if not current then
  return 0
end
local as_of = cjson.decode(current)['as_of']
if as_of and tonumber(as_of) > tonumber(ARGV[1]) then
  return 0
end
return redis.call('DEL', KEYS[1])
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
            as_of: Self::version(activity.refreshed_at),
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

    fn version(at: DateTime<Utc>) -> i64 {
        at.timestamp_micros()
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

    async fn set_if_newer(
        &self,
        user_id: &UserId,
        activity: &StoredActivity,
        ttl: Duration,
    ) -> Result<bool, ActivityStoreError> {
        let mut connection = self.connection().await?;
        let stored: i64 = redis::Script::new(SET_IF_NEWER)
            .key(Self::key(user_id))
            .arg(Self::encode(activity)?)
            .arg(Self::version(activity.refreshed_at))
            .arg(ttl.as_secs().max(1))
            .invoke_async(&mut connection)
            .await
            .map_err(|e| ActivityStoreError::Unavailable(e.to_string()))?;
        Ok(stored == 1)
    }

    async fn clear_if_not_newer(&self, user_id: &UserId, as_of: DateTime<Utc>) -> Result<bool, ActivityStoreError> {
        let mut connection = self.connection().await?;
        let removed: i64 = redis::Script::new(CLEAR_IF_NOT_NEWER)
            .key(Self::key(user_id))
            .arg(Self::version(as_of))
            .invoke_async(&mut connection)
            .await
            .map_err(|e| ActivityStoreError::Unavailable(e.to_string()))?;
        Ok(removed == 1)
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
