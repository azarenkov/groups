use crate::dto::RequestContext;

#[derive(Debug)]
pub struct ConfirmRegistrationRequest {
    pub verification_code: String,
    pub ctx: RequestContext,
}
