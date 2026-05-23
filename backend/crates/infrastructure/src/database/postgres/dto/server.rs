use chrono::{DateTime, Utc};
use domain::{
    server::{Server, description::ServerDescription, id::ServerId, name::ServerName},
    user::id::UserId,
};
use sqlx::{FromRow, types::Uuid};

#[derive(Debug, FromRow)]
pub struct ServerDto {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub description: Option<String>,
    pub accent_color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ServerDto> for Server {
    fn from(dto: ServerDto) -> Self {
        Server::restore(
            ServerId::new(dto.id),
            ServerName::new(dto.name),
            UserId::new(dto.owner_id),
            dto.description.map(ServerDescription::new),
            dto.accent_color,
            dto.created_at,
            dto.updated_at,
        )
    }
}
