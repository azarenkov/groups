use chrono::{DateTime, Utc};
use messaging_domain::community::Community;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCommunitiesByUserResponse {
    pub communities: Vec<CommunityResponse>,
}

impl ListCommunitiesByUserResponse {
    pub fn new(communities: Vec<Community>) -> Self {
        Self {
            communities: communities.into_iter().map(CommunityResponse::from).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunityResponse {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub description: Option<String>,
    pub accent_color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Community> for CommunityResponse {
    fn from(value: Community) -> Self {
        Self {
            id: value.id().uuid(),
            name: value.name().as_str().to_string(),
            owner_id: value.owner_id().uuid(),
            description: value.description().map(|d| d.as_str().to_string()),
            accent_color: value.accent_color().map(|s| s.to_string()),
            created_at: value.created_at(),
            updated_at: value.updated_at(),
        }
    }
}
