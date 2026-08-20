use identity::dto::requests::auth::refresh_tokens::RefreshTokensRequest;
use shared::client_context::ClientContext;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RefreshTokenRequestBody {
    #[validate(length(min = 1))]
    pub refresh_token: String,
}

impl RefreshTokenRequestBody {
    pub fn into_request(self, client: ClientContext) -> RefreshTokensRequest {
        RefreshTokensRequest {
            refresh_token: self.refresh_token,
            client,
        }
    }
}
