use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{dm_message::DmMessage, dm_message::id::DmMessageId, dm_message::view::DmMessageView, message::view::ReplyPreviewView};
use domain::{pagination::KeysetCursor, pagination::PaginatedResult, pagination::Pagination, user::id::UserId};
use ports::repository::RepositoryError;

#[derive(Debug, Clone)]
pub struct DmConversationPreview {
    pub message_id: Uuid,
    pub peer_id: Uuid,
    pub peer_name: String,
    pub sender_id: Uuid,
    pub content: String,
    pub call_status: Option<String>,
    pub has_attachments: bool,
    pub voice_duration_ms: Option<i32>,
    pub video_note_duration_ms: Option<i32>,
    pub sticker_emoji: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[async_trait]
pub trait DmMessageRepository {

    async fn linked_attachment_subset(
        &self,
        attachment_ids: &[uuid::Uuid],
    ) -> Result<Vec<uuid::Uuid>, RepositoryError>;

    async fn list_users_hiding(&self, message_id: uuid::Uuid) -> Result<Vec<UserId>, RepositoryError>;

    async fn list_attachment_links_for_dm_messages(
        &self,
        dm_message_ids: &[Uuid],
    ) -> Result<Vec<(Uuid, domain::attachment::AttachmentLink)>, RepositoryError>;

    async fn find_by_id(&self, id: &DmMessageId) -> Result<DmMessage, RepositoryError>;

    async fn find_view_between(
        &self,
        id: &DmMessageId,
        user_a: &UserId,
        user_b: &UserId,
    ) -> Result<DmMessageView, RepositoryError>;

    async fn list_dm_reply_previews(
        &self,
        dm_message_ids: &[Uuid],
        viewer: &UserId,
    ) -> Result<Vec<ReplyPreviewView>, RepositoryError>;

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


    async fn list_conversation_previews(
        &self,
        user_id: &UserId,
        cursor: Option<&KeysetCursor>,
        limit: i64,
    ) -> Result<Vec<DmConversationPreview>, RepositoryError>;

    async fn peers_with_history(
        &self,
        user_id: &UserId,
        candidates: &[UserId],
    ) -> Result<Vec<UserId>, RepositoryError>;

    async fn has_conversation(
        &self,
        user_id: &UserId,
        peer_id: &UserId,
    ) -> Result<bool, RepositoryError>;
}
