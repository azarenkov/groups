use identity::dto::{
    RequestContext,
    requests::{auth::register_user::RegisterUserRequest, client_context::ClientContext},
};
use identity_domain::user::locale::Locale;
use serde::Deserialize;
use validator::Validate;

use crate::dto::requests::auth::username::validate_username;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RegisterUserRequestBody {
    #[validate(length(min = 3, max = 15), custom(function = "validate_username"))]
    pub name: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 30))]
    pub password: String,
    #[serde(default)]
    pub locale: Option<String>,
    #[serde(default)]
    #[validate(length(max = 2048))]
    pub altcha: Option<String>,
}

impl RegisterUserRequestBody {
    pub fn into_request(self, client: ClientContext, ctx: RequestContext) -> RegisterUserRequest {
        let locale = self
            .locale
            .as_deref()
            .map(Locale::from_str_lenient)
            .unwrap_or_default();
        RegisterUserRequest {
            name: self.name,
            email: self.email,
            password: self.password,
            locale,
            client,
            ctx,
        }
    }
}
