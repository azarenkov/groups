use async_trait::async_trait;
use domain::user::id::UserId;
use messaging_domain::voice_listen::repository::VoiceListenRepository;
use ports::{outbox::record::AggregateType, outbox::record::OutboxRecord, repository::RepositoryError};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error, repositories::outbox_record::save_outbox_records,
};

pub struct PostgresVoiceListenRepository {
    pool: Pool<Postgres>,
}

impl PostgresVoiceListenRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl VoiceListenRepository for PostgresVoiceListenRepository {
    async fn mark_listened(
        &self,
        message_id: Uuid,
        listener: &UserId,
        notify: &[Uuid],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (SELECT 1 FROM (SELECT id FROM messages WHERE id = $1 FOR KEY SHARE) m)
                OR EXISTS (SELECT 1 FROM (SELECT id FROM dm_messages WHERE id = $1 FOR KEY SHARE) d)
            "#,
        )
        .bind(message_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        if !exists {
            return Ok(());
        }

        let listened_at = sqlx::query_scalar!(
            r#"
            INSERT INTO voice_listens (message_id, user_id)
            VALUES ($1, $2)
            ON CONFLICT (message_id, user_id) DO NOTHING
            RETURNING listened_at
            "#,
            message_id,
            listener.uuid(),
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        let Some(listened_at) = listened_at else {
            return Ok(());
        };

        let mut recipients: Vec<Uuid> = notify.to_vec();
        if !recipients.contains(&listener.uuid()) {
            recipients.push(listener.uuid());
        }
        let dm_pair: Option<(Uuid, Uuid)> =
            sqlx::query_as("SELECT sender_id, recipient_id FROM dm_messages WHERE id = $1")
                .bind(message_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
        let (aggregate_type, parent_id, parent_type, skipped_ids) = match dm_pair {
            Some((sender, recipient)) => {
                let peer = if sender == listener.uuid() { recipient } else { sender };
                (AggregateType::User, peer, AggregateType::User, Vec::new())
            }
            None => {
                let channel_id: Uuid = sqlx::query_scalar("SELECT channel_id FROM messages WHERE id = $1")
                    .bind(message_id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(map_sqlx_error_to_domain_error)?;
                let others: Vec<Uuid> = sqlx::query_scalar(
                    "SELECT user_id FROM channel_text_participants WHERE channel_id = $1 AND user_id <> ALL($2)",
                )
                .bind(channel_id)
                .bind(&recipients)
                .fetch_all(&mut *tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
                (AggregateType::Message, channel_id, AggregateType::Channel, others)
            }
        };
        let aggregate_id = if aggregate_type == AggregateType::User { listener.uuid() } else { message_id };
        let record = OutboxRecord {
            id: Uuid::new_v4(),
            aggregate_id,
            aggregate_type,
            parent_id: Some(parent_id),
            parent_type: Some(parent_type),
            event_type: "voice.listened".to_string(),
            payload: serde_json::json!({
                "message_id": message_id,
                "listener_id": listener.uuid(),
                "listened_at": listened_at,
                "recipients": recipients,
                "skipped_ids": skipped_ids,
            }),
            occurred_at: listened_at,
            request_id: None,
        };
        save_outbox_records(&mut tx, std::slice::from_ref(&record)).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn listened_in_channel(
        &self,
        message_ids: &[Uuid],
        viewer: &UserId,
    ) -> Result<Vec<Uuid>, RepositoryError> {
        sqlx::query_scalar!(
            r#"
            SELECT message_id FROM voice_listens
            WHERE message_id = ANY($1) AND user_id = $2
            "#,
            message_ids,
            viewer.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)
    }

    async fn listened_in_dm(
        &self,
        message_ids: &[Uuid],
        viewer: &UserId,
    ) -> Result<Vec<Uuid>, RepositoryError> {
        sqlx::query_scalar!(
            r#"
            SELECT DISTINCT l.message_id AS "message_id!"
            FROM voice_listens l
            INNER JOIN dm_messages d ON d.id = l.message_id
            WHERE l.message_id = ANY($1)
              AND (
                  l.user_id = $2
                  OR (d.sender_id = $2 AND l.user_id = d.recipient_id)
              )
            "#,
            message_ids,
            viewer.uuid(),
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)
    }
}
