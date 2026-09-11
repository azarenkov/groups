use std::sync::Arc;

use messaging_domain::{dm_message::repository::DmConversationPreview, dm_message::repository::DmMessageRepository};
use domain::{pagination::{KeysetCursor, Page}, user::id::UserId};
use ports::repository::RepositoryError;

#[derive(Debug, thiserror::Error)]
pub enum ListDmPreviewsError {
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

pub struct ListDmPreviews<M: DmMessageRepository> {
    dm_repository: Arc<M>,
}

impl<M: DmMessageRepository> ListDmPreviews<M> {
    pub fn new(dm_repository: Arc<M>) -> Self {
        Self { dm_repository }
    }

    pub async fn execute(
        &self,
        user_id: UserId,
        cursor: Option<KeysetCursor>,
        limit: i64,
    ) -> Result<Page<DmConversationPreview>, ListDmPreviewsError> {
        let previews = self
            .dm_repository
            .list_conversation_previews(&user_id, cursor.as_ref(), limit)
            .await?;
        Ok(Page::from_overfetch(previews, limit, |p| {
            KeysetCursor::new(p.created_at, p.message_id.to_string())
        }))
    }
}
