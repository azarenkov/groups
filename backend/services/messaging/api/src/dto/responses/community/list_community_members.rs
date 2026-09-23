use domain::{user::id::UserId, user::name::UserName};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCommunityMembersResponse {
    pub members: Vec<MemberResponse>,
}

impl ListCommunityMembersResponse {
    pub fn new(users: Vec<(UserId, UserName)>) -> Self {
        Self {
            members: users.into_iter().map(MemberResponse::from).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberResponse {
    pub id: Uuid,
    pub name: String,
}

impl From<(UserId, UserName)> for MemberResponse {
    fn from((id, name): (UserId, UserName)) -> Self {
        Self {
            id: id.uuid(),
            name: name.as_str().to_string(),
        }
    }
}
