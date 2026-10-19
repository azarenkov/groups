use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use messaging_domain::emoji_status::{EmojiStatus, EmojiStatusRepository, UserEmojiStatus};
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresEmojiStatusRepository {
    pool: Pool<Postgres>,
}

impl PostgresEmojiStatusRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EmojiStatusRepository for PostgresEmojiStatusRepository {
    async fn apply_emoji_status(
        &self,
        user_id: &UserId,
        emoji_status: Option<Option<EmojiStatus>>,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        let Some(emoji_status) = emoji_status else {
            return Ok(());
        };
        sqlx::query!(
            r#"
            INSERT INTO messaging_user_profiles AS p (
                user_id, name, updated_at,
                emoji_status_sticker_id, emoji_status_until, emoji_status_updated_at
            )
            VALUES ($1, '', to_timestamp(0), $2, $3, $4::timestamptz)
            ON CONFLICT (user_id) DO UPDATE
                SET emoji_status_sticker_id = CASE
                        WHEN p.emoji_status_updated_at IS NULL OR p.emoji_status_updated_at <= $4::timestamptz
                            THEN EXCLUDED.emoji_status_sticker_id
                        ELSE p.emoji_status_sticker_id END,
                    emoji_status_until = CASE
                        WHEN p.emoji_status_updated_at IS NULL OR p.emoji_status_updated_at <= $4::timestamptz
                            THEN EXCLUDED.emoji_status_until
                        ELSE p.emoji_status_until END,
                    emoji_status_updated_at = GREATEST(p.emoji_status_updated_at, $4::timestamptz)
            "#,
            user_id.uuid(),
            emoji_status.map(|status| status.sticker_id),
            emoji_status.and_then(|status| status.until),
            occurred_at,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn emoji_statuses(
        &self,
        user_ids: &[UserId],
        now: DateTime<Utc>,
    ) -> Result<Vec<UserEmojiStatus>, RepositoryError> {
        let ids: Vec<Uuid> = user_ids.iter().map(|id| id.uuid()).collect();
        let rows = sqlx::query!(
            r#"
            SELECT p.user_id, p.emoji_status_sticker_id, p.emoji_status_until, p.emoji_status_updated_at
            FROM messaging_user_profiles p
            WHERE p.user_id = ANY($1)
              AND p.active
              AND p.emoji_status_sticker_id IS NOT NULL
              AND (p.emoji_status_until IS NULL OR p.emoji_status_until > $2)
            "#,
            &ids,
            now,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows
            .into_iter()
            .map(|r| UserEmojiStatus {
                user_id: UserId::new(r.user_id),
                emoji_status: r
                    .emoji_status_sticker_id
                    .map(|sticker_id| EmojiStatus { sticker_id, until: r.emoji_status_until }),
                updated_at: r.emoji_status_updated_at,
            })
            .collect())
    }
}
