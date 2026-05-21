use application::dto::responses::server::server_invite::ServerInviteDto;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInviteResponse {
    pub id: Uuid,
    pub server_id: Uuid,
    pub code: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

impl From<ServerInviteDto> for ServerInviteResponse {
    fn from(value: ServerInviteDto) -> Self {
        Self {
            id: value.id,
            server_id: value.server_id,
            code: value.code,
            created_by: value.created_by,
            created_at: value.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListServerInvitesResponse {
    pub invites: Vec<ServerInviteResponse>,
}

impl ListServerInvitesResponse {
    pub fn new(invites: Vec<ServerInviteResponse>) -> Self {
        Self { invites }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedeemServerInviteResponse {
    pub server_id: Uuid,
}
