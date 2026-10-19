use async_trait::async_trait;
use chrono::{DateTime, Utc};
use domain::user::id::UserId;
use ports::repository::RepositoryError;

pub const MAX_EMOJI_STATUS_LOOKUP: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserEmojiStatus {
    pub user_id: UserId,
    pub emoji_status: Option<EmojiStatus>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmojiStatus {
    pub sticker_id: uuid::Uuid,
    pub until: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait EmojiStatusRepository: Send + Sync {
    async fn apply_emoji_status(
        &self,
        user_id: &UserId,
        emoji_status: Option<Option<EmojiStatus>>,
        occurred_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;
    async fn emoji_statuses(
        &self,
        user_ids: &[UserId],
        now: DateTime<Utc>,
    ) -> Result<Vec<UserEmojiStatus>, RepositoryError>;
}
