use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateTokenResponse {
    pub token: String,
    pub room_id: Uuid,
}

impl GenerateTokenResponse {
    pub fn new(token: String, room_id: Uuid) -> Self {
        Self { token, room_id }
    }
}
