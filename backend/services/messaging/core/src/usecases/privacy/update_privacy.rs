use std::sync::Arc;

use domain::user::id::UserId;
use messaging_domain::privacy::{ContactPolicy, PrivacyRepository, PrivacySettings};
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
    pub call_policy: Option<String>,
    pub accept_friend_requests: Option<bool>,
    pub show_local_time: Option<bool>,
}

pub struct UpdatePrivacy<P: PrivacyRepository> {
    privacy_repository: Arc<P>,
}

impl<P: PrivacyRepository> UpdatePrivacy<P> {
    pub fn new(privacy_repository: Arc<P>) -> Self {
        Self { privacy_repository }
    }

    pub async fn execute(&self, user_id: UserId, patch: PrivacyPatch) -> Result<PrivacySettings, UpdatePrivacyError> {
        let dm_policy = patch
            .dm_policy
            .map(|raw| ContactPolicy::parse(&raw).ok_or(UpdatePrivacyError::InvalidPolicy))
            .transpose()?;
        let call_policy = patch
            .call_policy
            .map(|raw| ContactPolicy::parse(&raw).ok_or(UpdatePrivacyError::InvalidPolicy))
            .transpose()?;
        Ok(self
            .privacy_repository
            .apply_patch(&user_id, dm_policy, call_policy, patch.accept_friend_requests, patch.show_local_time)
            .await?)
    }
}
