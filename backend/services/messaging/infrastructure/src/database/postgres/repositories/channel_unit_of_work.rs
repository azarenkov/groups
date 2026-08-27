use async_trait::async_trait;
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use messaging_domain::{events::MessagingEvent, channel::id::ChannelId, channel::unit_of_work::ChannelTransaction, channel::unit_of_work::ChannelUnitOfWork};
use sqlx::{Pool, Postgres, Transaction};

use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error, repositories::outbox_record::save_outbox_records,
};

pub struct PostgresChannelUnitOfWork {
    pool: Pool<Postgres>,
}

impl PostgresChannelUnitOfWork {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

pub struct PostgresChannelTransaction {
    tx: Transaction<'static, Postgres>,
}

#[async_trait]
impl ChannelUnitOfWork for PostgresChannelUnitOfWork {
    async fn begin(&self) -> Result<Box<dyn ChannelTransaction>, RepositoryError> {
        let tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        Ok(Box::new(PostgresChannelTransaction { tx }))
    }
}

#[async_trait]
impl ChannelTransaction for PostgresChannelTransaction {
    async fn join_text(&mut self, channel_id: &ChannelId, user_id: &UserId) -> Result<bool, RepositoryError> {
        let result = sqlx::query!(
            r#"
            INSERT INTO channel_text_participants (channel_id, user_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
            channel_id.uuid(),
            user_id.uuid(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(result.rows_affected() > 0)
    }

    async fn save_events(&mut self, events: &[MessagingEvent], request_id: Option<&str>) -> Result<(), RepositoryError> {
        let records = crate::outbox::mapping::to_records(events, request_id)?;
        save_outbox_records(&mut self.tx, &records).await
    }

    async fn commit(self: Box<Self>) -> Result<(), RepositoryError> {
        self.tx.commit().await.map_err(map_sqlx_error_to_domain_error)
    }
}
