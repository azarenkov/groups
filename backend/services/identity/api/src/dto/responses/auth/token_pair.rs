use identity::{
    ports::token_issuer::IssuedTokens,
    usecases::auth::login::LoginOutcome,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenPairResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub requires_verification: bool,
}

impl From<IssuedTokens> for TokenPairResponse {
    fn from(value: IssuedTokens) -> Self {
        Self {
            access_token: value.access_token,
            refresh_token: value.refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: value.access_token_expires_in,
            requires_verification: false,
        }
    }
}

impl From<LoginOutcome> for TokenPairResponse {
    fn from(value: LoginOutcome) -> Self {
        let mut response: TokenPairResponse = value.tokens.into();
        response.requires_verification = value.requires_verification;
        response
    }
}
