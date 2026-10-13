use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::{plan::Tier, user::id::UserId};
use ports::repository::RepositoryError;

pub const MAX_BADGE_LOOKUP: usize = 200;

pub fn attachments_per_message(tier: Tier) -> usize {
    match tier {
        Tier::Premium => 20,
        Tier::Pro => 10,
        Tier::Free => 5,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserBadge {
    pub user_id: UserId,
    pub tier: String,
    pub badge_style: String,
    pub member_number: Option<i64>,
    pub emoji_status: Option<BadgeEmojiStatus>,
    pub emoji_status_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BadgeEmojiStatus {
    pub sticker_id: uuid::Uuid,
    pub until: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait BadgeRepository: Send + Sync {
    async fn apply_member_number(&self, user_id: &UserId, member_number: i64) -> Result<(), RepositoryError>;
    async fn apply_profile_perks(
        &self,
        user_id: &UserId,
        badge_style: &str,
        emoji_status: Option<Option<BadgeEmojiStatus>>,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;
    async fn paid_badges(
        &self,
        user_ids: &[UserId],
        now: DateTime<Utc>,
    ) -> Result<Vec<UserBadge>, RepositoryError>;
}
