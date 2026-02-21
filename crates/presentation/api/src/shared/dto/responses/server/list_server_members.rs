use domain::user::User;
use serde::Serialize;
use uuid::Uuid;

use crate::shared::dto::pagination::PaginationMeta;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListServerMembersResponse {
    pub members: Vec<MemberResponse>,
    pub pagination: PaginationMeta,
}

impl ListServerMembersResponse {
    pub fn new(users: Vec<User>, pagination: PaginationMeta) -> Self {
        Self {
            members: users.into_iter().map(MemberResponse::from).collect(),
            pagination,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberResponse {
    pub id: Uuid,
    pub name: String,
}

impl From<User> for MemberResponse {
    fn from(value: User) -> Self {
        Self {
            id: value.id().uuid(),
            name: value.name().as_str().to_string(),
        }
    }
}
