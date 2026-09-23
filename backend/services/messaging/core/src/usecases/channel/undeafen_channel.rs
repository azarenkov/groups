use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, channel::ChannelError, channel::id::ChannelId, channel::repository::ChannelRepository, community::id::CommunityId, community::policy::PolicyError, community::unit_of_work::CommunityUnitOfWork};
use domain::user::id::UserId;
use ports::{repository::RepositoryError, user_profile::UserProfileRepository};

use crate::dto::requests::channel::undeafen_channel::UndeafenChannelRequest;

#[derive(Debug, thiserror::Error)]
pub enum UndeafenChannelError {
    #[error("Channel not found")]
    ChannelNotFound,
    #[error("Target user not found")]
    TargetUserNotFound,
    #[error("Voice not supported for this channel")]
    VoiceNotSupported,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Channel domain error: {0}")]
    Domain(#[from] ChannelError),
}

impl From<PolicyError> for UndeafenChannelError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::TargetNotMember => UndeafenChannelError::TargetUserNotFound,
            _ => UndeafenChannelError::Domain(ChannelError::NotAllowed),
        }
    }
}

#[derive(Debug)]
pub struct UndeafenChannel<C, U, W>
where
    C: ChannelRepository,
    U: UserProfileRepository,
    W: CommunityUnitOfWork,
{
    channel_repository: Arc<C>,
    user_repository: Arc<U>,
    unit_of_work: Arc<W>,
}

impl<C, U, W> UndeafenChannel<C, U, W>
where
    C: ChannelRepository,
    U: UserProfileRepository,
    W: CommunityUnitOfWork,
{
    pub fn new(channel_repository: Arc<C>, user_repository: Arc<U>, unit_of_work: Arc<W>) -> Self {
        Self {
            channel_repository,
            user_repository,
            unit_of_work,
        }
    }

    pub async fn execute(
        &self,
        request: UndeafenChannelRequest,
        initiator: UserId,
    ) -> Result<(), UndeafenChannelError> {
        let channel_id = ChannelId::new(request.channel_id);
        let target = UserId::new(request.target_user_id);

        let channel = self
            .channel_repository
            .find_channel_by_id(&channel_id)
            .await?;

        if channel.community_id().uuid() != request.community_id {
            return Err(UndeafenChannelError::ChannelNotFound);
        }

        if !channel.supports_voice() {
            return Err(UndeafenChannelError::VoiceNotSupported);
        }

        self.user_repository
            .get_name(&target)
            .await?
            .ok_or(UndeafenChannelError::TargetUserNotFound)?;

        let now = Utc::now();
        let community_id = CommunityId::new(request.community_id);
        let mut tx = self.unit_of_work.begin(&community_id).await?;
        let community = tx.load_community().await?;
        let mut channel = tx.load_channel(&channel_id).await?;
        let deafened_by = channel.deafen_record(&target).map(|d| d.deafened_by);
        let grant = community.ensure_can_undeafen(&initiator, &target, deafened_by.as_ref())?;

        channel.undeafen(target, initiator, grant, now)?;
        let events: Vec<MessagingEvent> = channel.pull_events().into_iter().map(MessagingEvent::from).collect();
        tx.save_deafen_state(&channel_id, &target, channel.deafen_record(&target))
            .await?;
        tx.save_events(&events, request.ctx.request_id.as_deref()).await?;
        tx.commit().await?;
        Ok(())
    }
}
