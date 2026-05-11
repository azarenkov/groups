use application::dto::requests::auth::confirm_registration::ConfirmRegistrationRequest;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmRegistrationRequestBody {
    #[validate(length(equal = 6))]
    pub verification_code: String,
}

impl From<ConfirmRegistrationRequestBody> for ConfirmRegistrationRequest {
    fn from(value: ConfirmRegistrationRequestBody) -> Self {
        Self {
            verification_code: value.verification_code,
        }
    }
}
