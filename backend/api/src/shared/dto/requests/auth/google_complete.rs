use application::dto::{
    RequestContext,
    requests::{auth::google_complete::GoogleCompleteRequest, client_context::ClientContext},
};
use domain::user::locale::Locale;
use serde::Deserialize;
use validator::Validate;

use crate::shared::dto::requests::auth::username::validate_username;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct GoogleCompleteRequestBody {
    #[validate(length(min = 1))]
    pub id_token: String,
    #[validate(length(min = 3, max = 15), custom(function = "validate_username"))]
    pub username: String,
    #[serde(default)]
    pub locale: Option<String>,
}

impl GoogleCompleteRequestBody {
    pub fn into_request(self, client: ClientContext, ctx: RequestContext) -> GoogleCompleteRequest {
        let locale = self
            .locale
            .as_deref()
            .map(Locale::from_str_lenient)
            .unwrap_or_default();
        GoogleCompleteRequest {
            id_token: self.id_token,
            username: self.username.trim().to_string(),
            locale,
            client,
            ctx,
        }
    }
}
