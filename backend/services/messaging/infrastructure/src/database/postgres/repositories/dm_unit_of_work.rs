use chrono::{DateTime, Utc};
use std::collections::HashSet;

use async_trait::async_trait;
use domain::attachment::AttachmentLink;
use domain::user::id::UserId;
use ports::repository::RepositoryError;
use messaging_domain::{events::MessagingEvent, dm_message::DmMessage, dm_message::id::DmMessageId, dm_message::unit_of_work::DmTransaction, dm_message::unit_of_work::DmUnitOfWork, friendship::block::BlockRelation};
use sqlx::{Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::outbox::storage::release_storage_keys;
use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error,
    repositories::{
        dm_message::{insert_dm_links, insert_dm_message},
        friendship::block_relation_for,
        friendship_unit_of_work::pair_key,
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
    async fn lock_pair(&mut self, a: &UserId, b: &UserId) -> Result<(), RepositoryError> {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1::text, 43))")
            .bind(pair_key(a, b))
            .execute(&mut *self.tx)
            .await
            .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn block_relation(
        &mut self,
        viewer: &UserId,
        other: &UserId,
    ) -> Result<BlockRelation, RepositoryError> {
        block_relation_for(&mut *self.tx, viewer, other).await
    }

    async fn users_hiding(&mut self, message_id: Uuid) -> Result<Vec<UserId>, RepositoryError> {
        let hiding = sqlx::query_scalar!(
            r#"SELECT user_id FROM hidden_messages WHERE message_id = $1"#,
            message_id,
        )
        .fetch_all(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(hiding.into_iter().map(UserId::new).collect())
    }

    async fn reply_target_hidden_for(
        &mut self,
        message_id: Uuid,
        a: &UserId,
        b: &UserId,
    ) -> Result<Option<Vec<UserId>>, RepositoryError> {
        let exists = sqlx::query_scalar!(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM dm_messages m
                WHERE m.id = $1
                  AND LEAST(m.sender_id, m.recipient_id) = LEAST($2::uuid, $3::uuid)
                  AND GREATEST(m.sender_id, m.recipient_id) = GREATEST($2::uuid, $3::uuid)
            ) AS "exists!"
            "#,
            message_id,
            a.uuid(),
            b.uuid(),
        )
        .fetch_one(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if !exists {
            return Ok(None);
        }
        Ok(Some(self.users_hiding(message_id).await?))
    }

    async fn insert_message(
        &mut self,
        message: &DmMessage,
        attachments: &[AttachmentLink],
    ) -> Result<(), RepositoryError> {
        insert_dm_message(&mut self.tx, message).await?;
        if message.forward().is_some() {
            let keys: Vec<String> = attachments.iter().map(|a| a.storage_key.clone()).collect();
            crate::outbox::storage::ensure_storage_keys_linked(&mut self.tx, &keys).await?;
        }
        insert_dm_links(&mut self.tx, message.id().uuid(), attachments).await
    }

    async fn last_read_at(&mut self, reader: &UserId, peer: &UserId) -> Result<Option<DateTime<Utc>>, RepositoryError> {
        crate::database::postgres::repositories::read_state::dm_last_read_at(&mut self.tx, reader, peer).await
    }

    async fn last_cleared_at(&mut self, user: &UserId, peer: &UserId) -> Result<Option<DateTime<Utc>>, RepositoryError> {
        sqlx::query_scalar!(
            r#"SELECT cleared_at FROM dm_clear_marks WHERE user_id = $1 AND peer_id = $2"#,
            user.uuid(),
            peer.uuid(),
        )
        .fetch_optional(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)
    }

    async fn hide_message(&mut self, id: &DmMessageId, users: &[UserId]) -> Result<(), RepositoryError> {
        let users: Vec<Uuid> = users.iter().map(|u| u.uuid()).collect();
        sqlx::query!(
            r#"
            INSERT INTO hidden_messages (user_id, message_id)
            SELECT u, $2 FROM unnest($1::uuid[]) AS u
            ON CONFLICT (user_id, message_id) DO NOTHING
            "#,
            &users,
            id.uuid(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn consume_call_log(
        &mut self,
        call_id: Uuid,
        sender_id: &UserId,
        recipient_id: &UserId,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"INSERT INTO call_log_consumed (call_id, sender_id, recipient_id) VALUES ($1, $2, $3)"#,
            call_id,
            sender_id.uuid(),
            recipient_id.uuid(),
        )
        .execute(&mut *self.tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
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
            release_storage_keys(&mut self.tx, &removed_keys).await?;
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
        release_storage_keys(&mut self.tx, &removed_keys).await?;
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
