use application::dto::{
    RequestContext,
    requests::{auth::register_user::RegisterUserRequest, client_context::ClientContext},
};
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RegisterUserRequestBody {
    #[validate(length(min = 3, max = 15))]
    pub name: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 4, max = 30))]
    pub password: String,
}

impl RegisterUserRequestBody {
    pub fn into_request(self, client: ClientContext, ctx: RequestContext) -> RegisterUserRequest {
        RegisterUserRequest {
            name: self.name,
            email: self.email,
            password: self.password,
            client,
            ctx,
        }
    }
}
