use async_trait::async_trait;
use ports::outbox_writer::OutboxWriter;
use domain::shared::{outbox::outbox_record::OutboxRecord, repository::error::RepositoryError};
use sqlx::{Pool, Postgres};

use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error, repositories::outbox_record::save_outbox_records,
};

pub struct PostgresOutboxWriter {
    pool: Pool<Postgres>,
}

impl PostgresOutboxWriter {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl OutboxWriter for PostgresOutboxWriter {
    async fn save(&self, records: &[OutboxRecord]) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        save_outbox_records(&mut tx, records).await?;
        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }
}
