use application::dto::requests::room::generate_token::GenerateTokenRequest as GenerateTokenRequestApplication;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct GenerateTokenRequest {
    #[validate(length(min = 3, max = 30))]
    pub room_name: String,
}

impl From<GenerateTokenRequest> for GenerateTokenRequestApplication {
    fn from(value: GenerateTokenRequest) -> Self {
        Self {
            room_name: value.room_name,
        }
    }
}
