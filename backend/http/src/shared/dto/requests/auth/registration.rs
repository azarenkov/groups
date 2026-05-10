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

impl From<RegisterUserRequestBody>
    for application::dto::requests::auth::register_user::RegisterUserRequest
{
    fn from(value: RegisterUserRequestBody) -> Self {
        Self {
            name: value.name,
            email: value.email,
            password: value.password,
        }
    }
}
