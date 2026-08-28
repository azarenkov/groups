use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{events::MessagingEvent, channel::ChannelError, channel::repository::ChannelRepository, server::policy::PolicyError, server::unit_of_work::ServerUnitOfWork};
use ports::repository::RepositoryError;

use crate::{
    dto::requests::channel::delete_channel::DeleteChannelRequest,
    ports::voice_service::{VoiceService, VoiceServiceError},
};

#[derive(Debug, thiserror::Error)]
pub enum DeleteChannelError {
    #[error("Channel not found")]
    ChannelNotFound,
    #[error("Only the channel owner can delete it")]
    NotOwner,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Channel domain error: {0}")]
    Domain(#[from] ChannelError),
    #[error("Voice service error: {0}")]
    VoiceService(#[from] VoiceServiceError),
}

impl From<PolicyError> for DeleteChannelError {
    fn from(_: PolicyError) -> Self {
        DeleteChannelError::NotOwner
    }
}

pub struct DeleteChannel<C, V, W>
where
    C: ChannelRepository,
    V: VoiceService,
    W: ServerUnitOfWork,
{
    channel_repository: Arc<C>,
    voice_service: Arc<V>,
    unit_of_work: Arc<W>,
}

impl<C, V, W> DeleteChannel<C, V, W>
where
    C: ChannelRepository,
    V: VoiceService,
    W: ServerUnitOfWork,
{
    pub fn new(channel_repository: Arc<C>, voice_service: Arc<V>, unit_of_work: Arc<W>) -> Self {
        Self {
            channel_repository,
            voice_service,
            unit_of_work,
        }
    }

    pub async fn execute(&self, request: DeleteChannelRequest) -> Result<(), DeleteChannelError> {
        let DeleteChannelRequest {
            server_id,
            channel_id,
            deleted_by,
            ctx,
        } = request;

        let mut channel = self
            .channel_repository
            .find_channel_by_id(&channel_id)
            .await?;

        if channel.server_id() != &server_id {
            return Err(DeleteChannelError::ChannelNotFound);
        }

        let now = Utc::now();
        let supports_voice = channel.supports_voice();

        let mut tx = self.unit_of_work.begin(&server_id).await?;
        let server = tx.load_server().await?;
        let grant = server.ensure_can_manage_channels(&deleted_by)?;
        channel.delete(deleted_by, grant, now).map_err(|e| match e {
            ChannelError::NotAllowed => DeleteChannelError::NotOwner,
            other => DeleteChannelError::Domain(other),
        })?;

        let member_ids = server.member_ids();
        let request_id = ctx.request_id.clone();
        let events = channel.pull_events();
        let events: Vec<MessagingEvent> = events.into_iter().map(|e| MessagingEvent::from(e.with_recipients(&member_ids))).collect();

        tx.delete_channel(&channel_id).await?;
        tx.save_events(&events, request_id.as_deref()).await?;
        tx.commit().await?;

        if supports_voice
            && let Err(err) = self.voice_service.delete_voice_channel(&channel_id).await
        {
            tracing::warn!(
                channel_id = %channel_id.uuid(),
                error = %err,
                "Failed to delete LiveKit room after channel removal",
            );
        }

        Ok(())
    }
}
