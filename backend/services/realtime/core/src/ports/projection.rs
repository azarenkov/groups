use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ports::repository::RepositoryError;
use uuid::Uuid;

#[async_trait]
pub trait RealtimeProjection: Send + Sync {
    async fn upsert_user_name(&self, user_id: Uuid, name: &str, occurred_at: DateTime<Utc>) -> Result<(), RepositoryError>;
    async fn add_friend_edge(&self, a: Uuid, b: Uuid, occurred_at: DateTime<Utc>) -> Result<(), RepositoryError>;
    async fn add_pending_friend_edge(&self, a: Uuid, b: Uuid, occurred_at: DateTime<Utc>) -> Result<(), RepositoryError>;
    async fn remove_friend_edge(&self, a: Uuid, b: Uuid, occurred_at: DateTime<Utc>) -> Result<(), RepositoryError>;
    async fn apply_block(&self, blocker_id: Uuid, blocked_id: Uuid, active: bool, occurred_at: DateTime<Utc>) -> Result<bool, RepositoryError>;
    async fn record_dm_pair(&self, a: Uuid, b: Uuid) -> Result<(), RepositoryError>;
    async fn remove_dm_pair(&self, a: Uuid, b: Uuid) -> Result<(), RepositoryError>;
    async fn add_community_member(&self, community_id: Uuid, user_id: Uuid, occurred_at: DateTime<Utc>) -> Result<(), RepositoryError>;
    async fn remove_community_member(&self, community_id: Uuid, user_id: Uuid, occurred_at: DateTime<Utc>) -> Result<(), RepositoryError>;
    async fn remove_community(&self, community_id: Uuid, occurred_at: DateTime<Utc>) -> Result<(), RepositoryError>;
}
