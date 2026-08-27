use identity_domain::verification::{channel::VerificationChannel, purpose::VerificationPurpose};

use crate::dto::RequestContext;

#[derive(Debug)]
pub struct IssueVerificationCodeRequest {
    pub purpose: VerificationPurpose,
    pub channel: VerificationChannel,
    pub ctx: RequestContext,
}
