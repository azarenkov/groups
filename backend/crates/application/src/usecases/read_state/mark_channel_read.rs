use std::sync::Arc;

use domain::{
    channel::id::ChannelId,
    read_state::repository::ReadStateRepository,
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

#[derive(Debug, thiserror::Error)]
pub enum MarkChannelReadError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct MarkChannelRead<R: ReadStateRepository> {
    repo: Arc<R>,
}

impl<R: ReadStateRepository> MarkChannelRead<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
    ) -> Result<(), MarkChannelReadError> {
        self.repo.mark_channel_read(&user_id, &channel_id).await?;
        Ok(())
    }
}
