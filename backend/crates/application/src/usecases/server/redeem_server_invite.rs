use std::sync::Arc;

use domain::{
    server::repository::ServerRepository,
    shared::repository::error::RepositoryError, user::id::UserId,
};
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum RedeemServerInviteError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Invite code is invalid or revoked")]
    InvalidCode,
    #[error("Already a member of this server")]
    AlreadyMember,
}

pub struct RedeemServerInvite<T>
where
    T: ServerRepository,
{
    server_repository: Arc<T>,
}

impl<T> RedeemServerInvite<T>
where
    T: ServerRepository,
{
    pub fn new(server_repository: Arc<T>) -> Self {
        Self { server_repository }
    }

    pub async fn execute(
        &self,
        code: &str,
        user_id: UserId,
    ) -> Result<Uuid, RedeemServerInviteError> {
        let invite = self
            .server_repository
            .find_invite_by_code(code.trim())
            .await?
            .filter(|i| i.is_active())
            .ok_or(RedeemServerInviteError::InvalidCode)?;

        let server_id = invite.server_id;

        let already_member = self
            .server_repository
            .is_server_member(&server_id, &user_id)
            .await?;
        if already_member {
            return Err(RedeemServerInviteError::AlreadyMember);
        }

        self.server_repository
            .save_new_member_server(&server_id, &user_id)
            .await?;

        Ok(server_id.uuid())
    }
}
