use chrono::{DateTime, Utc};
use domain::server::Server;
use serde::Serialize;
use uuid::Uuid;

use crate::shared::dto::pagination::PaginationMeta;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListServersByUserResponse {
    pub servers: Vec<ServerResponse>,
    pub pagination: PaginationMeta,
}

impl ListServersByUserResponse {
    pub fn new(servers: Vec<Server>, pagination: PaginationMeta) -> Self {
        Self {
            servers: servers.into_iter().map(ServerResponse::from).collect(),
            pagination,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerResponse {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Server> for ServerResponse {
    fn from(value: Server) -> Self {
        Self {
            id: value.id().uuid(),
            name: value.name().as_str().to_string(),
            owner_id: value.owner_id().uuid(),
            description: value.description().map(|d| d.as_str().to_string()),
            created_at: value.created_at(),
            updated_at: value.updated_at(),
        }
    }
}
