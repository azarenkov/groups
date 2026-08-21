use std::sync::Arc;

use chrono::{Duration, Utc};
use domain::{
    server::{id::ServerId, invite_code::InviteCode, repository::ServerRepository},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};
use uuid::Uuid;

const CODE_ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const CODE_LEN: usize = 8;

fn generate_code() -> String {
    let mut value = Uuid::new_v4().as_u128();
    let base = CODE_ALPHABET.len() as u128;
    let mut out = String::with_capacity(CODE_LEN);
    for _ in 0..CODE_LEN {
        out.push(CODE_ALPHABET[(value % base) as usize] as char);
        value /= base;
    }
    out
}

#[derive(Debug, thiserror::Error)]
pub enum CreateInviteCodeError {
    #[error("Not a member of this server")]
    NotMember,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct CreateInviteCode<S: ServerRepository> {
    server_repository: Arc<S>,
}

impl<S: ServerRepository> CreateInviteCode<S> {
    pub fn new(server_repository: Arc<S>) -> Self {
        Self { server_repository }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        user_id: UserId,
        expires_days: Option<i64>,
        max_uses: Option<i32>,
    ) -> Result<InviteCode, CreateInviteCodeError> {
        let is_member = self
            .server_repository
            .is_server_member(&server_id, &user_id)
            .await?;
        if !is_member {
            return Err(CreateInviteCodeError::NotMember);
        }

        let expires_at = expires_days
            .filter(|d| *d > 0)
            .map(|d| Utc::now() + Duration::days(d));

        loop {
            let code = generate_code();
            match self
                .server_repository
                .create_invite_code(&server_id, &user_id, &code, max_uses, expires_at)
                .await
            {
                Ok(invite) => return Ok(invite),
                Err(RepositoryError::AlreadyExists) => continue,
                Err(e) => return Err(e.into()),
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ListInviteCodesError {
    #[error("Not a member of this server")]
    NotMember,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct ListInviteCodes<S: ServerRepository> {
    server_repository: Arc<S>,
}

impl<S: ServerRepository> ListInviteCodes<S> {
    pub fn new(server_repository: Arc<S>) -> Self {
        Self { server_repository }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        user_id: UserId,
    ) -> Result<Vec<InviteCode>, ListInviteCodesError> {
        let is_member = self
            .server_repository
            .is_server_member(&server_id, &user_id)
            .await?;
        if !is_member {
            return Err(ListInviteCodesError::NotMember);
        }
        let invites = self.server_repository.list_invite_codes(&server_id).await?;
        Ok(invites)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RevokeInviteCodeError {
    #[error("Invite not found")]
    NotFound,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct RevokeInviteCode<S: ServerRepository> {
    server_repository: Arc<S>,
}

impl<S: ServerRepository> RevokeInviteCode<S> {
    pub fn new(server_repository: Arc<S>) -> Self {
        Self { server_repository }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        code: String,
        user_id: UserId,
    ) -> Result<(), RevokeInviteCodeError> {
        let owner = self.server_repository.find_server_owner(&server_id).await?;
        let is_owner = owner
            .map(|o| o.uuid() == user_id.uuid())
            .unwrap_or(false);
        let revoked = self
            .server_repository
            .revoke_invite_code(&server_id, &code, &user_id, is_owner)
            .await?;
        if !revoked {
            return Err(RevokeInviteCodeError::NotFound);
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RedeemInviteCodeError {
    #[error("Invalid or expired invite code")]
    InvalidCode,
    #[error("Server not found")]
    ServerNotFound,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct RedeemInviteCode<S: ServerRepository> {
    server_repository: Arc<S>,
}

impl<S: ServerRepository> RedeemInviteCode<S> {
    pub fn new(server_repository: Arc<S>) -> Self {
        Self { server_repository }
    }

    pub async fn execute(
        &self,
        code: String,
        user_id: UserId,
    ) -> Result<domain::server::Server, RedeemInviteCodeError> {
        let server_id = self
            .server_repository
            .redeem_invite_code(code.trim(), &user_id)
            .await?
            .ok_or(RedeemInviteCodeError::InvalidCode)?;
        let server = self
            .server_repository
            .find_server_by_id(&server_id)
            .await?
            .ok_or(RedeemInviteCodeError::ServerNotFound)?;
        Ok(server)
    }
}
