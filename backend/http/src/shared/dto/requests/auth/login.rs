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

impl From<LoginUserRequestBody> for application::dto::requests::auth::login_user::LoginUserRequest {
    fn from(value: LoginUserRequestBody) -> Self {
        Self {
            email: value.email,
            password: value.password,
        }
    }
}
