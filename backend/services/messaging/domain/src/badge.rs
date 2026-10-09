use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

pub const MAX_BADGE_LOOKUP: usize = 200;

pub fn attachments_per_message(tier: &str) -> usize {
    match tier {
        "premium" => 20,
        "pro" => 10,
        _ => 5,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserBadge {
    pub user_id: UserId,
    pub tier: String,
    pub badge_style: String,
    pub member_number: Option<i64>,
    pub emoji_status: Option<BadgeEmojiStatus>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BadgeEmojiStatus {
    pub sticker_id: uuid::Uuid,
    pub until: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait BadgeRepository: Send + Sync {
    async fn apply_tier(
        &self,
        user_id: &UserId,
        tier: &str,
        valid_until: Option<DateTime<Utc>>,
        member_number: Option<i64>,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;
    async fn apply_profile_perks(
        &self,
        user_id: &UserId,
        badge_style: &str,
        emoji_status: Option<BadgeEmojiStatus>,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;
    async fn tier_of(&self, user_id: &UserId, now: DateTime<Utc>) -> Result<String, RepositoryError>;
    async fn paid_badges(
        &self,
        user_ids: &[UserId],
        now: DateTime<Utc>,
    ) -> Result<Vec<UserBadge>, RepositoryError>;
}
