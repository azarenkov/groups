use std::sync::Arc;

use chrono::Utc;
use messaging_domain::{channel::repository::ChannelRepository, server::events::ServerEvent, server::events::lifecycle::ServerDeleted, server::repository::ServerRepository};
use domain::{shared::outbox::outbox_record::OutboxRecord, shared::outbox::to_outbox::OutboxError, shared::outbox::to_outbox::ToOutboxRecord, shared::repository::error::RepositoryError};

use crate::{
    dto::requests::server::delete_server::DeleteServerRequest,
    ports::voice_service::{VoiceService, VoiceServiceError},
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

pub struct DeleteServer<S, C, V>
where
    S: ServerRepository,
    C: ChannelRepository,
    V: VoiceService,
{
    server_repository: Arc<S>,
    channel_repository: Arc<C>,
    voice_service: Arc<V>,
}

impl<S, C, V> DeleteServer<S, C, V>
where
    S: ServerRepository,
    C: ChannelRepository,
    V: VoiceService,
{
    pub fn new(
        server_repository: Arc<S>,
        channel_repository: Arc<C>,
        voice_service: Arc<V>,
    ) -> Self {
        Self {
            server_repository,
            channel_repository,
            voice_service,
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

        if self
            .server_repository
            .delete_server_by_owner_with_events(&server_id, &owner_id, &records)
            .await?
            .is_none()
        {
            return Ok(false);
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
