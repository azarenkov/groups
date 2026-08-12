use async_trait::async_trait;

use crate::{
    channel::id::ChannelId,
    message::{Message, id::MessageId, view::MessageView},
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
        attachment_ids: &[uuid::Uuid],
    ) -> Result<(), RepositoryError>;

    async fn update_message_with_events(
        &self,
        message: &Message,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;

    async fn delete_message_with_events(
        &self,
        message_id: &MessageId,
        events: &[OutboxRecord],
    ) -> Result<Vec<String>, RepositoryError>;

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
