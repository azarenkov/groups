use domain::attachment::AttachmentLink;
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::{events::MessagingEvent, dm_message::DmMessage, dm_message::id::DmMessageId};

#[async_trait::async_trait]
pub trait DmUnitOfWork: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn DmTransaction>, RepositoryError>;
}

#[async_trait::async_trait]
pub trait DmTransaction: Send {
    async fn insert_message(
        &mut self,
        message: &DmMessage,
        attachments: &[AttachmentLink],
    ) -> Result<(), RepositoryError>;
    async fn consume_call_log(
        &mut self,
        call_id: Uuid,
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
