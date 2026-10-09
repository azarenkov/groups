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
    async fn apply_tier(
        &self,
        user_id: &UserId,
        tier: &str,
        valid_until: Option<DateTime<Utc>>,
        member_number: Option<i64>,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO messaging_user_profiles (user_id, name, updated_at, tier, tier_valid_until, member_number, tier_updated_at)
            VALUES ($1, '', to_timestamp(0), $2, $3, $4, $5)
            ON CONFLICT (user_id) DO UPDATE
                SET tier = EXCLUDED.tier,
                    tier_valid_until = EXCLUDED.tier_valid_until,
                    member_number = COALESCE(EXCLUDED.member_number, messaging_user_profiles.member_number),
                    tier_updated_at = EXCLUDED.tier_updated_at
                WHERE messaging_user_profiles.tier_updated_at IS NULL
                   OR messaging_user_profiles.tier_updated_at <= EXCLUDED.tier_updated_at
            "#,
            user_id.uuid(),
            tier,
            valid_until,
            member_number,
            occurred_at,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn apply_profile_perks(
        &self,
        user_id: &UserId,
        badge_style: &str,
        emoji_status: Option<BadgeEmojiStatus>,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO messaging_user_profiles (user_id, name, updated_at, badge_style, emoji_status_sticker_id, emoji_status_until, badge_updated_at)
            VALUES ($1, '', to_timestamp(0), $2, $3, $4, $5)
            ON CONFLICT (user_id) DO UPDATE
                SET badge_style = EXCLUDED.badge_style,
                    emoji_status_sticker_id = EXCLUDED.emoji_status_sticker_id,
                    emoji_status_until = EXCLUDED.emoji_status_until,
                    badge_updated_at = EXCLUDED.badge_updated_at
                WHERE messaging_user_profiles.badge_updated_at IS NULL
                   OR messaging_user_profiles.badge_updated_at <= EXCLUDED.badge_updated_at
            "#,
            user_id.uuid(),
            badge_style,
            emoji_status.map(|status| status.sticker_id),
            emoji_status.and_then(|status| status.until),
            occurred_at,
        )
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(())
    }

    async fn tier_of(&self, user_id: &UserId, now: DateTime<Utc>) -> Result<String, RepositoryError> {
        let tier = sqlx::query_scalar!(
            r#"
            SELECT tier FROM messaging_user_profiles
            WHERE user_id = $1
              AND (tier_valid_until IS NULL OR tier_valid_until > $2)
            "#,
            user_id.uuid(),
            now,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx_error_to_domain_error)?;
        Ok(tier.unwrap_or_else(|| "free".to_string()))
    }

    async fn paid_badges(
        &self,
        user_ids: &[UserId],
        now: DateTime<Utc>,
    ) -> Result<Vec<UserBadge>, RepositoryError> {
        let ids: Vec<Uuid> = user_ids.iter().map(|id| id.uuid()).collect();
        let rows = sqlx::query!(
            r#"
            SELECT user_id, tier, badge_style, member_number, emoji_status_sticker_id, emoji_status_until
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
            })
            .collect())
    }
}
