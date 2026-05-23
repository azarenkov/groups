use std::sync::Arc;

use domain::{
    server::{id::ServerId, repository::ServerRepository},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

#[derive(Debug, thiserror::Error)]
pub enum LeaveServerError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("You are not a member of this server")]
    NotMember,
    #[error("Server owners cannot leave their own server; delete it instead")]
    OwnerCannotLeave,
}

pub struct LeaveServer<S>
where
    S: ServerRepository,
{
    server_repository: Arc<S>,
}

impl<S> LeaveServer<S>
where
    S: ServerRepository,
{
    pub fn new(server_repository: Arc<S>) -> Self {
        Self { server_repository }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        user_id: UserId,
    ) -> Result<(), LeaveServerError> {
        let owner = self.server_repository.find_server_owner(&server_id).await?;
        if owner.as_ref() == Some(&user_id) {
            return Err(LeaveServerError::OwnerCannotLeave);
        }
        let removed = self
            .server_repository
            .remove_server_member(&server_id, &user_id)
            .await?;
        if !removed {
            return Err(LeaveServerError::NotMember);
        }
        Ok(())
    }
}
