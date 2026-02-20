use domain::user::User;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListServersMembersResponse {
    pub id: Uuid,
    pub name: String,
}

impl From<User> for ListServersMembersResponse {
    fn from(value: User) -> Self {
        Self {
            id: value.id().uuid(),
            name: value.name().as_str().to_string(),
        }
    }
}
