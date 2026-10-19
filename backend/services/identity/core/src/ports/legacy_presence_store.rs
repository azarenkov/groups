use async_trait::async_trait;
use domain::user::id::UserId;

#[derive(Debug, thiserror::Error)]
pub enum LegacyPresenceStoreError {
    #[error("Legacy presence store unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait LegacyPresenceStore: Send + Sync {
    async fn legacy_hidden_users(&self) -> Result<Vec<UserId>, LegacyPresenceStoreError>;
    async fn legacy_hidden(&self, user_id: &UserId) -> Result<bool, LegacyPresenceStoreError>;
    async fn forget_legacy_hidden(&self, user_id: &UserId) -> Result<(), LegacyPresenceStoreError>;
}
