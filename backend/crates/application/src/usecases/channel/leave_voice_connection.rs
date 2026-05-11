use std::sync::Arc;

use domain::{
    channel::{ChannelError, id::ChannelId, repository::ChannelRepository},
    shared::{
        outbox::{
            outbox_record::OutboxRecord,
            to_outbox::{OutboxError, ToOutboxRecord},
        },
        repository::error::RepositoryError,
    },
    user::id::UserId,
};

use crate::dto::requests::channel::leave_voice_connection::LeaveVoiceConnectionRequest;

#[derive(Debug, thiserror::Error)]
pub enum LeaveVoiceConnectionError {
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
pub struct LeaveVoiceConnection<T>
where
    T: ChannelRepository,
{
    channel_repository: Arc<T>,
}

impl<T> LeaveVoiceConnection<T>
where
    T: ChannelRepository,
{
    pub fn new(channel_repository: Arc<T>) -> Self {
        Self { channel_repository }
    }

    pub async fn execute(
        &self,
        request: LeaveVoiceConnectionRequest,
    ) -> Result<(), LeaveVoiceConnectionError> {
        let channel_id = ChannelId::new(request.channel_id);
        let user_id = UserId::new(request.user_id);

        let mut channel = self
            .channel_repository
            .find_channel_by_id(&channel_id)
            .await?;

        if !channel.kind().supports_voice() {
            return Err(LeaveVoiceConnectionError::VoiceNotSupported);
        }

        channel.leave(user_id, request.reason, request.occurred_at)?;

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
