use std::sync::Arc;

use domain::user::id::UserId;
use identity_domain::user::repository::UserRepository;
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum ListConnectionsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

#[derive(Debug)]
pub struct ConnectionView {
    pub provider: String,
    pub display_name: String,
}

pub struct ListConnections<U: UserRepository> {
    user_repository: Arc<U>,
}

impl<U: UserRepository> ListConnections<U> {
    pub fn new(user_repository: Arc<U>) -> Self {
        Self { user_repository }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
    ) -> Result<Vec<ConnectionView>, ListConnectionsError> {
        Ok(self
            .user_repository
            .list_oauth_connections(&user_id)
            .await?
            .into_iter()
            .filter(|(provider, _)| provider == "github" || provider == "steam")
            .map(|(provider, display_name)| ConnectionView {
                display_name: display_name.unwrap_or_default(),
                provider,
            })
            .collect())
    }
}
