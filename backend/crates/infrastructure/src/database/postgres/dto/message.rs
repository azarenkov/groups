use chrono::{DateTime, Utc};
use sqlx::{FromRow, types::Uuid};

#[derive(Debug, FromRow)]
pub struct MessageDto {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub author_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
}
