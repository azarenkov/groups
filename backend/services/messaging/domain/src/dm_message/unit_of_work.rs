use domain::attachment::AttachmentLink;
use ports::{outbox::record::OutboxRecord, repository::RepositoryError};
use uuid::Uuid;

use crate::{dm_message::DmMessage, dm_message::id::DmMessageId};

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
    async fn update_message(
        &mut self,
        message: &DmMessage,
        add_attachments: &[AttachmentLink],
        remove_attachment_ids: &[Uuid],
    ) -> Result<(), RepositoryError>;
    async fn delete_message(&mut self, id: &DmMessageId) -> Result<(), RepositoryError>;
    async fn save_outbox(&mut self, records: &[OutboxRecord]) -> Result<(), RepositoryError>;
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;
}
