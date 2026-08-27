use chrono::{DateTime, Utc};
use messaging_domain::server::invite_code::InviteCode;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteCodeResponse {
    pub code: String,
    pub created_by: Uuid,
    pub creator_name: Option<String>,
    pub uses: i32,
    pub max_uses: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<InviteCode> for InviteCodeResponse {
    fn from(i: InviteCode) -> Self {
        Self {
            code: i.code,
            created_by: i.created_by,
            creator_name: i.creator_name,
            uses: i.uses,
            max_uses: i.max_uses,
            expires_at: i.expires_at,
            created_at: i.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListInviteCodesResponse {
    pub invites: Vec<InviteCodeResponse>,
}
