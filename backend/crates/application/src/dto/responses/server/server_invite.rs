use chrono::{DateTime, Utc};
use domain::server::invite::ServerInvite;
use uuid::Uuid;

#[derive(Debug)]
pub struct ServerInviteDto {
    pub id: Uuid,
    pub server_id: Uuid,
    pub code: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

impl From<ServerInvite> for ServerInviteDto {
    fn from(value: ServerInvite) -> Self {
        Self {
            id: value.id,
            server_id: value.server_id.uuid(),
            code: value.code,
            created_by: value.created_by.uuid(),
            created_at: value.created_at,
        }
    }
}
