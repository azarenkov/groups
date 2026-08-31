use std::sync::Arc;

use domain::user::id::UserId;
use messaging_domain::privacy::{PrivacyRepository, PrivacySettings};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum GetPrivacyError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct GetPrivacy<P: PrivacyRepository> {
    privacy_repository: Arc<P>,
}

impl<P: PrivacyRepository> GetPrivacy<P> {
    pub fn new(privacy_repository: Arc<P>) -> Self {
        Self { privacy_repository }
    }

    pub async fn execute(&self, user_id: UserId) -> Result<PrivacySettings, GetPrivacyError> {
        Ok(self.privacy_repository.get(&user_id).await?)
    }
}
