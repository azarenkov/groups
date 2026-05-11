use std::sync::Arc;

use chrono::Utc;
use domain::{
    channel::{
        ChannelError, events::participant::LeaveReason, id::ChannelId,
        repository::ChannelRepository,
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

use crate::dto::requests::channel::leave_voice_channel::LeaveVoiceChannelRequest;

#[derive(Debug, thiserror::Error)]
pub enum LeaveVoiceChannelError {
    #[error("Channel not found")]
    ChannelNotFound,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Channel domain error: {0}")]
    Domain(#[from] ChannelError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
    #[error("Voice not supported for this channel")]
    VoiceNotSupported,
}

#[derive(Debug)]
pub struct LeaveVoiceChannel<T>
where
    T: ChannelRepository,
{
    channel_repository: Arc<T>,
}

impl<T> LeaveVoiceChannel<T>
where
    T: ChannelRepository,
{
    pub fn new(channel_repository: Arc<T>) -> Self {
        Self { channel_repository }
    }

    pub async fn execute(
        &self,
        request: LeaveVoiceChannelRequest,
        user_id: UserId,
    ) -> Result<(), LeaveVoiceChannelError> {
        let channel_id = ChannelId::new(request.channel_id);

        let mut channel = self
            .channel_repository
            .find_channel_by_id(&channel_id)
            .await?;

        if channel.server_id().uuid() != request.server_id {
            return Err(LeaveVoiceChannelError::ChannelNotFound);
        }

        if !channel.kind().supports_voice() {
            return Err(LeaveVoiceChannelError::VoiceNotSupported);
        }

        let now = Utc::now();

        channel.leave(user_id, LeaveReason::ClientInitiated, now)?;

        let records: Vec<OutboxRecord> = channel
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record())
            .collect::<Result<_, _>>()?;

        self.channel_repository
            .record_events(&channel_id, &records)
            .await?;

        Ok(())
    }
}
