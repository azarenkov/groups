use application::dto::requests::{auth::logout::LogoutRequest, client_context::ClientContext};
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LogoutRequestBody {
    #[validate(length(min = 1))]
    pub refresh_token: String,
}

impl LogoutRequestBody {
    pub fn into_request(self, client: ClientContext) -> LogoutRequest {
        LogoutRequest {
            refresh_token: self.refresh_token,
            client,
        }
    }
}
