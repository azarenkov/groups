use std::sync::Arc;

use chrono::Utc;
use domain::{
    channel::{ChannelError, repository::ChannelRepository},
    server::{policy::{ModerationPolicy, PolicyError}, unit_of_work::ServerUnitOfWork},
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
    },
};

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
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
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
        let snapshot = tx.authorization().await?;
        ModerationPolicy::manage_channels(&snapshot, &deleted_by)?;
        channel.delete(deleted_by, true, now).map_err(|e| match e {
            ChannelError::NotOwner => DeleteChannelError::NotOwner,
            other => DeleteChannelError::Domain(other),
        })?;

        let member_ids: Vec<_> = snapshot.members.iter().copied().collect();
        let request_id = ctx.request_id.clone();
        let mut records: Vec<OutboxRecord> = channel
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;
        crate::usecases::channel::recipients::inject_recipient_ids(&mut records, &member_ids);

        tx.delete_channel(&channel_id).await?;
        tx.save_outbox(&records).await?;
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
