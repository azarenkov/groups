use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    dm_message::{DmMessage, id::DmMessageId, view::DmMessageView},
    server::id::ServerId,
    shared::{
        outbox::outbox_record::OutboxRecord,
        pagination::{PaginatedResult, Pagination},
        repository::error::RepositoryError,
    },
    user::id::UserId,
};

#[derive(Debug, Clone)]
pub struct DmConversationPreview {
    pub message_id: Uuid,
    pub peer_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
    pub call_status: Option<String>,
    pub invite_server_id: Option<Uuid>,
    pub has_attachments: bool,
    pub created_at: DateTime<Utc>,
}

#[async_trait]
pub trait DmMessageRepository {
    async fn save_with_events(
        &self,
        message: &DmMessage,
        events: &[OutboxRecord],
        attachments: &[crate::attachment::AttachmentLink],
    ) -> Result<(), RepositoryError>;

    async fn update_with_events(
        &self,
        message: &DmMessage,
        events: &[OutboxRecord],
        add_attachments: &[crate::attachment::AttachmentLink],
        remove_attachment_ids: &[Uuid],
    ) -> Result<(), RepositoryError>;

    async fn delete_with_events(
        &self,
        id: &DmMessageId,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError>;

    async fn list_attachment_links_for_dm_messages(
        &self,
        dm_message_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, crate::attachment::AttachmentLink)>, RepositoryError>;

    async fn find_by_id(&self, id: &DmMessageId) -> Result<DmMessage, RepositoryError>;

    async fn list_between(
        &self,
        user_a: &UserId,
        user_b: &UserId,
        before: Option<&DmMessageId>,
        after: Option<&DmMessageId>,
        limit: i64,
    ) -> Result<Vec<DmMessageView>, RepositoryError>;

    async fn search_between(
        &self,
        user_a: &UserId,
        user_b: &UserId,
        query: &str,
        pagination: Pagination,
    ) -> Result<PaginatedResult<DmMessageView>, RepositoryError>;

    async fn has_pending_server_invite(
        &self,
        sender_id: &UserId,
        recipient_id: &UserId,
        server_id: &ServerId,
    ) -> Result<bool, RepositoryError>;

    async fn find_invite_recipients(
        &self,
        sender_id: &UserId,
        server_id: &ServerId,
    ) -> Result<Vec<UserId>, RepositoryError>;

    async fn list_conversation_previews(
        &self,
        user_id: &UserId,
    ) -> Result<Vec<DmConversationPreview>, RepositoryError>;
}
