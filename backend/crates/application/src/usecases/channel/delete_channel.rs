use std::sync::Arc;

use chrono::Utc;
use domain::{
    attachment::repository::AttachmentRepository,
    channel::{ChannelError, repository::ChannelRepository},
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
    ports::{
        file_storage::FileStorage,
        voice_service::{VoiceService, VoiceServiceError},
    },
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

pub struct DeleteChannel<C, V, A, FS>
where
    C: ChannelRepository,
    V: VoiceService,
    A: AttachmentRepository,
    FS: FileStorage,
{
    channel_repository: Arc<C>,
    voice_service: Arc<V>,
    attachment_repository: Arc<A>,
    storage: Arc<FS>,
}

impl<C, V, A, FS> DeleteChannel<C, V, A, FS>
where
    C: ChannelRepository,
    V: VoiceService,
    A: AttachmentRepository,
    FS: FileStorage,
{
    pub fn new(
        channel_repository: Arc<C>,
        voice_service: Arc<V>,
        attachment_repository: Arc<A>,
        storage: Arc<FS>,
    ) -> Self {
        Self {
            channel_repository,
            voice_service,
            attachment_repository,
            storage,
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

        channel.delete(deleted_by, now).map_err(|e| match e {
            ChannelError::NotOwner => DeleteChannelError::NotOwner,
            other => DeleteChannelError::Domain(other),
        })?;

        let request_id = ctx.request_id.clone();
        let records: Vec<OutboxRecord> = channel
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;

        let storage_keys = self
            .attachment_repository
            .storage_keys_for_channel(channel_id.uuid())
            .await
            .unwrap_or_default();

        self.channel_repository
            .delete_channel_with_events(&channel_id, &records)
            .await?;

        if let Err(e) = self.attachment_repository.enqueue_cleanup(&storage_keys).await {
            tracing::warn!(error = %e, "Failed to enqueue attachment cleanup");
        }
        for key in storage_keys {
            match self.storage.delete(&key).await {
                Ok(()) | Err(crate::ports::file_storage::FileStorageError::NotFound) => {
                    if let Err(e) = self.attachment_repository.dequeue_cleanup(&key).await {
                        tracing::warn!(error = %e, %key, "Failed to dequeue cleaned attachment");
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, %key, "Failed to delete attachment object; cleanup queue will retry");
                }
            }
        }

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
