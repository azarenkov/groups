use application::dto::requests::auth::logout::LogoutRequest;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LogoutRequestBody {
    #[validate(length(min = 1))]
    pub refresh_token: String,
}

impl From<LogoutRequestBody> for LogoutRequest {
    fn from(value: LogoutRequestBody) -> Self {
        Self {
            refresh_token: value.refresh_token,
        }
    }
}
