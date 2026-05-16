use async_trait::async_trait;
use domain::{
    channel::id::ChannelId,
    message::{
        Message, content::MessageContent, id::MessageId, repository::MessageRepository,
    },
    shared::{outbox::outbox_record::OutboxRecord, repository::error::RepositoryError},
    user::id::UserId,
};
use sqlx::{Pool, Postgres};

use crate::database::postgres::{
    dto::message::MessageDto, error::map_sqlx_error_to_domain_error,
    repositories::outbox_record::save_outbox_records,
};

pub struct PostgresMessageRepository {
    pool: Pool<Postgres>,
}

impl PostgresMessageRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MessageRepository for PostgresMessageRepository {
    async fn save_message_with_events(
        &self,
        message: &Message,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        sqlx::query!(
            r#"
            INSERT INTO messages (id, channel_id, author_id, content, created_at, edited_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            message.id().uuid(),
            message.channel_id().uuid(),
            message.author_id().uuid(),
            message.content().as_str(),
            message.created_at(),
            message.edited_at(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        save_outbox_records(&mut tx, events).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn update_message_with_events(
        &self,
        message: &Message,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let result = sqlx::query!(
            r#"
            UPDATE messages
            SET content = $2, edited_at = $3
            WHERE id = $1
            "#,
            message.id().uuid(),
            message.content().as_str(),
            message.edited_at(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        save_outbox_records(&mut tx, events).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn delete_message_with_events(
        &self,
        message_id: &MessageId,
        events: &[OutboxRecord],
    ) -> Result<(), RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(map_sqlx_error_to_domain_error)?;

        let result = sqlx::query!(
            r#"
            DELETE FROM messages WHERE id = $1
            "#,
            message_id.uuid(),
        )
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;

        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }

        save_outbox_records(&mut tx, events).await?;

        tx.commit().await.map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn find_message_by_id(
        &self,
        message_id: &MessageId,
    ) -> Result<Message, RepositoryError> {
        let dto = sqlx::query_as!(
            MessageDto,
            r#"
            SELECT id, channel_id, author_id, content, created_at, edited_at
            FROM messages
            WHERE id = $1
            "#,
            message_id.uuid(),
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?
        .ok_or(RepositoryError::NotFound)?;

        dto_to_message(dto)
    }

    async fn list_messages_in_channel(
        &self,
        channel_id: &ChannelId,
        before: Option<&MessageId>,
        after: Option<&MessageId>,
        limit: i64,
    ) -> Result<Vec<Message>, RepositoryError> {
        let dtos = match (before, after) {
            (Some(b), _) => sqlx::query_as!(
                MessageDto,
                r#"
                SELECT id, channel_id, author_id, content, created_at, edited_at
                FROM messages
                WHERE channel_id = $1
                  AND (created_at, id) < (
                      SELECT created_at, id FROM messages
                      WHERE id = $2 AND channel_id = $1
                  )
                ORDER BY created_at DESC, id DESC
                LIMIT $3
                "#,
                channel_id.uuid(),
                b.uuid(),
                limit,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?,
            (None, Some(a)) => {
                let mut rows = sqlx::query_as!(
                    MessageDto,
                    r#"
                    SELECT id, channel_id, author_id, content, created_at, edited_at
                    FROM messages
                    WHERE channel_id = $1
                      AND (created_at, id) > (
                          SELECT created_at, id FROM messages
                          WHERE id = $2 AND channel_id = $1
                      )
                    ORDER BY created_at ASC, id ASC
                    LIMIT $3
                    "#,
                    channel_id.uuid(),
                    a.uuid(),
                    limit,
                )
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx_error_to_domain_error)?;
                rows.reverse();
                rows
            }
            (None, None) => sqlx::query_as!(
                MessageDto,
                r#"
                SELECT id, channel_id, author_id, content, created_at, edited_at
                FROM messages
                WHERE channel_id = $1
                ORDER BY created_at DESC, id DESC
                LIMIT $2
                "#,
                channel_id.uuid(),
                limit,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_error_to_domain_error)?,
        };

        dtos.into_iter().map(dto_to_message).collect()
    }
}

fn dto_to_message(dto: MessageDto) -> Result<Message, RepositoryError> {
    let content = MessageContent::new(dto.content)
        .map_err(|e| RepositoryError::InvalidData(e.to_string()))?;
    Ok(Message::restore(
        MessageId::new(dto.id),
        ChannelId::new(dto.channel_id),
        UserId::new(dto.author_id),
        content,
        dto.created_at,
        dto.edited_at,
    ))
}
