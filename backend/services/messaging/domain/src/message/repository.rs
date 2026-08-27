use async_trait::async_trait;

use domain::{attachment::AttachmentLink, user::id::UserId, shared::pagination::PaginatedResult, shared::pagination::Pagination, shared::repository::error::RepositoryError};
use crate::{channel::id::ChannelId, message::Message, message::id::MessageId, message::view::MessageView};

#[async_trait]
pub trait MessageRepository {

    async fn list_attachment_links_for_messages(
        &self,
        message_ids: &[uuid::Uuid],
    ) -> Result<Vec<(uuid::Uuid, AttachmentLink)>, RepositoryError>;

    async fn find_attachment_for_user(
        &self,
        attachment_id: uuid::Uuid,
        user_id: &UserId,
    ) -> Result<Option<AttachmentLink>, RepositoryError>;

    async fn linked_attachment_subset(
        &self,
        attachment_ids: &[uuid::Uuid],
    ) -> Result<Vec<uuid::Uuid>, RepositoryError>;

    async fn find_message_by_id(
        &self,
        message_id: &MessageId,
    ) -> Result<Message, RepositoryError>;

    async fn list_messages_in_channel(
        &self,
        channel_id: &ChannelId,
        before: Option<&MessageId>,
        after: Option<&MessageId>,
        limit: i64,
    ) -> Result<Vec<MessageView>, RepositoryError>;

    async fn search_messages_in_channel(
        &self,
        channel_id: &ChannelId,
        query: &str,
        pagination: Pagination,
    ) -> Result<PaginatedResult<MessageView>, RepositoryError>;
}
