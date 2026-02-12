use application::dto::room::generate_token::GenerateTokenRequest as GenerateTokenRequestApplication;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GenerateTokenRequest {
    pub room_name: String,
}

impl From<GenerateTokenRequest> for GenerateTokenRequestApplication {
    fn from(value: GenerateTokenRequest) -> Self {
        Self {
            room_name: value.room_name,
        }
    }
}
