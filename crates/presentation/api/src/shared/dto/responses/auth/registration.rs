use chrono::{DateTime, Utc};
use domain::entites::user::User;
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterUserResponse {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<User> for RegisterUserResponse {
    fn from(value: User) -> Self {
        Self {
            id: value.id().uuid(),
            name: value.name().as_str().to_owned(),
            email: value.email().as_str().to_owned(),
            created_at: value.created_at(),
            updated_at: value.updated_at(),
        }
    }
}
