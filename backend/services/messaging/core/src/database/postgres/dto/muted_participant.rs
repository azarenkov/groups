use chrono::{DateTime, Utc};
use sqlx::{FromRow, types::Uuid};

use domain::{channel::participant_mute::ParticipantMute, user::id::UserId};

#[derive(Debug, FromRow)]
pub struct MutedParticipantDto {
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub muted_by: Uuid,
    pub muted_at: DateTime<Utc>,
}

impl From<MutedParticipantDto> for ParticipantMute {
    fn from(dto: MutedParticipantDto) -> Self {
        ParticipantMute::new(
            UserId::new(dto.user_id),
            UserId::new(dto.muted_by),
            dto.muted_at,
        )
    }
}
