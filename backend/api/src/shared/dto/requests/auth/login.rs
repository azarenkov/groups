use application::dto::{
    RequestContext,
    requests::{auth::login_user::LoginUserRequest, client_context::ClientContext},
};
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LoginUserRequestBody {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 4, max = 30))]
    pub password: String,
}

impl LoginUserRequestBody {
    pub fn into_request(self, client: ClientContext, ctx: RequestContext) -> LoginUserRequest {
        LoginUserRequest {
            email: self.email,
            password: self.password,
            client,
            ctx,
        }
    }
}
