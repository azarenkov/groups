use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ForgotPasswordRequestBody {
    #[validate(email)]
    pub email: String,
    #[serde(default)]
    #[validate(length(max = 2048))]
    pub altcha: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordRequestBody {
    #[validate(length(min = 1, max = 200))]
    pub token: String,
    #[validate(length(min = 8, max = 30))]
    pub new_password: String,
}
