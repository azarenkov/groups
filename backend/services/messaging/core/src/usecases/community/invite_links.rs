use domain::pagination::{KeysetCursor, Page};
use std::sync::Arc;

use chrono::{Duration, Utc};
use domain::user::id::UserId;
use messaging_domain::{events::MessagingEvent, message::system::SystemMessageKind, community::Community, community::events::CommunityEvent, community::events::lifecycle::CommunityMemberJoined, community::id::CommunityId, community::invite_link::InviteJoinOutcome, community::invite_link::InviteLink, community::invite_link::InviteLinkStatus, community::invite_link::InviteToken, community::policy::PolicyError, community::repository::CommunityRepository, community::unit_of_work::CommunityTransaction, community::unit_of_work::CommunityUnitOfWork};

use crate::usecases::community::membership_announcements::announce_membership;
use ports::repository::RepositoryError;
use uuid::Uuid;

const TOKEN_ATTEMPTS: usize = 5;

pub async fn create_with_fresh_token(
    tx: &mut dyn CommunityTransaction,
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
    #[error("Not a member of this community")]
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

pub struct CreateInviteLink<W: CommunityUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: CommunityUnitOfWork> CreateInviteLink<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        community_id: CommunityId,
        user_id: UserId,
        name: Option<String>,
        expires_days: Option<i64>,
        max_uses: Option<i32>,
    ) -> Result<InviteLink, ManageInviteLinksError> {
        let expires_at = expires_days
            .filter(|d| *d > 0)
            .map(|d| Utc::now() + Duration::days(d));
        let mut tx = self.unit_of_work.begin(&community_id).await?;
        let community = tx.load_community().await?;
        community.ensure_can_manage_invites(&user_id)?;
        let link = create_with_fresh_token(&mut *tx, || {
            InviteLink::new(community_id, user_id, name.clone(), false, max_uses, expires_at, Utc::now())
        })
        .await?;
        tx.commit().await?;
        Ok(link)
    }
}

pub struct InviteLinksPage {
    pub primary: InviteLink,
    pub page: Page<InviteLink>,
}

pub struct ListInviteLinks<S: CommunityRepository, W: CommunityUnitOfWork> {
    community_repository: Arc<S>,
    unit_of_work: Arc<W>,
}

impl<S: CommunityRepository, W: CommunityUnitOfWork> ListInviteLinks<S, W> {
    pub fn new(community_repository: Arc<S>, unit_of_work: Arc<W>) -> Self {
        Self {
            community_repository,
            unit_of_work,
        }
    }

    pub async fn execute(
        &self,
        community_id: CommunityId,
        user_id: UserId,
        cursor: Option<KeysetCursor>,
        limit: i64,
    ) -> Result<InviteLinksPage, ManageInviteLinksError> {
        let mut tx = self.unit_of_work.begin(&community_id).await?;
        let community = tx.load_community().await?;
        community.ensure_can_manage_invites(&user_id)?;
        let primary = match tx.find_primary_invite_link().await? {
            Some(link) => link,
            None => {
                let owner = *community.owner_id();
                create_with_fresh_token(&mut *tx, || InviteLink::primary(community_id, owner, Utc::now()))
                    .await?
            }
        };
        tx.commit().await?;
        let links = self
            .community_repository
            .list_invite_links(&community_id, cursor.as_ref(), limit)
            .await?;
        let page = Page::from_overfetch(links, limit, |link| {
            KeysetCursor::new(link.created_at, link.token.as_str().to_string())
        });
        Ok(InviteLinksPage { primary, page })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RevokeInviteLinkError {
    #[error("Invite link not found")]
    NotFound,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct RevokeInviteLink<W: CommunityUnitOfWork> {
    unit_of_work: Arc<W>,
}

impl<W: CommunityUnitOfWork> RevokeInviteLink<W> {
    pub fn new(unit_of_work: Arc<W>) -> Self {
        Self { unit_of_work }
    }

    pub async fn execute(
        &self,
        community_id: CommunityId,
        token: InviteToken,
        user_id: UserId,
    ) -> Result<(), RevokeInviteLinkError> {
        let mut tx = self.unit_of_work.begin(&community_id).await?;
        let community = tx.load_community().await?;
        if !community.is_member(&user_id) {
            return Err(RevokeInviteLinkError::NotFound);
        }
        let any_creator = community.can_revoke_any_invite(&user_id);
        let owner = *community.owner_id();
        let revoked = tx
            .revoke_invite_link(&token, &user_id, any_creator)
            .await?
            .ok_or(RevokeInviteLinkError::NotFound)?;
        if revoked.is_primary {
            create_with_fresh_token(&mut *tx, || InviteLink::primary(community_id, owner, Utc::now()))
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct InvitePreviewCommunity {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub accent_color: Option<String>,
    pub member_count: i64,
}

#[derive(Debug, Clone)]
pub struct InvitePreview {
    pub status: InviteLinkStatus,
    pub community: Option<InvitePreviewCommunity>,
}

#[derive(Debug, thiserror::Error)]
pub enum PreviewInviteLinkError {
    #[error("Invite link not found")]
    NotFound,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct PreviewInviteLink<S: CommunityRepository> {
    community_repository: Arc<S>,
}

impl<S: CommunityRepository> PreviewInviteLink<S> {
    pub fn new(community_repository: Arc<S>) -> Self {
        Self { community_repository }
    }

    pub async fn execute(&self, token: InviteToken) -> Result<InvitePreview, PreviewInviteLinkError> {
        let link = self
            .community_repository
            .find_invite_link(&token)
            .await?
            .ok_or(PreviewInviteLinkError::NotFound)?;
        let status = link.status(Utc::now());
        if status != InviteLinkStatus::Active {
            return Ok(InvitePreview { status, community: None });
        }
        let community = self
            .community_repository
            .find_community_by_id(&link.community_id)
            .await?
            .ok_or(PreviewInviteLinkError::NotFound)?;
        let member_count = self.community_repository.count_members(&link.community_id).await?;
        Ok(InvitePreview {
            status,
            community: Some(InvitePreviewCommunity {
                id: community.id().uuid(),
                name: community.name().as_str().to_owned(),
                description: community.description().map(|d| d.as_str().to_owned()),
                accent_color: community.accent_color().map(|s| s.to_owned()),
                member_count,
            }),
        })
    }
}

#[derive(Debug)]
pub struct InviteJoin {
    pub community: Community,
    pub already_member: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum JoinByInviteLinkError {
    #[error("Invite link not found")]
    NotFound,
    #[error("Invite link is not available")]
    Unavailable(InviteLinkStatus),
    #[error("Community not found")]
    CommunityNotFound,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct JoinByInviteLink<S: CommunityRepository, W: CommunityUnitOfWork> {
    community_repository: Arc<S>,
    unit_of_work: Arc<W>,
}

impl<S: CommunityRepository, W: CommunityUnitOfWork> JoinByInviteLink<S, W> {
    pub fn new(community_repository: Arc<S>, unit_of_work: Arc<W>) -> Self {
        Self {
            community_repository,
            unit_of_work,
        }
    }

    pub async fn execute(
        &self,
        token: InviteToken,
        user_id: UserId,
    ) -> Result<InviteJoin, JoinByInviteLinkError> {
        let link = self
            .community_repository
            .find_invite_link(&token)
            .await?
            .ok_or(JoinByInviteLinkError::NotFound)?;
        let community_id = link.community_id;
        let mut tx = self.unit_of_work.begin(&community_id).await?;
        let already_member = match tx.redeem_invite_link(&token, &user_id).await? {
            InviteJoinOutcome::Joined(_) => false,
            InviteJoinOutcome::AlreadyMember(_) => true,
            InviteJoinOutcome::Unavailable(status) => {
                return Err(JoinByInviteLinkError::Unavailable(status));
            }
            InviteJoinOutcome::NotFound => return Err(JoinByInviteLinkError::NotFound),
        };
        if !already_member {
            let now = Utc::now();
            let mut events = vec![MessagingEvent::from(CommunityEvent::MemberJoined(
                CommunityMemberJoined::record(community_id, user_id, now),
            ))];
            events.extend(announce_membership(&mut *tx, user_id, SystemMessageKind::MemberJoined, now).await?);
            tx.save_events(&events, None).await?;
        }
        tx.commit().await?;
        let community = self
            .community_repository
            .find_community_by_id(&community_id)
            .await?
            .ok_or(JoinByInviteLinkError::CommunityNotFound)?;
        Ok(InviteJoin {
            community,
            already_member,
        })
    }
}
