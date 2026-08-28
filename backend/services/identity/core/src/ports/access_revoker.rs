use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;

#[derive(Debug, thiserror::Error)]
pub enum AccessRevokerError {
    #[error("Access revocation store error: {0}")]
    Store(String),
}

#[async_trait]
pub trait AccessRevoker: Send + Sync {
    async fn revoke_access_issued_before(&self, user_id: &UserId, cutoff: DateTime<Utc>) -> Result<(), AccessRevokerError>;
}
