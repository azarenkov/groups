use domain::value_objects::room::token::RoomToken;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct GenerateTokenResponse {
    pub token: String,
}

impl GenerateTokenResponse {
    pub fn new(token: String) -> Self {
        Self { token }
    }
}

impl From<RoomToken> for GenerateTokenResponse {
    fn from(value: RoomToken) -> Self {
        Self {
            token: value.as_str().to_owned(),
        }
    }
}
