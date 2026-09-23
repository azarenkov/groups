use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{privacy::ContactPolicy, privacy::PrivacyRepository, events::MessagingEvent, dm_message::DmMessage, dm_message::repository::DmMessageRepository, friendship::repository::FriendshipRepository, friendship::status::FriendshipStatus, message::MessageError, message::content::MessageContent, community::id::CommunityId, community::policy::PolicyError, community::repository::CommunityRepository, community::unit_of_work::CommunityUnitOfWork};
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use ports::user_profile::UserProfileRepository;

use crate::dto::{RequestContext, responses::dm::send_dm_message::SendDmMessageResponse};

#[derive(Debug, thiserror::Error)]
pub enum SendCommunityInviteError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Message error: {0}")]
    Domain(#[from] MessageError),
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
    #[error("You've already invited this user to this community")]
    InviteAlreadyPending,
}

impl From<PolicyError> for SendCommunityInviteError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::NotMember => SendCommunityInviteError::NotCommunityMember,
            _ => SendCommunityInviteError::NotAllowed,
        }
    }
}

pub struct SendCommunityInvite<M, F, S, W, P, U>
where
    M: DmMessageRepository,
    F: FriendshipRepository,
    S: CommunityRepository,
    W: CommunityUnitOfWork,
    P: PrivacyRepository,
    U: UserProfileRepository,
{
    dm_repository: Arc<M>,
    friendship_repository: Arc<F>,
    community_repository: Arc<S>,
    unit_of_work: Arc<W>,
    privacy_repository: Arc<P>,
    user_profile_repository: Arc<U>,
}

impl<M, F, S, W, P, U> SendCommunityInvite<M, F, S, W, P, U>
where
    M: DmMessageRepository,
    F: FriendshipRepository,
    S: CommunityRepository,
    W: CommunityUnitOfWork,
    P: PrivacyRepository,
    U: UserProfileRepository,
{
    pub fn new(
        dm_repository: Arc<M>,
        friendship_repository: Arc<F>,
        community_repository: Arc<S>,
        unit_of_work: Arc<W>,
        privacy_repository: Arc<P>,
        user_profile_repository: Arc<U>,
    ) -> Self {
        Self {
            dm_repository,
            friendship_repository,
            community_repository,
            unit_of_work,
            privacy_repository,
            user_profile_repository,
        }
    }

    pub async fn execute(
        &self,
        community_id: CommunityId,
        friend_id: UserId,
        sender_id: UserId,
        ctx: RequestContext,
    ) -> Result<SendDmMessageResponse, SendCommunityInviteError> {
        if self
            .friendship_repository
            .block_relation(&sender_id, &friend_id)
            .await?
            .any()
        {
            return Err(SendCommunityInviteError::Blocked);
        }
        let pair = self
            .friendship_repository
            .find_pair(&sender_id, &friend_id)
            .await?;
        if !matches!(pair, Some(ref f) if f.status == FriendshipStatus::Accepted) {
            return Err(SendCommunityInviteError::NotFriends);
        }

        if !self.user_profile_repository.is_active(&friend_id).await? {
            return Err(SendCommunityInviteError::RecipientInactive);
        }

        if matches!(
            self.privacy_repository.get(&friend_id).await?.dm_policy,
            ContactPolicy::NoOne
        ) {
            return Err(SendCommunityInviteError::DmsClosed);
        }

        let community = self
            .community_repository
            .find_community_by_id(&community_id)
            .await?
            .ok_or(SendCommunityInviteError::CommunityNotFound)?;

        if self
            .dm_repository
            .has_pending_community_invite(&sender_id, &friend_id, &community_id)
            .await?
        {
            return Err(SendCommunityInviteError::InviteAlreadyPending);
        }

        let mut tx = self.unit_of_work.begin(&community_id).await?;
        let membership = tx.load_community().await?;
        membership.ensure_can_manage_invites(&sender_id)?;
        if membership.is_member(&friend_id) {
            return Err(SendCommunityInviteError::AlreadyMember);
        }

        let community_name = community.name().as_str().to_string();
        let content = MessageContent::new(community_name.clone())?;
        let now = Utc::now();
        let mut message =
            DmMessage::create_community_invite(sender_id, friend_id, community_id, content, now);

        let request_id = ctx.request_id.clone();
        let events = message.pull_events();
        let events: Vec<MessagingEvent> = events.into_iter().map(MessagingEvent::from).collect();
        let persisted = async {
            tx.lock_pair(&sender_id, &friend_id).await?;
            if tx.block_relation(&sender_id, &friend_id).await?.any() {
                return Err(SendCommunityInviteError::Blocked);
            }
            tx.insert_dm_message(&message).await?;
            tx.save_events(&events, request_id.as_deref()).await?;
            tx.commit().await?;
            Ok::<(), SendCommunityInviteError>(())
        }
        .await;
        match persisted {
            Ok(()) => {}
            Err(SendCommunityInviteError::Repository(RepositoryError::AlreadyExists)) => {
                return Err(SendCommunityInviteError::InviteAlreadyPending);
            }
            Err(e) => return Err(e),
        }

        Ok(SendDmMessageResponse::from_invite(&message, community_name))
    }
}
