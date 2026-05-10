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

#[derive(Debug, thiserror::Error)]
pub enum DeleteChannelError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Channel domain error: {0}")]
    Domain(#[from] ChannelError),
    #[error("Outbox error: {0}")]
    Outbox(#[from] OutboxError),
}

pub struct DeleteChannel<T>
where
    T: ChannelRepository,
{
    channel_repository: Arc<T>,
}

impl<T> DeleteChannel<T>
where
    T: ChannelRepository,
{
    pub fn new(channel_repository: Arc<T>) -> Self {
        Self { channel_repository }
    }

    pub async fn execute(
        &self,
        channel_id: ChannelId,
        deleted_by: UserId,
    ) -> Result<(), DeleteChannelError> {
        let mut channel = self
            .channel_repository
            .find_channel_by_id(&channel_id)
            .await?;

        let now = Utc::now();

        channel
            .delete(deleted_by, now)
            .map_err(|e| match e {
                ChannelError::NotOwner => DeleteChannelError::Repository(RepositoryError::NotFound),
                other => DeleteChannelError::Domain(other),
            })?;

        let records: Vec<OutboxRecord> = channel
            .pull_events()
            .iter()
            .map(|e| e.to_outbox_record())
            .collect::<Result<_, _>>()?;

        self.channel_repository
            .delete_channel_with_events(&channel_id, &records)
            .await?;

        Ok(())
    }
}
