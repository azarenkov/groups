use std::sync::Arc;

use chrono::{Duration, Utc};
use domain::user::id::UserId;
use messaging_domain::{server::Server, server::id::ServerId, server::invite_link::InviteJoinOutcome, server::invite_link::InviteLink, server::invite_link::InviteLinkStatus, server::invite_link::InviteToken, server::policy::PolicyError, server::repository::ServerRepository, server::unit_of_work::ServerTransaction, server::unit_of_work::ServerUnitOfWork};
use ports::repository::RepositoryError;
use uuid::Uuid;

const TOKEN_ATTEMPTS: usize = 5;

pub async fn create_with_fresh_token(
    tx: &mut dyn ServerTransaction,
    build: impl Fn() -> InviteLink,
) -> Result<InviteLink, RepositoryError> {
    for _ in 0..TOKEN_ATTEMPTS {
        let link = build();
        match tx.create_invite_link(&link).await {
            Ok(()) => return Ok(link),
            Err(RepositoryError::AlreadyExists) => continue,
            Err(e) => return Err(e),
        }
    }
    Err(RepositoryError::AlreadyExists)
}

#[derive(Debug, thiserror::Error)]
pub enum ManageInviteLinksError {
    #[error("Not a member of this server")]
    NotMember,
    #[error("Missing permission to manage invites")]
    NotAllowed,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

impl From<PolicyError> for ManageInviteLinksError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::NotMember => ManageInviteLinksError::NotMember,
            _ => ManageInviteLinksError::NotAllowed,
        }
    }
}

pub struct CreateInviteLink<W: ServerUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: ServerUnitOfWork> CreateInviteLink<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        user_id: UserId,
        name: Option<String>,
        expires_days: Option<i64>,
        max_uses: Option<i32>,
    ) -> Result<InviteLink, ManageInviteLinksError> {
        let expires_at = expires_days
            .filter(|d| *d > 0)
            .map(|d| Utc::now() + Duration::days(d));
        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let server = tx.load_server().await?;
        server.ensure_can_manage_invites(&user_id)?;
        let link = create_with_fresh_token(&mut *tx, || {
            InviteLink::new(server_id, user_id, name.clone(), false, max_uses, expires_at, Utc::now())
        })
        .await?;
        tx.commit().await?;
        Ok(link)
    }
}

pub struct ListInviteLinks<S: ServerRepository, W: ServerUnitOfWork> {
    server_repository: Arc<S>,
    unit_of_work: Arc<W>,
}

impl<S: ServerRepository, W: ServerUnitOfWork> ListInviteLinks<S, W> {
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
    ) -> Result<Vec<InviteLink>, ManageInviteLinksError> {
        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let server = tx.load_server().await?;
        server.ensure_can_manage_invites(&user_id)?;
        if tx.find_primary_invite_link().await?.is_none() {
            let owner = *server.owner_id();
            create_with_fresh_token(&mut *tx, || InviteLink::primary(server_id, owner, Utc::now()))
                .await?;
        }
        tx.commit().await?;
        Ok(self.server_repository.list_invite_links(&server_id).await?)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RevokeInviteLinkError {
    #[error("Invite link not found")]
    NotFound,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct RevokeInviteLink<W: ServerUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: ServerUnitOfWork> RevokeInviteLink<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        server_id: ServerId,
        token: InviteToken,
        user_id: UserId,
    ) -> Result<(), RevokeInviteLinkError> {
        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let server = tx.load_server().await?;
        if !server.is_member(&user_id) {
            return Err(RevokeInviteLinkError::NotFound);
        }
        let any_creator = server.can_revoke_any_invite(&user_id);
        let owner = *server.owner_id();
        let revoked = tx
            .revoke_invite_link(&token, &user_id, any_creator)
            .await?
            .ok_or(RevokeInviteLinkError::NotFound)?;
        if revoked.is_primary {
            create_with_fresh_token(&mut *tx, || InviteLink::primary(server_id, owner, Utc::now()))
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct InvitePreviewServer {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub accent_color: Option<String>,
    pub member_count: i64,
}

#[derive(Debug, Clone)]
pub struct InvitePreview {
    pub status: InviteLinkStatus,
    pub server: Option<InvitePreviewServer>,
}

#[derive(Debug, thiserror::Error)]
pub enum PreviewInviteLinkError {
    #[error("Invite link not found")]
    NotFound,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct PreviewInviteLink<S: ServerRepository> {
    server_repository: Arc<S>,
}

impl<S: ServerRepository> PreviewInviteLink<S> {
    pub fn new(server_repository: Arc<S>) -> Self {
        Self { server_repository }
    }

    pub async fn execute(&self, token: InviteToken) -> Result<InvitePreview, PreviewInviteLinkError> {
        let link = self
            .server_repository
            .find_invite_link(&token)
            .await?
            .ok_or(PreviewInviteLinkError::NotFound)?;
        let status = link.status(Utc::now());
        if status != InviteLinkStatus::Active {
            return Ok(InvitePreview { status, server: None });
        }
        let server = self
            .server_repository
            .find_server_by_id(&link.server_id)
            .await?
            .ok_or(PreviewInviteLinkError::NotFound)?;
        let member_count = self.server_repository.count_members(&link.server_id).await?;
        Ok(InvitePreview {
            status,
            server: Some(InvitePreviewServer {
                id: server.id().uuid(),
                name: server.name().as_str().to_owned(),
                description: server.description().map(|d| d.as_str().to_owned()),
                accent_color: server.accent_color().map(|s| s.to_owned()),
                member_count,
            }),
        })
    }
}

#[derive(Debug)]
pub struct InviteJoin {
    pub server: Server,
    pub already_member: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum JoinByInviteLinkError {
    #[error("Invite link not found")]
    NotFound,
    #[error("Invite link is not available")]
    Unavailable(InviteLinkStatus),
    #[error("Server not found")]
    ServerNotFound,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct JoinByInviteLink<S: ServerRepository> {
    server_repository: Arc<S>,
}

impl<S: ServerRepository> JoinByInviteLink<S> {
    pub fn new(server_repository: Arc<S>) -> Self {
        Self { server_repository }
    }

    pub async fn execute(
        &self,
        token: InviteToken,
        user_id: UserId,
    ) -> Result<InviteJoin, JoinByInviteLinkError> {
        let (server_id, already_member) = match self
            .server_repository
            .join_by_invite_link(&token, &user_id)
            .await?
        {
            InviteJoinOutcome::Joined(server_id) => (server_id, false),
            InviteJoinOutcome::AlreadyMember(server_id) => (server_id, true),
            InviteJoinOutcome::Unavailable(status) => {
                return Err(JoinByInviteLinkError::Unavailable(status));
            }
            InviteJoinOutcome::NotFound => return Err(JoinByInviteLinkError::NotFound),
        };
        let server = self
            .server_repository
            .find_server_by_id(&server_id)
            .await?
            .ok_or(JoinByInviteLinkError::ServerNotFound)?;
        Ok(InviteJoin {
            server,
            already_member,
        })
    }
}
