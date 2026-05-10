use chrono::{DateTime, Utc};
use sqlx::{FromRow, types::Uuid};

use super::channel_type::ChannelTypeDto;

#[derive(Debug, FromRow)]
pub struct ChannelDto {
    pub id: Uuid,
    pub server_id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub max_participants: i32,
    pub channel_type: ChannelTypeDto,
    pub password_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_join: DateTime<Utc>,
}
