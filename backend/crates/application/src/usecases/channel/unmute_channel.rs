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
    user::{id::UserId, repository::UserRepository},
};

use crate::dto::requests::channel::unmute_channel::UnmuteChannelRequest;

#[derive(Debug, thiserror::Error)]
pub enum UnmuteChannelError {
    #[error("Channel not found")]
    ChannelNotFound,
    #[error("Target user not found")]
    TargetUserNotFound,
    #[error("Voice not supported for this channel")]
    VoiceNotSupported,
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Channel domain error: {0}")]
    Domain(#[from] ChannelError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
}

#[derive(Debug)]
pub struct UnmuteChannel<C, U>
where
    C: ChannelRepository,
    U: UserRepository,
{
    channel_repository: Arc<C>,
    user_repository: Arc<U>,
}

impl<C, U> UnmuteChannel<C, U>
where
    C: ChannelRepository,
    U: UserRepository,
{
    pub fn new(channel_repository: Arc<C>, user_repository: Arc<U>) -> Self {
        Self {
            channel_repository,
            user_repository,
        }
    }

    pub async fn execute(
        &self,
        request: UnmuteChannelRequest,
        initiator: UserId,
    ) -> Result<(), UnmuteChannelError> {
        let channel_id = ChannelId::new(request.channel_id);
        let target = UserId::new(request.target_user_id);

        let mut channel = self
            .channel_repository
            .find_channel_by_id(&channel_id)
            .await?;

        if channel.server_id().uuid() != request.server_id {
            return Err(UnmuteChannelError::ChannelNotFound);
        }

        if !channel.supports_voice() {
            return Err(UnmuteChannelError::VoiceNotSupported);
        }

        self.user_repository
            .find_by_id(&target)
            .await?
            .ok_or(UnmuteChannelError::TargetUserNotFound)?;

        let now = Utc::now();

        channel.unmute(target, initiator, now)?;

        let request_id = shared::middleware::request_id::current();
        let records: Vec<OutboxRecord> = channel
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record_with_request_id(request_id.clone()))
            .collect::<Result<_, _>>()?;

        self.channel_repository
            .unmute_participant_with_events(&channel_id, &target, &records)
            .await?;

        Ok(())
    }
}
