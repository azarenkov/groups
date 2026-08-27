use identity::dto::{
    RequestContext,
    requests::{auth::google_login::GoogleLoginRequest, client_context::ClientContext},
};
use identity_domain::user::locale::Locale;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct GoogleLoginRequestBody {
    #[validate(length(min = 1))]
    pub id_token: String,
    #[serde(default)]
    pub locale: Option<String>,
}

impl GoogleLoginRequestBody {
    pub fn into_request(self, client: ClientContext, ctx: RequestContext) -> GoogleLoginRequest {
        let locale = self
            .locale
            .as_deref()
            .map(Locale::from_str_lenient)
            .unwrap_or_default();
        GoogleLoginRequest {
            id_token: self.id_token,
            locale,
            client,
            ctx,
        }
    }
}
