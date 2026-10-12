use async_trait::async_trait;
use domain::user::id::UserId;
use messaging_domain::hidden_message::repository::HiddenMessageRepository;
use ports::{outbox::record::AggregateType, outbox::record::OutboxRecord, repository::RepositoryError};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::{
    error::map_sqlx_error_to_domain_error, repositories::outbox_record::save_outbox_records,
};

pub struct PostgresHiddenMessageRepository {
    pool: Pool<Postgres>,
}

impl PostgresHiddenMessageRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HiddenMessageRepository for PostgresHiddenMessageRepository {
    async fn hide_for_user(&self, user_id: &UserId, message_id: Uuid) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let hidden_at = sqlx::query_scalar!(
            r#"
            INSERT INTO hidden_messages (user_id, message_id)
            SELECT $1, $2
            WHERE EXISTS (SELECT 1 FROM dm_messages WHERE id = $2 FOR SHARE)
               OR EXISTS (SELECT 1 FROM messages WHERE id = $2 FOR SHARE)
            ON CONFLICT (user_id, message_id) DO UPDATE SET hidden_at = hidden_messages.hidden_at
            RETURNING hidden_at
            "#,
            user_id.uuid(),
            message_id,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .ok_or(RepositoryError::NotFound)?;

        let dm_pair: Option<(Uuid, Uuid)> =
            sqlx::query_as("SELECT sender_id, recipient_id FROM dm_messages WHERE id = $1")
                .bind(message_id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
        let (aggregate_id, aggregate_type, parent_id, parent_type, skipped_ids) = match dm_pair {
            Some((sender, recipient)) => {
                let peer = if sender == user_id.uuid() { recipient } else { sender };
                (user_id.uuid(), AggregateType::User, peer, AggregateType::User, Vec::new())
            }
            None => {
                let channel_id: Uuid = sqlx::query_scalar("SELECT channel_id FROM messages WHERE id = $1")
                    .bind(message_id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(map_sqlx_error_to_domain_error)?;
                let others: Vec<Uuid> = sqlx::query_scalar(
                    "SELECT user_id FROM channel_text_participants WHERE channel_id = $1 AND user_id <> $2",
                )
                .bind(channel_id)
                .bind(user_id.uuid())
                .fetch_all(&mut *tx)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
                (message_id, AggregateType::Message, channel_id, AggregateType::Channel, others)
            }
        };

        let record = OutboxRecord {
            id: Uuid::new_v4(),
            aggregate_id,
            aggregate_type,
            parent_id: Some(parent_id),
            parent_type: Some(parent_type),
            event_type: "message.hidden".to_string(),
            payload: serde_json::json!({
                "user_id": user_id.uuid(),
                "message_id": message_id,
                "hidden_at": hidden_at,
                "skipped_ids": skipped_ids,
            }),
            occurred_at: hidden_at,
            request_id: None,
        };
        save_outbox_records(&mut tx, std::slice::from_ref(&record)).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }
}
