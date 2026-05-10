use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LoginUserRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 4, max = 30))]
    pub password: String,
}

impl From<LoginUserRequest> for application::dto::requests::auth::login_user::LoginUserRequest {
    fn from(value: LoginUserRequest) -> Self {
        Self {
            email: value.email,
            password: value.password,
        }
    }
}
