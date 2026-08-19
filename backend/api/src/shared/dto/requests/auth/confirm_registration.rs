use application::dto::{
    RequestContext, requests::auth::confirm_registration::ConfirmRegistrationRequest,
};
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmRegistrationRequestBody {
    #[validate(length(equal = 6))]
    pub verification_code: String,
}

impl ConfirmRegistrationRequestBody {
    pub fn into_request(self, ctx: RequestContext) -> ConfirmRegistrationRequest {
        ConfirmRegistrationRequest {
            verification_code: self.verification_code,
            ctx,
        }
    }
}
