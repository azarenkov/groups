use application::dto::requests::room::generate_token::GenerateTokenRequest as GenerateTokenRequestApplication;
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct GenerateTokenRequest {
    pub room_id: Uuid,
    pub server_id: Uuid,
}

impl From<&GenerateTokenRequest> for GenerateTokenRequestApplication {
    fn from(value: &GenerateTokenRequest) -> Self {
        Self {
            room_id: value.room_id,
            server_id: value.server_id,
        }
    }
}
