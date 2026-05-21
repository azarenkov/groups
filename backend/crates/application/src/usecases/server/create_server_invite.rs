use std::sync::Arc;

use chrono::Utc;
use domain::{
    server::{id::ServerId, invite::ServerInvite, repository::ServerRepository},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

use crate::dto::responses::server::server_invite::ServerInviteDto;

#[derive(Debug, thiserror::Error)]
pub enum CreateServerInviteError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not a member of the server")]
    NotMember,
}

pub struct CreateServerInvite<T>
where
    T: ServerRepository,
{
    server_repository: Arc<T>,
}

impl<T> CreateServerInvite<T>
where
    T: ServerRepository,
{
    pub fn new(server_repository: Arc<T>) -> Self {
        Self { server_repository }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        requester_id: UserId,
    ) -> Result<ServerInviteDto, CreateServerInviteError> {
        let is_member = self
            .server_repository
            .is_server_member(&server_id, &requester_id)
            .await?;
        if !is_member {
            return Err(CreateServerInviteError::NotMember);
        }

        let mut invite = ServerInvite::create(server_id, requester_id, Utc::now());
        for _ in 0..5 {
            match self.server_repository.create_invite(&invite).await {
                Ok(()) => return Ok(ServerInviteDto::from(invite)),
                Err(RepositoryError::AlreadyExists) => {
                    invite = ServerInvite::create(server_id, requester_id, Utc::now());
                }
                Err(e) => return Err(CreateServerInviteError::Repository(e)),
            }
        }
        Err(CreateServerInviteError::Repository(
            RepositoryError::AlreadyExists,
        ))
    }
}
