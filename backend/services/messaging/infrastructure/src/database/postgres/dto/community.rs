use chrono::{DateTime, Utc};
use messaging_domain::{community::Community, community::description::CommunityDescription, community::id::CommunityId, community::name::CommunityName};
use domain::user::id::UserId;
use sqlx::{FromRow, types::Uuid};

#[derive(Debug, FromRow)]
pub struct CommunityDto {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub description: Option<String>,
    pub accent_color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<CommunityDto> for Community {
    fn from(dto: CommunityDto) -> Self {
        Community::restore(
            CommunityId::new(dto.id),
            CommunityName::new(dto.name),
            UserId::new(dto.owner_id),
            dto.description.map(CommunityDescription::new),
            dto.accent_color,
            dto.created_at,
            dto.updated_at,
        )
    }
}
