use async_trait::async_trait;
use domain::shared::{outbox::outbox_record::OutboxRecord, repository::error::RepositoryError};

#[async_trait]
pub trait OutboxWriter {
    async fn save(&self, records: &[OutboxRecord]) -> Result<(), RepositoryError>;
}
