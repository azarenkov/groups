use std::sync::Arc;

use chrono::Utc;
use domain::{
    attachment::repository::AttachmentRepository,
    channel::repository::ChannelRepository,
    server::{
        events::{ServerEvent, lifecycle::ServerDeleted},
        repository::ServerRepository,
    },
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
    },
};

use crate::{
    dto::requests::server::delete_server::DeleteServerRequest,
    ports::{
        file_storage::FileStorage,
        voice_service::{VoiceService, VoiceServiceError},
    },
};

#[derive(Debug, thiserror::Error)]
pub enum DeleteServerError {
    #[error("Server not found")]
    ServerNotFound,
    #[error("Only the server owner can delete it")]
    NotOwner,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("Voice service error: {0}")]
    VoiceService(#[from] VoiceServiceError),
}

pub struct DeleteServer<S, C, V, A, FS>
where
    S: ServerRepository,
    C: ChannelRepository,
    V: VoiceService,
    A: AttachmentRepository,
    FS: FileStorage,
{
    server_repository: Arc<S>,
    channel_repository: Arc<C>,
    voice_service: Arc<V>,
    attachment_repository: Arc<A>,
    storage: Arc<FS>,
}

impl<S, C, V, A, FS> DeleteServer<S, C, V, A, FS>
where
    S: ServerRepository,
    C: ChannelRepository,
    V: VoiceService,
    A: AttachmentRepository,
    FS: FileStorage,
{
    pub fn new(
        server_repository: Arc<S>,
        channel_repository: Arc<C>,
        voice_service: Arc<V>,
        attachment_repository: Arc<A>,
        storage: Arc<FS>,
    ) -> Self {
        Self {
            server_repository,
            channel_repository,
            voice_service,
            attachment_repository,
            storage,
        }
    }

    pub async fn execute(&self, request: DeleteServerRequest) -> Result<bool, DeleteServerError> {
        let DeleteServerRequest {
            server_id,
            owner_id,
            ctx,
        } = request;

        let actual_owner = self
            .server_repository
            .find_server_owner(&server_id)
            .await?
            .ok_or(DeleteServerError::ServerNotFound)?;

        if actual_owner.uuid() != owner_id.uuid() {
            return Err(DeleteServerError::NotOwner);
        }

        let voice_channel_ids = self
            .channel_repository
            .list_voice_channel_ids_by_server(&server_id)
            .await?;

        let member_ids = self
            .server_repository
            .find_member_ids_by_server_id(&server_id)
            .await?;

        let event = ServerEvent::Deleted(ServerDeleted::record(
            server_id,
            owner_id,
            member_ids,
            Utc::now(),
        ));
        let records: Vec<OutboxRecord> =
            vec![event.to_outbox_record_with_request_id(ctx.request_id)?];

        let Some(storage_keys) = self
            .server_repository
            .delete_server_by_owner_with_events(&server_id, &owner_id, &records)
            .await?
        else {
            return Ok(false);
        };

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

        for channel_id in &voice_channel_ids {
            if let Err(err) = self.voice_service.delete_voice_channel(channel_id).await {
                tracing::warn!(
                    channel_id = %channel_id.uuid(),
                    server_id = %server_id.uuid(),
                    error = %err,
                    "Failed to delete LiveKit room after deleting server",
                );
            }
        }

        Ok(true)
    }
}
