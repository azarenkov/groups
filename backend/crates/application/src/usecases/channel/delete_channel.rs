use std::sync::Arc;

use domain::{
    channel::{repository::ChannelRepository, id::ChannelId},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

#[derive(Debug, thiserror::Error)]
pub enum DeleteChannelError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
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
        owner_id: UserId,
    ) -> Result<bool, DeleteChannelError> {
        Ok(self
            .channel_repository
            .delete_channel_by_owner(&channel_id, &owner_id)
            .await?)
    }
}
