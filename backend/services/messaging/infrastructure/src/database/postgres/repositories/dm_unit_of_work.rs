use std::collections::HashSet;

use async_trait::async_trait;
use domain::attachment::AttachmentLink;
use ports::repository::RepositoryError;
use messaging_domain::{events::MessagingEvent, dm_message::DmMessage, dm_message::id::DmMessageId, dm_message::unit_of_work::DmTransaction, dm_message::unit_of_work::DmUnitOfWork};
use sqlx::{Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error,
    repositories::{
        dm_message::{insert_dm_links, insert_dm_message},
        outbox_record::save_outbox_records,
    },
};

pub struct PostgresDmUnitOfWork {
    pool: Pool<Postgres>,
}

impl PostgresDmUnitOfWork {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

pub struct PostgresDmTransaction {
    tx: Transaction<'static, Postgres>,
}

#[async_trait]
impl DmUnitOfWork for PostgresDmUnitOfWork {
    async fn begin(&self) -> Result<Box<dyn DmTransaction>, RepositoryError> {
        let tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        Ok(Box::new(PostgresDmTransaction { tx }))
    }
}

#[async_trait]
impl DmTransaction for PostgresDmTransaction {
    async fn insert_message(
        &mut self,
        message: &DmMessage,
        attachments: &[AttachmentLink],
    ) -> Result<(), RepositoryError> {
        insert_dm_message(&mut self.tx, message).await?;
        insert_dm_links(&mut self.tx, message.id().uuid(), attachments).await
    }

    async fn update_message(
        &mut self,
        message: &DmMessage,
        add_attachments: &[AttachmentLink],
        remove_attachment_ids: &[Uuid],
    ) -> Result<(), RepositoryError> {
        let result = sqlx::query!(
            r#"
            UPDATE dm_messages
            SET content = $2, edited_at = $3
            WHERE id = $1
            "#,
            message.id().uuid(),
            message.content().as_str(),
            message.edited_at(),
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
                DELETE FROM dm_message_attachments
                WHERE attachment_id = ANY($1) AND dm_message_id = $2
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
            sqlx::query!(
                r#"
                INSERT INTO storage_cleanup_queue (storage_key)
                SELECT * FROM UNNEST($1::text[])
                ON CONFLICT (storage_key) DO NOTHING
                "#,
                &removed_keys,
            )
            .execute(&mut *self.tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        }
        insert_dm_links(&mut self.tx, message.id().uuid(), add_attachments).await?;
        let final_count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) AS "count!" FROM dm_message_attachments WHERE dm_message_id = $1
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

    async fn delete_message(&mut self, id: &DmMessageId) -> Result<(), RepositoryError> {
        let locked = sqlx::query_scalar!(
            r#"SELECT id FROM dm_messages WHERE id = $1 FOR UPDATE"#,
            id.uuid(),
        )
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if locked.is_none() {
            return Err(RepositoryError::NotFound);
        }
        let removed_keys = sqlx::query_scalar!(
            r#"
            DELETE FROM dm_message_attachments WHERE dm_message_id = $1
            RETURNING storage_key
            "#,
            id.uuid(),
        )
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        sqlx::query!(
            r#"
            INSERT INTO storage_cleanup_queue (storage_key)
            SELECT * FROM UNNEST($1::text[])
            ON CONFLICT (storage_key) DO NOTHING
            "#,
            &removed_keys,
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        let result = sqlx::query!(
            r#"
            DELETE FROM dm_messages WHERE id = $1
            "#,
            id.uuid(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
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
