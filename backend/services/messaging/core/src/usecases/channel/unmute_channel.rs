use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, channel::ChannelError, channel::UnmuteOutcome, channel::id::ChannelId, channel::repository::ChannelRepository, server::id::ServerId, server::policy::PolicyError, server::unit_of_work::ServerUnitOfWork};
use domain::user::id::UserId;
use ports::{repository::RepositoryError, user_profile::UserProfileRepository};

use crate::dto::requests::channel::unmute_channel::UnmuteChannelRequest;

#[derive(Debug, thiserror::Error)]
pub enum UnmuteChannelError {
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

impl From<PolicyError> for UnmuteChannelError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::TargetNotMember => UnmuteChannelError::TargetUserNotFound,
            _ => UnmuteChannelError::Domain(ChannelError::NotOwner),
        }
    }
}

#[derive(Debug)]
pub struct UnmuteChannel<C, U, W>
where
    C: ChannelRepository,
    U: UserProfileRepository,
    W: ServerUnitOfWork,
{
    channel_repository: Arc<C>,
    user_repository: Arc<U>,
    unit_of_work: Arc<W>,
}

impl<C, U, W> UnmuteChannel<C, U, W>
where
    C: ChannelRepository,
    U: UserProfileRepository,
    W: ServerUnitOfWork,
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
        request: UnmuteChannelRequest,
        initiator: UserId,
    ) -> Result<(), UnmuteChannelError> {
        let channel_id = ChannelId::new(request.channel_id);
        let target = UserId::new(request.target_user_id);

        let mut channel = self
            .channel_repository
            .find_channel_by_id(&channel_id)
            .await?;

        if channel.server_id().uuid() != request.server_id {
            return Err(UnmuteChannelError::ChannelNotFound);
        }

        if !channel.supports_voice() {
            return Err(UnmuteChannelError::VoiceNotSupported);
        }

        self.user_repository
            .get_name(&target)
            .await?
            .ok_or(UnmuteChannelError::TargetUserNotFound)?;

        let now = Utc::now();
        let server_id = ServerId::new(request.server_id);
        let observed_muted_by = channel
            .muted_participants_list()
            .find(|m| m.user_id == target)
            .map(|m| m.muted_by);

        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let server = tx.load_server().await?;
        server.ensure_can_unmute(&initiator, &target, observed_muted_by.as_ref())?;
        let can_override = server.is_owner(&initiator) || target != initiator;

        let outcome = channel.unmute(target, initiator, can_override, now)?;
        let downgrade = outcome == UnmuteOutcome::DowngradedToSelfMute;
        let events: Vec<MessagingEvent> = channel.pull_events().into_iter().map(MessagingEvent::from).collect();

        if !tx
            .unmute_participant(&channel_id, &target, observed_muted_by.as_ref(), downgrade)
            .await?
        {
            return Err(UnmuteChannelError::Domain(ChannelError::NotMuted));
        }
        tx.save_events(&events, request.ctx.request_id.as_deref()).await?;
        tx.commit().await?;
        Ok(())
    }
}
