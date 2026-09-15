use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const STORAGE_KEYS_RELEASED: &str = "storage.keys_released";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageKeysReleasedV1 {
    pub storage_keys: Vec<String>,
    pub occurred_at: DateTime<Utc>,
}
