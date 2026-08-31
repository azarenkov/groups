use std::sync::Arc;

use domain::user::id::UserId;
use messaging_domain::privacy::{DmPolicy, PrivacyRepository, PrivacySettings};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum UpdatePrivacyError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Invalid DM policy")]
    InvalidPolicy,
}

pub struct PrivacyPatch {
    pub dm_policy: Option<String>,
    pub accept_friend_requests: Option<bool>,
}

pub struct UpdatePrivacy<P: PrivacyRepository> {
    privacy_repository: Arc<P>,
}

impl<P: PrivacyRepository> UpdatePrivacy<P> {
    pub fn new(privacy_repository: Arc<P>) -> Self {
        Self { privacy_repository }
    }

    pub async fn execute(&self, user_id: UserId, patch: PrivacyPatch) -> Result<PrivacySettings, UpdatePrivacyError> {
        let current = self.privacy_repository.get(&user_id).await?;
        let dm_policy = match patch.dm_policy {
            Some(raw) => DmPolicy::parse(&raw).ok_or(UpdatePrivacyError::InvalidPolicy)?,
            None => current.dm_policy,
        };
        let settings = PrivacySettings {
            dm_policy,
            accept_friend_requests: patch.accept_friend_requests.unwrap_or(current.accept_friend_requests),
        };
        self.privacy_repository.set(&user_id, settings).await?;
        Ok(settings)
    }
}
