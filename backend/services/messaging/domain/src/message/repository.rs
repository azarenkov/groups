use async_trait::async_trait;

use domain::{attachment::AttachmentLink, pagination::PaginatedResult, pagination::Pagination, user::id::UserId};
use ports::repository::RepositoryError;
use crate::{channel::id::ChannelId, message::Message, message::id::MessageId, message::view::MessageView, message::view::ReplyPreviewView};

#[async_trait]
pub trait MessageRepository {

    async fn find_reply_author(&self, message_id: &MessageId) -> Result<Option<UserId>, RepositoryError>;

    async fn list_users_hiding(&self, message_id: uuid::Uuid) -> Result<Vec<UserId>, RepositoryError>;

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

    async fn find_message_view(
        &self,
        message_id: &MessageId,
        viewer: &UserId,
    ) -> Result<MessageView, RepositoryError>;

    async fn list_reply_previews(
        &self,
        message_ids: &[uuid::Uuid],
        viewer: &UserId,
    ) -> Result<Vec<ReplyPreviewView>, RepositoryError>;

    async fn list_messages_in_channel(
        &self,
        channel_id: &ChannelId,
        viewer: &UserId,
        before: Option<&MessageId>,
        after: Option<&MessageId>,
        limit: i64,
    ) -> Result<Vec<MessageView>, RepositoryError>;

    async fn search_messages_in_channel(
        &self,
        channel_id: &ChannelId,
        viewer: &UserId,
        query: &str,
        pagination: Pagination,
    ) -> Result<PaginatedResult<MessageView>, RepositoryError>;
}
