use domain::user::User;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListServerMembersResponse {
    pub members: Vec<MemberResponse>,
}

impl ListServerMembersResponse {
    pub fn new(users: Vec<User>) -> Self {
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

impl From<User> for MemberResponse {
    fn from(value: User) -> Self {
        Self {
            id: value.id().uuid(),
            name: value.name().as_str().to_string(),
        }
    }
}
