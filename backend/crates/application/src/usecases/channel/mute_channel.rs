use std::sync::Arc;

use chrono::Utc;
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

use crate::dto::requests::channel::mute_channel::MuteChannelRequest;

#[derive(Debug, thiserror::Error)]
pub enum MuteChannelError {
    #[error("Channel not found")]
    ChannelNotFound,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Channel domain error: {0}")]
    Domain(#[from] ChannelError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
}

#[derive(Debug)]
pub struct MuteChannel<T>
where
    T: ChannelRepository,
{
    channel_repository: Arc<T>,
}

impl<T> MuteChannel<T>
where
    T: ChannelRepository,
{
    pub fn new(channel_repository: Arc<T>) -> Self {
        Self { channel_repository }
    }

    pub async fn execute(
        &self,
        request: MuteChannelRequest,
        initiator: UserId,
    ) -> Result<(), MuteChannelError> {
        let channel_id = ChannelId::new(request.channel_id);
        let target = UserId::new(request.target_user_id);

        let mut channel = self
            .channel_repository
            .find_channel_by_id(&channel_id)
            .await?;

        if channel.server_id().uuid() != request.server_id {
            return Err(MuteChannelError::ChannelNotFound);
        }

        let now = Utc::now();

        channel.mute(target, initiator, now)?;

        let records: Vec<OutboxRecord> = channel
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record())
            .collect::<Result<_, _>>()?;

        self.channel_repository
            .mute_participant_with_events(&channel_id, &target, &initiator, now, &records)
            .await?;

        Ok(())
    }
}
