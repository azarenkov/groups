use chrono::{DateTime, Utc};
use sqlx::{FromRow, types::Uuid};

use super::{kind::ChannelKindDto, visibility::ChannelVisibilityDto};

#[derive(Debug, FromRow)]
pub struct ChannelDto {
    pub id: Uuid,
    pub server_id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub kind: ChannelKindDto,
    pub visibility: ChannelVisibilityDto,
    pub password_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_join: DateTime<Utc>,
}
