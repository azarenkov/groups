use async_trait::async_trait;
use crate::{outbox::record::OutboxRecord, repository::RepositoryError};

#[async_trait]
pub trait OutboxWriter {
    async fn save(&self, records: &[OutboxRecord]) -> Result<(), RepositoryError>;
}
