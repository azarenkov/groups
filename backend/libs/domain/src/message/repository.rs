use async_trait::async_trait;

use crate::{
    attachment::AttachmentLink,
    channel::id::ChannelId,
    message::{Message, id::MessageId, view::MessageView},
    user::id::UserId,
    shared::{
        outbox::outbox_record::OutboxRecord,
        pagination::{PaginatedResult, Pagination},
        repository::error::RepositoryError,
    },
};

#[async_trait]
pub trait MessageRepository {
    async fn save_message_with_events(
        &self,
        message: &Message,
        events: &[OutboxRecord],
        attachments: &[AttachmentLink],
        mentions: &[uuid::Uuid],
    ) -> Result<(), RepositoryError>;

    async fn update_message_with_events(
        &self,
        message: &Message,
        events: &[OutboxRecord],
        add_attachments: &[AttachmentLink],
        remove_attachment_ids: &[uuid::Uuid],
        mentions: &[uuid::Uuid],
    ) -> Result<(), RepositoryError>;

    async fn delete_message_with_events(
        &self,
        message_id: &MessageId,
        server_id: &crate::server::id::ServerId,
        actor: &crate::user::id::UserId,
        delegated: bool,
        events: &[OutboxRecord],
    ) -> Result<bool, RepositoryError>;

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
