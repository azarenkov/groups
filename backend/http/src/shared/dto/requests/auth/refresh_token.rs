use application::dto::requests::auth::refresh_tokens::RefreshTokensRequest;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RefreshTokenRequestBody {
    #[validate(length(min = 1))]
    pub refresh_token: String,
}

impl From<RefreshTokenRequestBody> for RefreshTokensRequest {
    fn from(value: RefreshTokenRequestBody) -> Self {
        Self {
            refresh_token: value.refresh_token,
        }
    }
}
