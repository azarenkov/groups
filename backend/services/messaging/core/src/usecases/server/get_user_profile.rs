use std::sync::Arc;

use domain::user::id::UserId;
use ports::{repository::RepositoryError, user_profile::PublicProfile, user_profile::UserProfileRepository};

#[derive(Debug, thiserror::Error)]
pub enum GetUserProfileError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("User not found")]
    NotFound,
}

pub struct GetUserProfile<P: UserProfileRepository> {
    profiles: Arc<P>,
}

impl<P: UserProfileRepository> GetUserProfile<P> {
    pub fn new(profiles: Arc<P>) -> Self {
        Self { profiles }
    }

    pub async fn execute(&self, user_id: UserId) -> Result<PublicProfile, GetUserProfileError> {
        self.profiles
            .get_public_profile(&user_id)
            .await?
            .ok_or(GetUserProfileError::NotFound)
    }
}
