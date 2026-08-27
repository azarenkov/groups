use std::sync::Arc;

use chrono::{Duration, Utc};
use messaging_domain::{server::id::ServerId, server::invite_code::InviteCode, server::policy::ModerationPolicy, server::policy::PolicyError, server::repository::ServerRepository, server::unit_of_work::ServerUnitOfWork};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
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
    #[error("Missing permission to manage invites")]
    NotAllowed,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

impl From<PolicyError> for CreateInviteCodeError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::NotMember => CreateInviteCodeError::NotMember,
            _ => CreateInviteCodeError::NotAllowed,
        }
    }
}

pub struct CreateInviteCode<W: ServerUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: ServerUnitOfWork> CreateInviteCode<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        user_id: UserId,
        expires_days: Option<i64>,
        max_uses: Option<i32>,
    ) -> Result<InviteCode, CreateInviteCodeError> {
        let expires_at = expires_days
            .filter(|d| *d > 0)
            .map(|d| Utc::now() + Duration::days(d));

        loop {
            let mut tx = self.unit_of_work.begin(&server_id).await?;
            let snapshot = tx.authorization().await?;
            ModerationPolicy::manage_invites(&snapshot, &user_id)?;
            let code = generate_code();
            match tx
                .create_invite_code(&user_id, &code, max_uses, expires_at)
                .await
            {
                Ok(invite) => {
                    tx.commit().await?;
                    return Ok(invite);
                }
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
    #[error("Missing permission to manage invites")]
    NotAllowed,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

impl From<PolicyError> for ListInviteCodesError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::NotMember => ListInviteCodesError::NotMember,
            _ => ListInviteCodesError::NotAllowed,
        }
    }
}

pub struct ListInviteCodes<S: ServerRepository, W: ServerUnitOfWork> {
    server_repository: Arc<S>,
    unit_of_work: Arc<W>,
}

impl<S: ServerRepository, W: ServerUnitOfWork> ListInviteCodes<S, W> {
    pub fn new(server_repository: Arc<S>, unit_of_work: Arc<W>) -> Self {
        Self {
            server_repository,
            unit_of_work,
        }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        user_id: UserId,
    ) -> Result<Vec<InviteCode>, ListInviteCodesError> {
        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let snapshot = tx.authorization().await?;
        ModerationPolicy::manage_invites(&snapshot, &user_id)?;
        drop(tx);
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

pub struct RevokeInviteCode<W: ServerUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: ServerUnitOfWork> RevokeInviteCode<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        code: String,
        user_id: UserId,
    ) -> Result<(), RevokeInviteCodeError> {
        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let snapshot = tx.authorization().await?;
        if !snapshot.is_member(&user_id) {
            return Err(RevokeInviteCodeError::NotFound);
        }
        let any_creator = ModerationPolicy::can_revoke_any_invite(&snapshot, &user_id);
        if !tx.revoke_invite_code(&code, &user_id, any_creator).await? {
            return Err(RevokeInviteCodeError::NotFound);
        }
        tx.commit().await?;
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
    ) -> Result<messaging_domain::server::Server, RedeemInviteCodeError> {
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
