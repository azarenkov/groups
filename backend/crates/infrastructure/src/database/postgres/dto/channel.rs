use chrono::{DateTime, Utc};
use domain::{
    channel::{Channel, id::ChannelId, max_paticipants::MaxParticipants, name::ChannelName},
    server::id::ServerId,
    shared::value_objects::password_hash::PasswordHash,
    user::id::UserId,
};
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

impl From<ChannelDto> for Channel {
    fn from(value: ChannelDto) -> Self {
        Self::restore(
            ChannelId::new(value.id),
            ServerId::new(value.server_id),
            UserId::new(value.owner_id),
            ChannelName::new(value.name),
            MaxParticipants::new(value.max_participants as u8),
            value.channel_type.into(),
            value.password_hash.map(PasswordHash::new),
            value.created_at,
            value.updated_at,
            value.last_join,
        )
    }
}
