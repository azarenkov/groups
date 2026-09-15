use std::collections::HashSet;

use async_trait::async_trait;
use domain::attachment::AttachmentLink;
use ports::repository::RepositoryError;
use messaging_domain::{events::MessagingEvent, message::Message, message::unit_of_work::MessageTransaction, message::unit_of_work::MessageUnitOfWork};
use sqlx::{Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::outbox::storage::release_storage_keys;
use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error,
    repositories::{message::insert_message_links, outbox_record::save_outbox_records},
};

pub struct PostgresMessageUnitOfWork {
    pool: Pool<Postgres>,
}

impl PostgresMessageUnitOfWork {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

pub struct PostgresMessageTransaction {
    tx: Transaction<'static, Postgres>,
}

#[async_trait]
impl MessageUnitOfWork for PostgresMessageUnitOfWork {
    async fn begin(&self) -> Result<Box<dyn MessageTransaction>, RepositoryError> {
        let tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        Ok(Box::new(PostgresMessageTransaction { tx }))
    }
}

#[async_trait]
impl MessageTransaction for PostgresMessageTransaction {
    async fn insert_message(
        &mut self,
        message: &Message,
        attachments: &[AttachmentLink],
        mentions: &[Uuid],
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO messages (id, channel_id, author_id, content, created_at, edited_at, mentions, kind, system_payload)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, ($9::text)::jsonb)
            "#,
            message.id().uuid(),
            message.channel_id().uuid(),
            message.author_id().uuid(),
            message.content().as_str(),
            message.created_at(),
            message.edited_at(),
            mentions,
            message.kind().as_str(),
            message.system().map(|s| s.to_json().to_string()),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        insert_message_links(&mut self.tx, message.id().uuid(), attachments).await
    }

    async fn update_message(
        &mut self,
        message: &Message,
        add_attachments: &[AttachmentLink],
        remove_attachment_ids: &[Uuid],
        mentions: &[Uuid],
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query!(
            r#"
            UPDATE messages
            SET content = $2, edited_at = $3, mentions = $4
            WHERE id = $1
            "#,
            message.id().uuid(),
            message.content().as_str(),
            message.edited_at(),
            mentions,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        if !remove_attachment_ids.is_empty() {
            let removed_keys = sqlx::query_scalar!(
                r#"
                DELETE FROM message_attachments
                WHERE attachment_id = ANY($1) AND message_id = $2
                RETURNING storage_key
                "#,
                remove_attachment_ids,
                message.id().uuid(),
            )
            .fetch_all(&mut *self.tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
            let distinct: HashSet<&Uuid> = remove_attachment_ids.iter().collect();
            if removed_keys.len() != distinct.len() {
                return Err(RepositoryError::InvalidData(
                    "attachments unavailable".to_string(),
                ));
            }
            release_storage_keys(&mut self.tx, &removed_keys).await?;
        }
        insert_message_links(&mut self.tx, message.id().uuid(), add_attachments).await?;
        let final_count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) AS "count!" FROM message_attachments WHERE message_id = $1
            "#,
            message.id().uuid(),
        )
        .fetch_one(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if final_count > 10 || (final_count == 0 && message.content().as_str().is_empty()) {
            return Err(RepositoryError::InvalidData(
                "attachments unavailable".to_string(),
            ));
        }
        Ok(())
    }

    async fn save_events(&mut self, events: &[MessagingEvent], request_id: Option<&str>) -> Result<(), RepositoryError> {
        let records = crate::outbox::mapping::to_records(events, request_id)?;
        save_outbox_records(&mut self.tx, &records).await
    }

    async fn commit(self: Box<Self>) -> Result<(), RepositoryError> {
        self.tx.commit().await.map_err(map_sqlx_error_to_domain_error)
    }
}
