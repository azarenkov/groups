use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use messaging_domain::badge::{BadgeEmojiStatus, BadgeRepository, UserBadge};
use ports::repository::RepositoryError;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::database::postgres::error::map_sqlx_error_to_domain_error;

pub struct PostgresBadgeRepository {
    pool: Pool<Postgres>,
}

impl PostgresBadgeRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BadgeRepository for PostgresBadgeRepository {
    async fn apply_member_number(&self, user_id: &UserId, member_number: i64) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            INSERT INTO messaging_user_profiles (user_id, name, updated_at, member_number)
            VALUES ($1, '', to_timestamp(0), $2)
            ON CONFLICT (user_id) DO UPDATE
                SET member_number = EXCLUDED.member_number
            "#,
        )
        .bind(user_id.uuid())
        .bind(member_number)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn apply_profile_perks(
        &self,
        user_id: &UserId,
        badge_style: &str,
        emoji_status: Option<Option<BadgeEmojiStatus>>,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        let replace_status = emoji_status.is_some();
        let emoji_status = emoji_status.flatten();
        sqlx::query!(
            r#"
            INSERT INTO messaging_user_profiles AS p (
                user_id, name, updated_at, badge_style, badge_updated_at,
                emoji_status_sticker_id, emoji_status_until, emoji_status_updated_at
            )
            VALUES ($1, '', to_timestamp(0), $2, $5::timestamptz, $3, $4, CASE WHEN $6::boolean THEN $5::timestamptz END)
            ON CONFLICT (user_id) DO UPDATE
                SET badge_style = CASE
                        WHEN p.badge_updated_at IS NULL OR p.badge_updated_at <= $5::timestamptz THEN EXCLUDED.badge_style
                        ELSE p.badge_style END,
                    badge_updated_at = GREATEST(p.badge_updated_at, $5::timestamptz),
                    emoji_status_sticker_id = CASE
                        WHEN $6::boolean AND (p.emoji_status_updated_at IS NULL OR p.emoji_status_updated_at <= $5::timestamptz)
                            THEN EXCLUDED.emoji_status_sticker_id
                        ELSE p.emoji_status_sticker_id END,
                    emoji_status_until = CASE
                        WHEN $6::boolean AND (p.emoji_status_updated_at IS NULL OR p.emoji_status_updated_at <= $5::timestamptz)
                            THEN EXCLUDED.emoji_status_until
                        ELSE p.emoji_status_until END,
                    emoji_status_updated_at = CASE
                        WHEN $6::boolean AND (p.emoji_status_updated_at IS NULL OR p.emoji_status_updated_at <= $5::timestamptz) THEN $5::timestamptz
                        ELSE p.emoji_status_updated_at END
            "#,
            user_id.uuid(),
            badge_style,
            emoji_status.map(|status| status.sticker_id),
            emoji_status.and_then(|status| status.until),
            occurred_at,
            replace_status,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn paid_badges(
        &self,
        user_ids: &[UserId],
        now: DateTime<Utc>,
    ) -> Result<Vec<UserBadge>, RepositoryError> {
        let ids: Vec<Uuid> = user_ids.iter().map(|id| id.uuid()).collect();
        let rows = sqlx::query!(
            r#"
            SELECT user_id, tier, badge_style, member_number, emoji_status_sticker_id, emoji_status_until,
                   emoji_status_updated_at
            FROM messaging_user_profiles
            WHERE user_id = ANY($1)
              AND active
              AND tier <> 'free'
              AND (tier_valid_until IS NULL OR tier_valid_until > $2)
            "#,
            &ids,
            now,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(rows
            .into_iter()
            .map(|r| UserBadge {
                user_id: UserId::new(r.user_id),
                tier: r.tier,
                badge_style: r.badge_style,
                member_number: r.member_number,
                emoji_status: r
                    .emoji_status_sticker_id
                    .filter(|_| r.emoji_status_until.is_none_or(|until| until > now))
                    .map(|sticker_id| BadgeEmojiStatus { sticker_id, until: r.emoji_status_until }),
                emoji_status_updated_at: r.emoji_status_updated_at,
            })
            .collect())
    }
}
