use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ports::repository::RepositoryError;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresencePreference {
    pub appear_offline: bool,
    pub as_of: DateTime<Utc>,
}

#[async_trait]
pub trait RecipientDirectory: Send + Sync {
    async fn community_members(&self, community_id: Uuid) -> Result<Vec<Uuid>, RepositoryError>;
    async fn user_name(&self, user_id: Uuid) -> Result<Option<String>, RepositoryError>;
    async fn user_name_versioned(&self, user_id: Uuid) -> Result<Option<(String, DateTime<Utc>)>, RepositoryError>;
    async fn rename_recipients(&self, user_id: Uuid) -> Result<Vec<Uuid>, RepositoryError>;
    async fn profile_recipients(&self, user_id: Uuid) -> Result<Vec<Uuid>, RepositoryError>;
    async fn interested_in(&self, user_id: Uuid) -> Result<Vec<Uuid>, RepositoryError>;
    async fn pending_dm_peers(&self, user_id: Uuid) -> Result<Vec<Uuid>, RepositoryError>;
    async fn presence_preferences(&self, user_ids: &[Uuid]) -> Result<Vec<(Uuid, PresencePreference)>, RepositoryError>;
}
