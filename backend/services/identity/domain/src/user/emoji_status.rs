use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EmojiStatus {
    sticker_id: Uuid,
    until: Option<DateTime<Utc>>,
}

impl EmojiStatus {
    pub fn new(sticker_id: Uuid, until: Option<DateTime<Utc>>) -> Self {
        Self { sticker_id, until }
    }

    pub fn sticker_id(&self) -> Uuid {
        self.sticker_id
    }

    pub fn until(&self) -> Option<DateTime<Utc>> {
        self.until
    }

    pub fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        self.until.is_none_or(|until| until > now)
    }
}
