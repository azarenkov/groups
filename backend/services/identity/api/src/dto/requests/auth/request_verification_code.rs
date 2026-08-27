use identity_domain::verification::channel::VerificationChannel;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RequestVerificationCodeRequestBody {
    pub channel: VerificationChannelRequest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerificationChannelRequest {
    Email,
    Phone,
}

impl From<VerificationChannelRequest> for VerificationChannel {
    fn from(value: VerificationChannelRequest) -> Self {
        match value {
            VerificationChannelRequest::Email => VerificationChannel::Email,
            VerificationChannelRequest::Phone => VerificationChannel::Phone,
        }
    }
}
