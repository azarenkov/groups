use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, channel::ChannelError, channel::id::ChannelId, channel::repository::ChannelRepository, server::id::ServerId, server::policy::PolicyError, server::unit_of_work::ServerUnitOfWork};
use domain::user::id::UserId;
use ports::{repository::RepositoryError, user_profile::UserProfileRepository};

use crate::dto::requests::channel::deafen_channel::DeafenChannelRequest;

#[derive(Debug, thiserror::Error)]
pub enum DeafenChannelError {
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

impl From<PolicyError> for DeafenChannelError {
    fn from(err: PolicyError) -> Self {
        match err {
            PolicyError::TargetNotMember => DeafenChannelError::TargetUserNotFound,
            _ => DeafenChannelError::Domain(ChannelError::NotAllowed),
        }
    }
}

#[derive(Debug)]
pub struct DeafenChannel<C, U, W>
where
    C: ChannelRepository,
    U: UserProfileRepository,
    W: ServerUnitOfWork,
{
    channel_repository: Arc<C>,
    user_repository: Arc<U>,
    unit_of_work: Arc<W>,
}

impl<C, U, W> DeafenChannel<C, U, W>
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
        request: DeafenChannelRequest,
        initiator: UserId,
    ) -> Result<(), DeafenChannelError> {
        let channel_id = ChannelId::new(request.channel_id);
        let target = UserId::new(request.target_user_id);

        let channel = self
            .channel_repository
            .find_channel_by_id(&channel_id)
            .await?;

        if channel.server_id().uuid() != request.server_id {
            return Err(DeafenChannelError::ChannelNotFound);
        }

        if !channel.supports_voice() {
            return Err(DeafenChannelError::VoiceNotSupported);
        }

        self.user_repository
            .get_name(&target)
            .await?
            .ok_or(DeafenChannelError::TargetUserNotFound)?;

        let now = Utc::now();
        let server_id = ServerId::new(request.server_id);
        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let server = tx.load_server().await?;
        let grant = server.ensure_can_deafen(&initiator, &target)?;

        let mut channel = tx.load_channel(&channel_id).await?;
        channel.deafen(target, initiator, grant, now)?;
        let events: Vec<MessagingEvent> = channel.pull_events().into_iter().map(MessagingEvent::from).collect();
        tx.save_deafen_state(&channel_id, &target, channel.deafen_record(&target))
            .await?;
        tx.save_events(&events, request.ctx.request_id.as_deref()).await?;
        tx.commit().await?;
        Ok(())
    }
}
