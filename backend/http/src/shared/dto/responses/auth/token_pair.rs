use application::ports::token_issuer::IssuedTokens;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenPairResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

impl From<IssuedTokens> for TokenPairResponse {
    fn from(value: IssuedTokens) -> Self {
        Self {
            access_token: value.access_token,
            refresh_token: value.refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: value.access_token_expires_in,
        }
    }
}
