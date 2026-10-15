use chrono::{DateTime, Utc};
use domain::attachment::AttachmentLink;
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::{
    dm_message::DmMessage, dm_message::id::DmMessageId, events::MessagingEvent,
    friendship::block::BlockRelation,
};

#[async_trait::async_trait]
pub trait DmUnitOfWork: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn DmTransaction>, RepositoryError>;
}

#[async_trait::async_trait]
pub trait DmTransaction: Send {
    async fn lock_pair(&mut self, a: &UserId, b: &UserId) -> Result<(), RepositoryError>;
    async fn block_relation(
        &mut self,
        viewer: &UserId,
        other: &UserId,
    ) -> Result<BlockRelation, RepositoryError>;
    async fn users_hiding(&mut self, message_id: Uuid) -> Result<Vec<UserId>, RepositoryError>;
    async fn reply_target_hidden_for(
        &mut self,
        message_id: Uuid,
        a: &UserId,
        b: &UserId,
    ) -> Result<Option<Vec<UserId>>, RepositoryError>;
    async fn insert_message(
        &mut self,
        message: &DmMessage,
        attachments: &[AttachmentLink],
    ) -> Result<(), RepositoryError>;
    async fn last_read_at(&mut self, reader: &UserId, peer: &UserId) -> Result<Option<DateTime<Utc>>, RepositoryError>;
    async fn consume_call_log(
        &mut self,
        call_id: Uuid,
        sender_id: &UserId,
        recipient_id: &UserId,
    ) -> Result<(), RepositoryError>;
    async fn update_message(
        &mut self,
        message: &DmMessage,
        add_attachments: &[AttachmentLink],
        remove_attachment_ids: &[Uuid],
    ) -> Result<(), RepositoryError>;
    async fn delete_message(&mut self, id: &DmMessageId) -> Result<(), RepositoryError>;
    async fn save_events(&mut self, events: &[MessagingEvent], request_id: Option<&str>) -> Result<(), RepositoryError>;
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;
}
