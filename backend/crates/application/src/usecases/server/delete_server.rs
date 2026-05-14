use std::{sync::Arc, time::Duration};

use chrono::Utc;
use domain::{
    channel::repository::ChannelRepository,
    server::{
        events::{ServerEvent, lifecycle::ServerDeleted},
        id::ServerId,
        repository::ServerRepository,
    },
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
    },
    user::id::UserId,
};

use crate::ports::voice_service::{VoiceService, VoiceServiceError};

const VOICE_DRAIN_GRACE: Duration = Duration::from_millis(750);

#[derive(Debug, thiserror::Error)]
pub enum DeleteServerError {
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

    pub async fn execute(
        &self,
        server_id: ServerId,
        owner_id: UserId,
    ) -> Result<bool, DeleteServerError> {
        let voice_channel_ids = self
            .channel_repository
            .list_voice_channel_ids_by_server(&server_id)
            .await?;

        let mut had_voice_rooms = false;
        for channel_id in &voice_channel_ids {
            match self.voice_service.delete_voice_channel(channel_id).await {
                Ok(_) => had_voice_rooms = true,
                Err(err) => {
                    tracing::warn!(
                        channel_id = %channel_id.uuid(),
                        server_id = %server_id.uuid(),
                        error = %err,
                        "Failed to delete LiveKit room while deleting server",
                    );
                }
            }
        }

        if had_voice_rooms {
            tokio::time::sleep(VOICE_DRAIN_GRACE).await;
        }

        let event = ServerEvent::Deleted(ServerDeleted::record(server_id, owner_id, Utc::now()));
        let records: Vec<OutboxRecord> = vec![event.to_outbox_record()?];

        let deleted = self
            .server_repository
            .delete_server_by_owner_with_events(&server_id, &owner_id, &records)
            .await?;

        Ok(deleted)
    }
}
