use domain::attachment::AttachmentLink;
use ports::{outbox::record::OutboxRecord, repository::RepositoryError};
use uuid::Uuid;

use crate::message::Message;

#[async_trait::async_trait]
pub trait MessageUnitOfWork: Send + Sync {
    async fn begin(&self) -> Result<Box<dyn MessageTransaction>, RepositoryError>;
}

#[async_trait::async_trait]
pub trait MessageTransaction: Send {
    async fn insert_message(
        &mut self,
        message: &Message,
        attachments: &[AttachmentLink],
        mentions: &[Uuid],
    ) -> Result<(), RepositoryError>;
    async fn update_message(
        &mut self,
        message: &Message,
        add_attachments: &[AttachmentLink],
        remove_attachment_ids: &[Uuid],
        mentions: &[Uuid],
    ) -> Result<(), RepositoryError>;
    async fn save_outbox(&mut self, records: &[OutboxRecord]) -> Result<(), RepositoryError>;
    async fn commit(self: Box<Self>) -> Result<(), RepositoryError>;
}
