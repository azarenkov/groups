use std::sync::Arc;

use domain::user::id::UserId;
use identity_domain::user::repository::UserRepository;
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum RemoveConnectionError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Unknown provider")]
    UnknownProvider,
}

pub struct RemoveConnection<U: UserRepository> {
    user_repository: Arc<U>,
}

impl<U: UserRepository> RemoveConnection<U> {
    pub fn new(user_repository: Arc<U>) -> Self {
        Self { user_repository }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        provider: &str,
    ) -> Result<(), RemoveConnectionError> {
        if !matches!(provider, "github" | "steam") {
            return Err(RemoveConnectionError::UnknownProvider);
        }
        self.user_repository
            .unlink_connection(&user_id, provider)
            .await?;
        Ok(())
    }
}
