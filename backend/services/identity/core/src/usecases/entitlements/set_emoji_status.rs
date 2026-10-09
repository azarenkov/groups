use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use domain::user::id::UserId;
use identity_domain::{
    events::IdentityEvent,
    unit_of_work::IdentityUnitOfWork,
    user::UserError,
    user::emoji_status::EmojiStatus,
    user::tier::Tier,
};
use ports::repository::RepositoryError;
use uuid::Uuid;

use crate::ports::sticker_lookup::{StickerLookup, StickerLookupError};

#[derive(Debug, thiserror::Error)]
pub enum SetEmojiStatusError {
    #[error("A paid plan is required")]
    ProRequired,
    #[error("Sticker not found")]
    StickerNotFound,
    #[error("Unknown status duration")]
    UnknownDuration,
    #[error("User not found")]
    UserNotFound,
    #[error("Sticker lookup unavailable: {0}")]
    Unavailable(String),
    #[error("Repository error: {0}")]
    Repository(#[from] RepositoryError),
}

impl From<UserError> for SetEmojiStatusError {
    fn from(_: UserError) -> Self {
        Self::UserNotFound
    }
}

impl From<StickerLookupError> for SetEmojiStatusError {
    fn from(error: StickerLookupError) -> Self {
        match error {
            StickerLookupError::Unavailable(reason) => Self::Unavailable(reason),
        }
    }
}

pub fn status_until(duration: Option<&str>, now: DateTime<Utc>) -> Result<Option<DateTime<Utc>>, SetEmojiStatusError> {
    let span = match duration {
        None => return Ok(None),
        Some("1h") => Duration::hours(1),
        Some("8h") => Duration::hours(8),
        Some("1d") => Duration::days(1),
        Some("7d") => Duration::days(7),
        Some(_) => return Err(SetEmojiStatusError::UnknownDuration),
    };
    Ok(Some(now + span))
}

pub struct SetEmojiStatus<W: IdentityUnitOfWork, S: StickerLookup> {
    unit_of_work: Arc<W>,
    stickers: Arc<S>,
}

impl<W: IdentityUnitOfWork, S: StickerLookup> SetEmojiStatus<W, S> {
    pub fn new(unit_of_work: Arc<W>, stickers: Arc<S>) -> Self {
        Self { unit_of_work, stickers }
    }

    pub async fn set(
        &self,
        user_id: UserId,
        sticker_id: Uuid,
        duration: Option<&str>,
        request_id: Option<&str>,
    ) -> Result<(EmojiStatus, DateTime<Utc>), SetEmojiStatusError> {
        let until = status_until(duration, Utc::now())?;
        if !self.has_pro(&user_id).await? {
            return Err(SetEmojiStatusError::ProRequired);
        }
        if !self.stickers.usable_sticker(&user_id, sticker_id).await? {
            return Err(SetEmojiStatusError::StickerNotFound);
        }
        let status = EmojiStatus::new(sticker_id, until);
        let changed_at = self.apply(user_id, Some(status), request_id).await?;
        Ok((status, changed_at))
    }

    pub async fn clear(&self, user_id: UserId, request_id: Option<&str>) -> Result<DateTime<Utc>, SetEmojiStatusError> {
        self.apply(user_id, None, request_id).await
    }

    async fn has_pro(&self, user_id: &UserId) -> Result<bool, SetEmojiStatusError> {
        let mut tx = self.unit_of_work.begin().await?;
        let entitlements = tx.load_entitlements(user_id).await?;
        tx.commit().await?;
        Ok(entitlements.active_at(Utc::now()).includes(Tier::Pro))
    }

    async fn apply(
        &self,
        user_id: UserId,
        status: Option<EmojiStatus>,
        request_id: Option<&str>,
    ) -> Result<DateTime<Utc>, SetEmojiStatusError> {
        let mut tx = self.unit_of_work.begin().await?;
        let mut user = tx
            .load_user_for_update(&user_id)
            .await?
            .ok_or(SetEmojiStatusError::UserNotFound)?;
        let now = Utc::now();
        if status.is_some() && !tx.load_entitlements(&user_id).await?.active_at(now).includes(Tier::Pro) {
            return Err(SetEmojiStatusError::ProRequired);
        }
        user.set_emoji_status(status, now)?;
        let events: Vec<IdentityEvent> = user.pull_events().into_iter().map(IdentityEvent::from).collect();
        tx.save_user(&user).await?;
        tx.save_events(&events, request_id).await?;
        tx.commit().await?;
        Ok(now)
    }
}

#[cfg(test)]
#[path = "set_emoji_status_tests.rs"]
mod tests;
