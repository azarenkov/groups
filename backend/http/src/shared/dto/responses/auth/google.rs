use serde::Serialize;

use crate::shared::dto::responses::auth::token_pair::TokenPairResponse;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleAuthResponse {
    pub needs_username: bool,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub tokens: Option<TokenPairResponse>,
}

impl GoogleAuthResponse {
    pub fn authenticated(tokens: TokenPairResponse) -> Self {
        Self {
            needs_username: false,
            tokens: Some(tokens),
        }
    }

    pub fn needs_username() -> Self {
        Self {
            needs_username: true,
            tokens: None,
        }
    }
}
