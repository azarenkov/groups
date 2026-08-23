use std::sync::Arc;

use chrono::Utc;
use domain::{
    server::role::{PERM_MANAGE_CHANNELS, ServerRoleRepository, has_permission},
    channel::{ChannelError, repository::ChannelRepository},
    server::repository::ServerRepository,
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

pub struct DeleteChannel<C, V, S, R>
where
    C: ChannelRepository,
    V: VoiceService,
    S: ServerRepository,
    R: ServerRoleRepository,
{
    channel_repository: Arc<C>,
    voice_service: Arc<V>,
    server_repository: Arc<S>,
    role_repository: Arc<R>,
}

impl<C, V, S, R> DeleteChannel<C, V, S, R>
where
    C: ChannelRepository,
    V: VoiceService,
    S: ServerRepository,
    R: ServerRoleRepository,
{
    pub fn new(
        channel_repository: Arc<C>,
        voice_service: Arc<V>,
        server_repository: Arc<S>,
        role_repository: Arc<R>,
    ) -> Self {
        Self {
            channel_repository,
            voice_service,
            server_repository,
            role_repository,
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

        let can_override = if channel.owner_id() == &deleted_by {
            false
        } else {
            let server_owner = self
                .server_repository
                .find_server_owner(&server_id)
                .await?;
            server_owner.as_ref() == Some(&deleted_by)
                || has_permission(
                    self.role_repository
                        .user_permissions(&server_id, &deleted_by)
                        .await?,
                    PERM_MANAGE_CHANNELS,
                )
        };
        channel.delete(deleted_by, can_override, now).map_err(|e| match e {
            ChannelError::NotOwner => DeleteChannelError::NotOwner,
            other => DeleteChannelError::Domain(other),
        })?;

        let member_ids = self
            .server_repository
            .find_member_ids_by_server_id(&server_id)
            .await?;

        let request_id = ctx.request_id.clone();
        let mut records: Vec<OutboxRecord> = channel
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;
        crate::usecases::channel::recipients::inject_recipient_ids(&mut records, &member_ids);

        self.channel_repository
            .delete_channel_with_events(&channel_id, &records)
            .await?;

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
