use std::sync::Arc;

use domain::{
    channel::{id::ChannelId, repository::ChannelRepository},
    read_state::repository::ReadStateRepository,
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

#[derive(Debug, thiserror::Error)]
pub enum MarkChannelReadError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not a member of this channel")]
    NotMember,
}

pub struct MarkChannelRead<R: ReadStateRepository, C: ChannelRepository> {
    repo: Arc<R>,
    channel_repo: Arc<C>,
}

impl<R: ReadStateRepository, C: ChannelRepository> MarkChannelRead<R, C> {
    pub fn new(repo: Arc<R>, channel_repo: Arc<C>) -> Self {
        Self { repo, channel_repo }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
    ) -> Result<(), MarkChannelReadError> {
        let is_member = self
            .channel_repo
            .is_text_channel_member(&channel_id, &user_id)
            .await?;
        if !is_member {
            return Err(MarkChannelReadError::NotMember);
        }
        self.repo.mark_channel_read(&user_id, &channel_id).await?;
        Ok(())
    }
}
