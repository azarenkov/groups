use std::sync::Arc;

use domain::{
    channel::{id::ChannelId, repository::ChannelRepository},
    read_state::repository::{MemberRead, ReadStateRepository},
    shared::repository::error::RepositoryError,
    user::id::UserId,
};

#[derive(Debug, thiserror::Error)]
pub enum GetChannelReadReceiptsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
    #[error("Not a member of this channel")]
    NotMember,
}

pub struct GetChannelReadReceipts<R: ReadStateRepository, C: ChannelRepository> {
    repo: Arc<R>,
    channel_repo: Arc<C>,
}

impl<R: ReadStateRepository, C: ChannelRepository> GetChannelReadReceipts<R, C> {
    pub fn new(repo: Arc<R>, channel_repo: Arc<C>) -> Self {
        Self { repo, channel_repo }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
    ) -> Result<Vec<MemberRead>, GetChannelReadReceiptsError> {
        let is_member = self
            .channel_repo
            .is_text_channel_member(&channel_id, &user_id)
            .await?;
        if !is_member {
            return Err(GetChannelReadReceiptsError::NotMember);
        }
        let reads = self
            .repo
            .channel_member_read_ats(&channel_id, &user_id)
            .await?;
        Ok(reads)
    }
}
