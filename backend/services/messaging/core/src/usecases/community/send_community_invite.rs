use std::sync::Arc;

use chrono::Utc;
use domain::user::id::UserId;
use messaging_domain::{
    community::{
        id::CommunityId,
        invitation::{Invitation, InvitationError, InvitationRepository},
        unit_of_work::CommunityUnitOfWork,
    },
    events::MessagingEvent,
    friendship::{repository::FriendshipRepository, status::FriendshipStatus},
    privacy::{ContactPolicy, PrivacyRepository},
};
use ports::repository::RepositoryError;
use ports::user_profile::UserProfileRepository;

use crate::dto::RequestContext;

#[derive(Debug, thiserror::Error)]
pub enum SendCommunityInviteError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not friends with this user")]
    NotFriends,
    #[error("This user is blocked")]
    Blocked,
    #[error("This user does not accept direct messages")]
    DmsClosed,
    #[error("This user is unavailable")]
    RecipientInactive,
    #[error("Community not found")]
    CommunityNotFound,
    #[error("Missing permission to manage invites")]
    NotAllowed,
    #[error("Only community members can send invites")]
    NotCommunityMember,
    #[error("This user is already a member of the community")]
    AlreadyMember,
    #[error("This user already has a pending invite to this community")]
    InviteAlreadyPending,
    #[error("You can't invite yourself")]
    SelfInvite,
}

impl From<InvitationError> for SendCommunityInviteError {
    fn from(err: InvitationError) -> Self {
        match err {
            InvitationError::NotMember => Self::NotCommunityMember,
            InvitationError::AlreadyMember => Self::AlreadyMember,
            InvitationError::SelfInvite => Self::SelfInvite,
            InvitationError::NotAllowed | InvitationError::NotInvitee => Self::NotAllowed,
        }
    }
}

pub struct SendCommunityInvite<F, I, W, P, U>
where
    F: FriendshipRepository,
    I: InvitationRepository,
    W: CommunityUnitOfWork,
    P: PrivacyRepository,
    U: UserProfileRepository,
{
    friendship_repository: Arc<F>,
    invitations: Arc<I>,
    unit_of_work: Arc<W>,
    privacy_repository: Arc<P>,
    user_profile_repository: Arc<U>,
}

impl<F, I, W, P, U> SendCommunityInvite<F, I, W, P, U>
where
    F: FriendshipRepository,
    I: InvitationRepository,
    W: CommunityUnitOfWork,
    P: PrivacyRepository,
    U: UserProfileRepository,
{
    pub fn new(
        friendship_repository: Arc<F>,
        invitations: Arc<I>,
        unit_of_work: Arc<W>,
        privacy_repository: Arc<P>,
        user_profile_repository: Arc<U>,
    ) -> Self {
        Self { friendship_repository, invitations, unit_of_work, privacy_repository, user_profile_repository }
    }

    pub async fn execute(
        &self,
        community_id: CommunityId,
        invitee_id: UserId,
        inviter_id: UserId,
        ctx: RequestContext,
    ) -> Result<Invitation, SendCommunityInviteError> {
        if self.friendship_repository.block_relation(&inviter_id, &invitee_id).await?.any() {
            return Err(SendCommunityInviteError::Blocked);
        }
        let pair = self.friendship_repository.find_pair(&inviter_id, &invitee_id).await?;
        if !matches!(pair, Some(ref f) if f.status == FriendshipStatus::Accepted) {
            return Err(SendCommunityInviteError::NotFriends);
        }
        if !self.user_profile_repository.is_active(&invitee_id).await? {
            return Err(SendCommunityInviteError::RecipientInactive);
        }
        if matches!(self.privacy_repository.get(&invitee_id).await?.dm_policy, ContactPolicy::NoOne) {
            return Err(SendCommunityInviteError::DmsClosed);
        }
        if self.invitations.is_pending(&community_id, &invitee_id).await? {
            return Err(SendCommunityInviteError::InviteAlreadyPending);
        }

        let mut tx = self.unit_of_work.begin(&community_id).await?;
        let community = match tx.load_community().await {
            Ok(community) => community,
            Err(RepositoryError::NotFound) => return Err(SendCommunityInviteError::CommunityNotFound),
            Err(e) => return Err(e.into()),
        };
        tx.lock_pair(&inviter_id, &invitee_id).await?;
        if tx.block_relation(&inviter_id, &invitee_id).await?.any() {
            return Err(SendCommunityInviteError::Blocked);
        }
        let (invitation, event) = Invitation::issue(&community, inviter_id, invitee_id, Utc::now())?;
        match tx.insert_invitation(&invitation).await {
            Ok(()) => {}
            Err(RepositoryError::AlreadyExists) => return Err(SendCommunityInviteError::InviteAlreadyPending),
            Err(e) => return Err(e.into()),
        }
        tx.save_events(&[MessagingEvent::from(event)], ctx.request_id.as_deref()).await?;
        tx.commit().await?;
        Ok(invitation)
    }
}
