use std::sync::Arc;

use messaging_domain::{read_state::repository::ChannelUnread, read_state::repository::DmUnread, read_state::repository::ReadStateRepository};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum GetUnreadCountsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct UnreadCounts {
    pub channels: Vec<ChannelUnread>,
    pub dms: Vec<DmUnread>,
}

pub struct GetUnreadCounts<R: ReadStateRepository> {
    repo: Arc<R>,
}

impl<R: ReadStateRepository> GetUnreadCounts<R> {
    pub fn new(repo: Arc<R>) -> Self {
        Self { repo }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        default_level: &str,
    ) -> Result<UnreadCounts, GetUnreadCountsError> {
        let channel_default = match default_level {
            "mentions" | "none" => default_level,
            _ => "all",
        };
        let channels = self
            .repo
            .channel_unread_counts(&user_id, channel_default)
            .await?;
        let dms = self.repo.dm_unread_counts(&user_id).await?;
        Ok(UnreadCounts { channels, dms })
    }
}
